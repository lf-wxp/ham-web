import type { Page } from "@playwright/test";
import { bankQuestions, expect, option, settleReveal, test } from "./fixtures";

// 「点了要等一会」的回归守卫。判据都是**结构性**的（某件重活是否被挪到了首帧之后、某个请求是否只发一次），
// 不断言毫秒数：dev 构建比 release 慢一个量级，CI 的机器也不固定，写死阈值只会制造抖动。
// 背景与实测见 `docs/ui/interaction.md` §2 与 `docs/ui/motion.md`「长列表优先分页」。

async function seedModern(page: Page) {
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "modern");
    localStorage.setItem("theme", "dark");
  });
}

/** 站内导航（路由器拦截的是真实的链接点击，这里放一个临时链接点一下）。 */
async function clientNav(page: Page, href: string) {
  await page.evaluate((h) => {
    const a = document.createElement("a");
    a.href = h;
    document.body.append(a);
    a.click();
    a.remove();
  }, href);
}

test.describe("响应：重活不挡首帧", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("现代风换页：首屏内容不会先显示再变暗，折叠线以下的块才浮现", async ({ page }) => {
    await seedModern(page);
    // 从首页出发：`/` → `/glossary` 是「前进」，待浮现的块应带 12px 位移。
    await page.goto("/");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await settleReveal(page);

    // 换页后逐帧采样视口内内容块的不透明度：一旦超过 0.5 之后又往下掉，就是「出现 → 消失 → 再浮上来」。
    const { dip, frames } = await page.evaluate(async () => {
      const frame = () => new Promise<void>((r) => requestAnimationFrame(() => r()));
      const a = document.createElement("a");
      a.href = "/glossary";
      document.body.append(a);
      a.click();
      a.remove();
      let peak = 0;
      let dip = 0;
      let frames = 0;
      // 采到足够多的帧才收工：只采一两帧的话，「出现 → 消失 → 再来」根本来不及被看到，
      // 判据会退化成「什么都没发生」而恒绿。150 帧给慢机器上的导航留余量。
      for (let i = 0; i < 150 && frames < 30; i++) {
        await frame();
        if (location.pathname !== "/glossary") continue;
        const vh = innerHeight;
        const rc = document.getElementById("route-content");
        const rcOp = rc ? Number(getComputedStyle(rc).opacity) : 1;
        const blocks = [...document.querySelectorAll("#route-content .md-page > *, #route-content .md-term")].filter((el) => {
          const r = el.getBoundingClientRect();
          return r.height > 8 && r.top < vh * 0.85 && r.bottom > 60;
        });
        if (!blocks.length) continue;
        frames++;
        const min = Math.min(...blocks.map((el) => Number(getComputedStyle(el).opacity))) * rcOp;
        peak = Math.max(peak, min);
        if (peak > 0.5) dip = Math.max(dip, peak - min);
      }
      return { dip, frames };
    });
    expect(
      frames,
      "换页后采不到 30 帧内容块：要么选择器失效，要么采样窗口太短 —— 两种情况判据都等于没跑",
    ).toBeGreaterThanOrEqual(30);
    expect(dip, "首屏内容出现之后又被藏起来重播了一次").toBeLessThan(0.05);

    // 折叠线以下确实有块在等浮现，而且用的是现代风自己的平滑规则（曾经选择器写错，一直在用像素风的 4 格跳变）。
    // 初始位移随换页方向变（`interaction.css` §1）：前进 12px、后退 -10px、平级不位移 —— 像素风是 8px。
    await settleReveal(page);
    const styles = await page.evaluate(() => {
      const pending = document.querySelector<HTMLElement>('#route-content [data-reveal=""]');
      const shown = document.querySelector<HTMLElement>('#route-content [data-reveal="in"]');
      const read = (el: HTMLElement | null) =>
        el && {
          transform: getComputedStyle(el).transform,
          duration: getComputedStyle(el).transitionDuration,
          timing: getComputedStyle(el).transitionTimingFunction,
        };
      return {
        dir: document.getElementById("route-content")?.getAttribute("data-nav-dir") ?? "",
        pending: read(pending),
        shown: read(shown),
      };
    });
    const expected: Record<string, string> = {
      forward: "matrix(1, 0, 0, 1, 0, 12)",
      back: "matrix(1, 0, 0, 1, 0, -10)",
      lateral: "none",
    };
    expect(Object.keys(expected), `未知的换页方向 ${styles.dir}`).toContain(styles.dir);
    expect(styles.pending, "术语表折叠线以下应有等待浮现的块").not.toBeNull();
    expect(styles.shown, "术语表首屏应有已入场的块").not.toBeNull();
    expect(styles.pending?.transform, `待浮现的块（${styles.dir}）位移不对`).toBe(expected[styles.dir]);
    expect(styles.pending?.duration, "藏起来的那一下不该播过渡").toBe("0s");
    expect(styles.shown?.timing, "浮现应是平滑曲线，不是逐格跳变").not.toContain("steps");
  });

  test("「更多」目录：打开的第一帧只挂第一块入口，其余随后补齐", async ({ page }) => {
    await seedModern(page);
    await page.goto("/countdown");
    await expect(page.locator("button.md-dock-item")).toBeVisible();
    const counts = await page.evaluate(async () => {
      const frame = () => new Promise<void>((r) => requestAnimationFrame(() => r()));
      document.querySelector<HTMLButtonElement>("button.md-dock-item")!.click();
      const out: number[] = [];
      for (let i = 0; i < 60; i++) {
        await frame();
        out.push(document.querySelectorAll(".md-menu-item").length);
      }
      return out;
    });
    const first = counts.find((n) => n > 0) ?? 0;
    expect(first, "面板出现时一个入口都没有").toBeGreaterThan(0);
    expect(first, "第一帧就挂了整份目录（分块失效）").toBeLessThanOrEqual(24);
    await expect.poll(() => page.locator(".md-menu-item").count()).toBeGreaterThan(100);
  });

  test("术语表：题库已在内存时，出现次数也在首帧之后才统计", async ({ page }) => {
    await seedModern(page);
    // 先进题库浏览：三个题库都进了内存。没有这一步，首帧「统计中」只说明题库还在路上，证明不了统计被推迟了。
    await page.goto("/browse");
    await expect(page.locator(".md-q-row").first()).toBeVisible();
    const pendingAtFirstCards = await page.evaluate(async () => {
      const frame = () => new Promise<void>((r) => requestAnimationFrame(() => r()));
      const a = document.createElement("a");
      a.href = "/glossary";
      document.body.append(a);
      a.click();
      a.remove();
      for (let i = 0; i < 120; i++) {
        await frame();
        if (document.querySelector(".md-term")) break;
      }
      return [...document.querySelectorAll(".md-term")].filter((el) =>
        el.textContent?.includes("统计题库出现次数…"),
      ).length;
    });
    expect(pendingAtFirstCards, "卡片出来的那一帧，出现次数已经全部算完了（统计挤进了首帧）").toBeGreaterThan(0);
    // 最终全部统计完：有胶囊、没有残留的「统计中」。
    await expect(page.locator('a[href^="/browse?bank="]').first()).toBeVisible();
    await expect(page.getByText("统计题库出现次数…")).toHaveCount(0);
  });
});

test.describe("响应：题库预热与去重", () => {
  test("空闲时预热 A 类题库，之后进练习页不再请求", async ({ page }) => {
    await seedModern(page);
    const requests: string[] = [];
    page.on("request", (r) => {
      if (/\/questions\/[ABC]\.json/.test(r.url())) requests.push(new URL(r.url()).pathname);
    });
    // 工具页本身不需要题库：这里出现的 A.json 只能来自启动 3 秒后的空闲预热（`bank_updates::prewarm`）。
    await page.goto("/tools");
    await expect.poll(() => requests.includes("/questions/A.json"), { timeout: 15_000 }).toBe(true);
    expect(requests, "没用过的 B / C 类不该预热").toEqual(["/questions/A.json"]);
    await clientNav(page, "/practice");
    await expect(page.locator("main .md-opt").first()).toBeVisible();
    expect(requests, "预热过的题库进练习页时又请求了一次").toEqual(["/questions/A.json"]);
  });

  test("同一份题库同时只请求一次（题库浏览还没拿到，就进了术语表）", async ({ page }) => {
    await seedModern(page);
    const requests: string[] = [];
    page.on("request", (r) => {
      if (/\/questions\/B\.json/.test(r.url())) requests.push(r.url());
    });
    // 用 B 类：题库浏览按 A → B → C 依次加载，扣住 B 的响应 3 秒，就能在「B 已请求、还没到」时进术语表，
    // 两处对 B 的请求一定重叠（扣 A 的话，等到页面能操作时 A 早就到了）。
    // 响应标成 `no-store`：否则浏览器的 HTTP 缓存会把两个同址请求在网络层合并成一个，
    // 用例看到「只请求一次」，应用里却照样解析了两遍 —— 那样这条用例就没有牙齿。
    await page.route(/\/questions\/B\.json/, async (route) => {
      await new Promise((r) => setTimeout(r, 3000));
      const response = await route.fetch();
      await route.fulfill({ response, headers: { ...response.headers(), "cache-control": "no-store" } });
    });
    await page.goto("/browse");
    await expect.poll(() => requests.length, { timeout: 15_000 }).toBe(1);
    await clientNav(page, "/glossary");
    await expect(page.locator('a[href^="/browse?bank="]').first()).toBeVisible({ timeout: 20_000 });
    expect(requests, "B 类在下载途中被第二个页面又请求了一次").toHaveLength(1);
  });
});

/** 一本日志：`n` 条通联，全部寄出纸卡、都还没收到（= 全部待回卡）。 */
function pendingLog(n: number): string {
  const entries = Array.from({ length: n }, (_, i) => ({
    id: i + 1,
    date: `2025-01-${String((i % 28) + 1).padStart(2, "0")}`,
    time: "12:00",
    freq: "14.074",
    band: "20m",
    mode: "FT8",
    callsign: `BG${i % 10}T${String.fromCharCode(65 + (i % 26))}${String.fromCharCode(65 + ((i * 7) % 26))}`,
    gridsquare: "OM89",
    qsl_sent: true,
    // `remark` 没有 serde 默认值：缺了它整本日志反序列化失败，页面拿到的是空日志。
    remark: "",
  }));
  return JSON.stringify({ entries });
}

type Counts = { setBook: number; getLog: number };

test.describe("响应：用得越久越慢的那一类", () => {
  test("作答：答对不在错题本里的题不整本写回错题本，也不再去读整本日志", async ({ page }) => {
    // 计数 `localStorage` 的读写：作答链路此前每答一题都整本写回错题本（答对且不在本里时内容根本没变），
    // 还要从存储里解析整本通联日志做成就判定。现在前者跳过、后者读日志 store 上的摘要 Memo。
    await page.addInitScript(() => {
      if (window !== window.top) return;
      const counts = { setBook: 0, getLog: 0 };
      (window as unknown as { __counts: Counts }).__counts = counts;
      const set = Storage.prototype.setItem;
      const get = Storage.prototype.getItem;
      Storage.prototype.setItem = function (k: string, v: string) {
        if (this === localStorage && k === "mistake-book") counts.setBook++;
        return set.call(this, k, v);
      };
      Storage.prototype.getItem = function (k: string) {
        if (this === localStorage && k === "logbook") counts.getLog++;
        return get.call(this, k);
      };
    });
    await page.addInitScript((log) => {
      if (window !== window.top) return;
      if (!localStorage.getItem("logbook")) localStorage.setItem("logbook", log);
    }, pendingLog(30));
    await page.addInitScript(() => {
      localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
    });
    await page.goto("/practice");
    await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
    // 启动阶段的读取（日志快照、空闲时算的摘要）不算：等它们都过去再清零。
    await page.waitForTimeout(1500);
    // 按题库文件作答，不点「显示正确答案」：看过答案的题不计入作答记录，那样什么都不会写。
    const stemText = (await page.locator("main [data-slot=card-content] .whitespace-pre-line").first().innerText()).trim();
    const q = bankQuestions("A").find((x) => x.question.trim() === stemText);
    expect(q, `题库里找不到这道题：${stemText}`).toBeTruthy();
    await page.evaluate(() => {
      const c = (window as unknown as { __counts: Counts }).__counts;
      c.setBook = 0;
      c.getLog = 0;
    });
    for (const letter of q!.answer_keys) await option(page, letter).click();
    const check = page.getByRole("button", { name: "检查答案" });
    if (await check.count()) await check.first().click();
    // 作答在翻到下一题时才记进错题本 / 统计。
    await page.getByRole("button", { name: "下一题" }).first().click();
    await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();
    await page.waitForTimeout(500);
    const counts = await page.evaluate(() => (window as unknown as { __counts: Counts }).__counts);
    expect(counts.setBook, "答对一道不在错题本里的题，错题本内容没变，不该整本写回").toBe(0);
    expect(counts.getLog, "作答时又从存储里读了整本日志").toBe(0);
  });

  test("日志页：首屏以下的统计明细在首帧之后挂载，待回卡先列 20 条", async ({ page }) => {
    await seedModern(page);
    await page.addInitScript((log) => {
      if (window !== window.top) return;
      if (!localStorage.getItem("logbook")) localStorage.setItem("logbook", log);
    }, pendingLog(60));
    await page.goto("/countdown");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    // 「站内跳进来 → 第一帧看明细在不在」放在同一次 evaluate 里：跨一次 Playwright 往返就看不到首帧了。
    const atFirstFrame = await page.evaluate(async () => {
      const frame = () => new Promise((r) => requestAnimationFrame(() => r(null)));
      const a = document.createElement("a");
      a.href = "/log";
      document.body.append(a);
      a.click();
      a.remove();
      // 在「页头第一次出现」的那个帧回调里看：它之后紧跟着的就是新页面的第一帧绘制。
      // 不要先等路径变化再多等一帧 —— 路由是异步落位的，多等的那一帧里延后挂载早已完成。
      for (let i = 0; i < 120; i++) {
        await frame();
        const text = document.getElementById("route-content")?.textContent ?? "";
        if (text.includes("通联日志")) return { header: true, pending: text.includes("待确认 QSL") };
      }
      return { header: false, pending: false };
    });
    expect(atFirstFrame.header, "日志页的首帧应已有页头").toBe(true);
    expect(atFirstFrame.pending, "待回卡在首屏以下，首帧不该同步建好").toBe(false);

    // 外层还套着一层 `section`：按文档顺序取最后一个，即离标题最近的那一节。
    const pending = page
      .locator("section", { has: page.getByRole("heading", { name: /待确认 QSL/ }) })
      .last();
    await expect(pending).toBeVisible();
    const rows = pending.locator(".divide-y > div");
    await expect(rows).toHaveCount(20);
    await pending.getByRole("button", { name: "显示全部 60 条" }).click();
    await expect(rows).toHaveCount(60);
  });
});

test.describe("响应：首屏看不到的不先建", () => {
  test("分页签：只挂当前面板，切到别的面板才挂、切回来状态还在", async ({ page }) => {
    await page.goto("/morse");
    const panels = page.locator('main [role="tabpanel"]');
    await expect(panels).toHaveCount(4);
    const filled = () => panels.evaluateAll((els) => els.map((el) => el.childElementCount > 0));
    expect(await filled(), "进页面时只有当前面板有内容").toEqual([true, false, false, false]);
    await page.getByRole("tab", { name: "解码练习" }).click();
    await expect.poll(filled).toEqual([true, false, true, false]);
    await page.getByRole("tab", { name: "Koch 训练" }).click();
    // 切回来：两块都还在（已挂载的面板只切显隐，不卸载）。
    expect(await filled()).toEqual([true, false, true, false]);
    // 面板外壳始终在：`aria-controls` 指向的节点必须真实存在。
    const controls = await page.getByRole("tab").evaluateAll((els) => els.map((el) => el.getAttribute("aria-controls")));
    for (const id of controls) expect(await page.locator(`[id="${id}"]`).count()).toBe(1);
  });

  test("/bands：桌面表格与手机卡片只建当前宽度那一套，跨断点时切换", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto("/bands");
    const table = page.locator("main table");
    const cards = page.locator('main [class~="md:hidden"]');
    await expect(table).toHaveCount(1);
    await expect(cards).toHaveCount(0);
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(table).toHaveCount(0);
    await expect(cards).toHaveCount(1);
  });

  test("世界地图：首帧只建中间那份，左右两份随后补上", async ({ page }) => {
    await seedModern(page);
    await page.goto("/countdown");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    const TILES =
      'svg g[transform="translate(-720 0)"], svg g[transform="translate(0 0)"], svg g[transform="translate(720 0)"]';
    const atFirstFrame = await page.evaluate(async (tiles) => {
      const frame = () => new Promise((r) => requestAnimationFrame(() => r(null)));
      const a = document.createElement("a");
      a.href = "/zone-map";
      document.body.append(a);
      a.click();
      a.remove();
      for (let i = 0; i < 120 && !document.querySelector(tiles); i++) await frame();
      return document.querySelectorAll(tiles).length;
    }, TILES);
    expect(atFirstFrame, "第一帧就建了三份世界").toBe(1);
    await expect(page.locator(TILES)).toHaveCount(3);
  });

  test("首页：首屏滚出视口后暂停循环动画，滚回来继续", async ({ page }) => {
    await seedModern(page);
    await page.goto("/");
    const band = page.locator(".md-hero-band");
    await expect(band).toBeVisible();
    await expect(band).not.toHaveAttribute("data-offscreen");
    await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
    await expect(band).toHaveAttribute("data-offscreen", "");
    const state = await page.locator(".md-aurora").evaluate((el) => getComputedStyle(el).animationPlayState);
    expect(state, "首屏滚走之后极光动画还在跑").toBe("paused");
    await page.evaluate(() => window.scrollTo(0, 0));
    await expect(band).not.toHaveAttribute("data-offscreen");
  });

  test("像素风首页：题库选择器不为点亮按钮下载 B / C 类题库", async ({ page }) => {
    const requests: string[] = [];
    page.on("request", (r) => {
      if (/\/questions\/[ABC]\.json/.test(r.url())) requests.push(r.url());
    });
    await page.goto("/");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    // 启动 3 秒后的空闲预热只拉 A 类（见上面那条预热用例）；在那之前看选择器有没有自己去拉。
    await page.waitForTimeout(2500);
    test.skip(
      requests.length > 0 && !requests.some((u) => u.includes("?v=")),
      "开发构建的题库地址没有修订号，选择器仍要逐个加载判断可用性",
    );
    const bc = requests.filter((u) => /\/questions\/[BC]\.json/.test(u));
    expect(bc, "选择器为判断可用性下载了 B / C 类").toEqual([]);
  });
});

test.describe("响应：输入", () => {
  // 两套皮肤各一遍：像素风走 `ui::Input`，现代风的搜索框是另一个控件 `ModernInput`，
  // 组字门控要两边都有（只测像素风时，现代风漏掉也看不出来）。
  for (const style of ["pixel", "modern"] as const) {
    test(`输入法组字期间不重算搜索，选字结束才算一次（${style}）`, async ({ page }) => {
      if (style === "modern") await seedModern(page);
      await page.goto("/glossary");
      const input = page.locator('main input[type="search"]').first();
      await expect(input).toBeVisible();
      const terms = page.locator("main article");
      await expect(terms.first()).toBeVisible();
      const before = await terms.count();
      // 模拟拼音输入「tian」还没选字：浏览器派发的是 `isComposing: true` 的 input 事件。
      await input.evaluate((el: HTMLInputElement) => {
        el.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true }));
        el.value = "tian";
        el.dispatchEvent(new InputEvent("input", { bubbles: true, isComposing: true, data: "tian" }));
      });
      await page.waitForTimeout(500);
      expect(await terms.count(), "组字中的拼音被当成关键词搜了").toBe(before);
      // 选字：组字结束，这时才按「天线」搜一次。
      await input.evaluate((el: HTMLInputElement) => {
        el.value = "天线";
        el.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: "天线" }));
      });
      await expect.poll(() => terms.count()).toBeLessThan(before);
      expect(await terms.count(), "选字结束后应按「天线」筛出至少一条").toBeGreaterThan(0);
    });

    // 「尾随的重复 input 只派发一次」由 core 的 `ime_gate` 单元测试逐条钉住（页面上数不出派发次数）；
    // 这里守的是端到端的结果：Safari 的事件顺序下搜索结果正确，而且门控不会卡在「组字中」，之后照常输入。
    test(`Safari 的事件顺序：compositionend 后紧跟同值 input，结果正确且之后继续输入照常生效（${style}）`, async ({ page }) => {
      if (style === "modern") await seedModern(page);
      await page.goto("/glossary");
      const input = page.locator('main input[type="search"]').first();
      await expect(input).toBeVisible();
      const terms = page.locator("main article");
      await expect(terms.first()).toBeVisible();
      const before = await terms.count();
      await input.evaluate((el: HTMLInputElement) => {
        el.value = "天线";
        el.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: "天线" }));
        // 紧跟其后的 input：`isComposing` 已为 false、值相同（Safari 的顺序）。
        el.dispatchEvent(new InputEvent("input", { bubbles: true, isComposing: false, data: "天线" }));
      });
      await expect.poll(() => terms.count()).toBeLessThan(before);
      const filtered = await terms.count();
      expect(filtered, "按「天线」应筛出至少一条").toBeGreaterThan(0);
      // 门控没有卡住：之后一次普通的 input（非组字、值不同）必须生效。
      await input.evaluate((el: HTMLInputElement) => {
        el.value = "天线zzzzqq";
        el.dispatchEvent(new InputEvent("input", { bubbles: true, isComposing: false, data: "q" }));
      });
      await expect.poll(() => terms.count()).toBe(0);
    });
  }

  test("非搜索的输入框不受组字门控：组字期间也逐字回调（字段依赖逐字回调才能即时联动）", async ({ page }) => {
    await page.goto("/callsign");
    // 呼号查询框的 on_change 会把值转大写再写回：回调一旦被门控吞掉，DOM 里的小写就不会变大写。
    const input = page.getByLabel("呼号", { exact: true });
    await expect(input).toBeVisible();
    await input.evaluate((el: HTMLInputElement) => {
      el.value = "bg4xyz";
      el.dispatchEvent(new InputEvent("input", { bubbles: true, isComposing: true, data: "bg4xyz" }));
    });
    await expect(input).toHaveValue("BG4XYZ");
  });
});

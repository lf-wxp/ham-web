import type { Page } from "@playwright/test";
import {
  bankQuestions,
  expect,
  mistakeRecord,
  option,
  seedLog,
  settleReveal,
  test,
} from "./fixtures";

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

/**
 * 站内跳到 `href`，返回新页面**第一次被画出来**那一刻 `selector`（相对 `#route-content`）命中的节点数。
 *
 * 用 `MutationObserver` 记第一次出现，不用逐帧轮询：机器繁忙（并发跑全量）时两帧间隔会被拖长，
 * 轮询一次就可能越过「首帧只有前几张」的中间态，用例间歇红。观察器每次 DOM 变动都回调，不会漏。
 * 「点链接 → 等第一次出现」必须在同一次 `evaluate` 里：跨一次 Playwright 往返就看不到首帧了。
 *
 * - 默认：命中数第一次大于 0 时返回；
 * - `readyText`：改为等路由容器里出现这段文字（如页头），返回那一刻的命中数 —— 可以是 0，
 *   用来断言「页头已在、下面的块还没挂」；
 * - `mark`：在第一个命中的节点上写 `data-mark="1"`，之后断言「补齐只追加、已建好的不重建」。
 * 15 秒内没等到返回 -1。
 */
async function firstFrame(
  page: Page,
  href: string,
  selector: string,
  opts: { readyText?: string; mark?: boolean } = {},
): Promise<number> {
  return page.evaluate(
    ({ href, selector, readyText, mark }) => {
      const root = document.getElementById("route-content") as HTMLElement;
      return new Promise<number>((resolve) => {
        let timer: ReturnType<typeof setTimeout> | undefined;
        const obs = new MutationObserver(() => {
          if (readyText && !(root.textContent ?? "").includes(readyText)) return;
          const nodes = root.querySelectorAll(selector);
          if (!readyText && nodes.length === 0) return;
          if (mark && nodes[0]) nodes[0].setAttribute("data-mark", "1");
          obs.disconnect();
          clearTimeout(timer);
          resolve(nodes.length);
        });
        obs.observe(root, { childList: true, subtree: true, characterData: true });
        timer = setTimeout(() => {
          obs.disconnect();
          resolve(-1);
        }, 15_000);
        const a = document.createElement("a");
        a.href = href;
        document.body.append(a);
        a.click();
        a.remove();
      });
    },
    { href, selector, readyText: opts.readyText ?? null, mark: opts.mark ?? false },
  );
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

test.describe("响应：首帧只建一部分，其余分批补齐", () => {
  test("错题本：首帧只建前几张卡，之后分批补到一页（40 张）", async ({ page }) => {
    await seedModern(page);
    // 60 条错题，超过一页；题目取自题库，快照与题库一致。
    const records = bankQuestions("A").slice(0, 60).map((q) => mistakeRecord(q));
    await page.addInitScript((book) => {
      if (window !== window.top) return;
      if (!localStorage.getItem("mistake-book")) localStorage.setItem("mistake-book", book);
    }, JSON.stringify({ records }));
    await page.goto("/countdown");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    const atFirst = await firstFrame(page, "/mistakes", ".md-q-row");
    expect(atFirst, "进页面后一直没有出现错题卡").toBeGreaterThan(0);
    expect(atFirst, "首帧不该一次把一页 40 张卡都建出来").toBeLessThan(12);
    // 补齐之后正好一页，不多不少；「加载更多」仍可用。
    await expect(page.locator("#route-content .md-q-row")).toHaveCount(40);
    await page.getByRole("button", { name: /加载更多/ }).click();
    await expect(page.locator("#route-content .md-q-row")).toHaveCount(60);
  });

  test("术语表：首帧只建前几张卡，之后分批补到一页（60 张）", async ({ page }) => {
    await seedModern(page);
    await page.goto("/countdown");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    const atFirst = await firstFrame(page, "/glossary", "article");
    expect(atFirst, "进页面后一直没有出现术语卡").toBeGreaterThan(0);
    expect(atFirst, "首帧不该一次把一页 60 张卡都建出来").toBeLessThan(30);
    await expect(page.locator("#route-content article")).toHaveCount(60);
    // 补齐过程中已建好的卡不重建：在第一张卡上做个记号，补齐之后记号还在。
    // （此前整体 `collect_view`，每补一批都会把全部卡片重建一遍。）
    await page.evaluate(() => document.querySelector("#route-content article")?.setAttribute("data-mark", "1"));
    await page.getByRole("button", { name: /加载更多/ }).click();
    await expect(page.locator("#route-content article")).toHaveCount(120);
    await expect(page.locator('#route-content article[data-mark="1"]')).toHaveCount(1);
  });

  test("题库浏览：首帧只建前几行，之后分批补到一页；加载更多、换关键词后行跟着对", async ({ page }) => {
    await seedModern(page);
    await page.goto("/countdown");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    // 站内跳进来（题库已在内存里）；题库还在路上的整页打开见下一条。同时在第一行上做记号。
    const atFirst = await firstFrame(page, "/browse?bank=A", ".md-q-row", { mark: true });
    expect(atFirst, "进页面后一直没有出现题目行").toBeGreaterThan(0);
    expect(atFirst, "第一批不该一次把一页 40 道题都建出来").toBeLessThan(40);
    const rows = page.locator("#route-content .md-q-row");
    await expect(rows).toHaveCount(40);
    // 补齐只追加：已建好的第一行还是原来那个节点。
    await expect(page.locator('#route-content .md-q-row[data-mark="1"]')).toHaveCount(1);
    const firstText = await rows.first().innerText();
    await page.getByRole("button", { name: /加载更多/ }).click();
    await expect(rows).toHaveCount(80);
    await expect(page.locator('#route-content .md-q-row[data-mark="1"]')).toHaveCount(1);
    // 换关键词：第一行要换成新的题（只断言内容：节点是原地改还是换新的，是实现细节）。
    await page.locator('main input[type="search"]').first().fill("天线");
    await expect
      .poll(async () => (await rows.count()) > 0 && (await rows.first().innerText()) !== firstText)
      .toBe(true);
  });

  test("日志页：记录列表在首帧之后才挂，挂上后是一页 50 行", async ({ page }) => {
    await seedModern(page);
    await page.addInitScript((log) => {
      if (window !== window.top) return;
      if (!localStorage.getItem("logbook")) localStorage.setItem("logbook", log);
    }, pendingLog(80));
    await page.goto("/countdown");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    // 页头出现的那一刻数一数记录行：-1 = 页头一直没出现。
    const rowsAtHeader = await firstFrame(page, "/log", "tbody tr", { readyText: "通联日志" });
    expect(rowsAtHeader, "日志页的首帧应已有页头").not.toBe(-1);
    expect(rowsAtHeader, "记录列表在表单之下，首帧不该同步建好").toBe(0);
    await expect(page.locator("#route-content tbody tr")).toHaveCount(50);
  });
});

test.describe("响应：补齐循环必须能结束", () => {
  // `grow_cap` 的步进是个有限步内必然结束的循环；它曾经在「现在没东西可显示」时原地等，
  // 靠外部状态（数据到达）才能结束：加载失败时永不收工，页面隐藏（帧回调同步执行）时变成无终止的同步递归。

  /** 让 `document.hidden` 恒为 true：后台标签页里 rAF 暂停，应用的帧回调改为同步执行。 */
  async function pretendHidden(page: Page) {
    await page.addInitScript(() => {
      if (window !== window.top) return;
      Object.defineProperty(document, "hidden", { configurable: true, get: () => true });
    });
  }

  test("页面隐藏时打开题库浏览，题库还在路上：不递归、不报错，数据到了照常出题", async ({ page }) => {
    await seedModern(page);
    await pretendHidden(page);
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.route(/\/questions\/[ABC]\.json/, async (route) => {
      await new Promise((r) => setTimeout(r, 1500));
      await route.continue();
    });
    await page.goto("/browse?bank=A");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(page.locator("#route-content .md-q-row").first()).toBeVisible({ timeout: 20_000 });
    await expect(page.locator("#route-content .md-q-row")).toHaveCount(40);
    expect(errors, "补齐循环不该把栈撑爆").toEqual([]);
  });

  test("页面隐藏时打开空的错题本：不递归、不报错", async ({ page }) => {
    await seedModern(page);
    await pretendHidden(page);
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto("/mistakes");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    // 给循环几帧的时间：旧写法在这里同步递归到栈溢出。
    await page.waitForTimeout(500);
    expect(errors, "补齐循环不该把栈撑爆").toEqual([]);
  });

  test("题库浏览整页直接打开（题库慢到）：第一批仍只建前几行，随后补满一页", async ({ page }) => {
    await seedModern(page);
    // 扣住题库 1.5 秒：补齐不能在数据到达之前就把上限走完，否则数据一到就是一整页一次建完。
    await page.route(/\/questions\/[ABC]\.json/, async (route) => {
      await new Promise((r) => setTimeout(r, 1500));
      await route.continue();
    });
    await page.addInitScript(() => {
      if (window !== window.top) return;
      // 记下行数的第一个非零值（见 `firstFrame` 的说明：观察器不会像轮询那样漏掉中间态）。
      (window as unknown as { __firstRows: number }).__firstRows = 0;
      // 整页打开时 `#route-content` 要等 wasm 起来才有：观察整个文档，每次变动再去找它。
      document.addEventListener("DOMContentLoaded", () => {
        const obs = new MutationObserver(() => {
          const n = document.querySelectorAll("#route-content .md-q-row").length;
          if (n > 0) {
            (window as unknown as { __firstRows: number }).__firstRows = n;
            obs.disconnect();
          }
        });
        obs.observe(document.documentElement, { childList: true, subtree: true });
      });
    });
    await page.goto("/browse?bank=A");
    await expect(page.locator("#route-content .md-q-row")).toHaveCount(40, { timeout: 20_000 });
    const first = await page.evaluate(() => (window as unknown as { __firstRows: number }).__firstRows);
    expect(first, "进页面后一直没有出现题目行").toBeGreaterThan(0);
    expect(first, "数据到达后的第一批不该一次把一页 40 道题都建出来").toBeLessThan(40);
  });
});

test.describe("响应：会话级缓存要跟着数据失效", () => {
  /** 在日志页的表单里记一条（只填呼号，其余走默认）。 */
  async function addQso(page: Page, call: string) {
    const form = page.locator("section", { hasText: "添加通联记录" });
    await form.locator('input[placeholder="BG4XXX"]').fill(call);
    await page.getByRole("button", { name: "添加记录" }).click();
  }

  test("记一条新实体的通联后，站内跳到统计页立刻看到新数字（奖项进度缓存会失效）", async ({ page }) => {
    await seedLog(page, [{ callsign: "JA1AA", band: "20m" }]);
    await page.goto("/stats");
    // 奖项进度由 store 上的会话级缓存提供：第一次读到 1 个实体。
    await expect(page.getByText(/^1 \/ 100$/)).toBeVisible();

    // 站内（不重载）去日志页记一条另一个国家的通联，再站内回统计页。
    await clientNav(page, "/log");
    await expect(page.locator("section", { hasText: "添加通联记录" })).toBeVisible();
    await addQso(page, "DL1AA");
    // 记录列表标题上的条数：写进了日志才会变（添加记录没有 toast，不能等它）。
    await expect(page.getByText("共 2 条", { exact: true })).toBeVisible();
    await clientNav(page, "/stats");
    await expect(page.getByText(/^2 \/ 100$/)).toBeVisible();
    // 日志页的概览与奖项面板读的是同一份缓存，同样要跟上。
    await clientNav(page, "/log");
    await expect(page.locator("#awards")).toBeVisible();
    await expect(page.getByText("2 / 100").first()).toBeVisible();
  });

  test("统计口径：另一个标签页改了口径，站内换页后本页跟上（口径信号只在启动读一次存储的话会过期）", async ({ page }) => {
    // 两个操作员：BA1AA 两条、BG1BB 一条。口径按操作员过滤，统计页的总数跟着变。
    const qso = (id: number, operator: string, callsign: string) => ({
      id,
      operator,
      date: "2026-01-01",
      time: "00:00",
      freq: "14.074",
      band: "20m",
      mode: "FT8",
      callsign,
      gridsquare: "",
      remark: "",
    });
    const log = JSON.stringify({
      entries: [qso(1, "BA1AA", "JA1AA"), qso(2, "BA1AA", "DL1AA"), qso(3, "BG1BB", "W1AW")],
    });
    await page.addInitScript((json) => {
      if (window !== window.top) return;
      if (!sessionStorage.getItem("e2e:seeded-log")) {
        localStorage.setItem("logbook", json);
        sessionStorage.setItem("e2e:seeded-log", "1");
      }
    }, log);
    const total = page.getByText("总 QSO", { exact: true }).locator("..");

    await page.goto("/stats");
    await expect(total).toHaveText(/^3\s*总 QSO$/);

    // 模拟另一个标签页把口径改成「只看 BA1AA」：这个页面的内存信号并不知道，只有存储里的值变了。
    await page.evaluate(() => localStorage.setItem("log-stats-scope", "o:BA1AA"));
    await clientNav(page, "/log");
    await expect(page.locator("section", { hasText: "添加通联记录" })).toBeVisible();
    await clientNav(page, "/stats");
    await expect(total).toHaveText(/^2\s*总 QSO$/);
  });
});

test.describe("响应：重块各排各的任务", () => {
  test("/nec：结果区、三维方向图、扫频、传输线、NVIS、方向表不在同一个任务里挂上", async ({ page }) => {
    await seedModern(page);
    await page.goto("/countdown");
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    // 同一个任务里挂上的节点，会在 `MutationObserver` 的**同一次回调**里一起送到；分在不同任务里则是不同的回调。
    // 记下每块的标记第一次出现在第几次回调里 —— 六块各自的回调序号必须互不相同。
    const firstSeen = await page.evaluate(async () => {
      const root = document.getElementById("route-content") as HTMLElement;
      const markers: Record<string, () => boolean> = {
        结果区: () => (root.textContent ?? "").includes("馈电点阻抗"),
        三维方向图: () => !!root.querySelector('[role="img"][data-renderer]'),
        扫频: () => (root.textContent ?? "").includes("频带扫描（SWR 曲线）"),
        传输线: () => (root.textContent ?? "").includes("传输线（TL）"),
        NVIS: () => (root.textContent ?? "").includes("NVIS 仰角-距离覆盖图"),
        方向表: () => (root.textContent ?? "").includes("方向增益（对比 DXCC 实体）"),
      };
      return new Promise<Record<string, number>>((resolve) => {
        const seen: Record<string, number> = {};
        let call = 0;
        const obs = new MutationObserver(() => {
          call++;
          for (const [name, present] of Object.entries(markers)) if (!(name in seen) && present()) seen[name] = call;
          if (Object.keys(seen).length === Object.keys(markers).length) {
            obs.disconnect();
            resolve(seen);
          }
        });
        obs.observe(root, { childList: true, subtree: true, characterData: true });
        setTimeout(() => {
          obs.disconnect();
          resolve(seen);
        }, 20_000);
        const a = document.createElement("a");
        a.href = "/nec";
        document.body.append(a);
        a.click();
        a.remove();
      });
    });
    expect(Object.keys(firstSeen), "六块都应在 20 秒内挂上").toHaveLength(6);
    const calls = Object.values(firstSeen);
    expect(new Set(calls).size, `六块应各自在不同的任务里挂上，实际回调序号：${JSON.stringify(firstSeen)}`).toBe(6);
  });
});

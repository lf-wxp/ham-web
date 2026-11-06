import type { Page } from "@playwright/test";
import { expect, settleReveal, test } from "./fixtures";

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

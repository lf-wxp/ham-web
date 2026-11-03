import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";

// 布局一致性回归：结构页头、现代风页头不吸顶、尺度收敛、首屏入口。
// 判据写在各用例里；现行的页头 / 尺度规则见 `docs/ui/modern.md`。

async function seed(page: Page) {
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "modern");
    localStorage.setItem("theme", "dark");
  });
}

test("迁移页都有结构页头（标题 + 副标题 + data-page-head）", async ({ page }) => {
  await seed(page);
  for (const url of ["/browse", "/glossary", "/log", "/tools", "/reference", "/bookmarks"]) {
    await page.goto(url);
    const head = page.locator("header[data-page-head]").first();
    await expect(head, `${url} 缺少结构页头`).toBeVisible();
    await expect(head.locator("h1"), `${url} 页头缺 h1`).not.toBeEmpty();
    await expect(head.locator("p.md-lede"), `${url} 页头缺副标题`).toHaveCount(1);
  }
});

test("桥接页与迁移页用同一个页头组件，且不吸顶", async ({ page }) => {
  await seed(page);
  await page.goto("/radio-law");
  // `PageHeader` 在现代风下直接派发到 `ModernPageHead`：桥接页也长这样。
  const header = page.locator("header[data-page-head]").first();
  await expect(header).toBeVisible();
  await expect(header).toHaveCSS("position", "static");
});

test("同层级页面的首块内容起点一致（差值 ≤ 8px）", async ({ page }) => {
  await seed(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1440, height: 1000 });
  const top = async (url: string) => {
    await page.goto(url);
    await page.waitForTimeout(400);
    return page.evaluate(() => {
      const main = document.querySelector("#main-content") as HTMLElement;
      const first = main.querySelector(
        ".md-page > *:not(header), .max-w-5xl.space-y-6 > *",
      ) as HTMLElement | null;
      return first ? Math.round(first.getBoundingClientRect().top + window.scrollY) : -1;
    });
  };
  const migrated = await top("/browse");
  const bridged = await top("/radio-law");
  // 取不到首块时两页都返回 -1，差值为 0 会「通过」：先证明两页都真的量到了。
  expect(migrated, "/browse 没找到首块内容").toBeGreaterThan(0);
  expect(bridged, "/radio-law 没找到首块内容").toBeGreaterThan(0);
  expect(Math.abs(migrated - bridged), `${migrated} vs ${bridged}`).toBeLessThanOrEqual(8);
});

test("字号与卡片层次落在收敛后的尺度内", async ({ page }) => {
  await seed(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1440, height: 1000 });

  await page.goto("/");
  let size = await page.locator("h1").first().evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
  expect(size, "display-1 上限 72px").toBeLessThanOrEqual(72);

  await page.goto("/browse");
  size = await page.locator("h1").first().evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
  expect(size, "页标题上限 52px").toBeLessThanOrEqual(52);

  // 卡片必须有一层投影：完全平面时浅色下只剩 1px 描边，认不出「一张卡」。
  // `/log` 已迁到现代组件（卡片是 `.md-panel`），判据不变，选择器按皮肤各认一个。
  await page.goto("/log");
  await page.waitForTimeout(600);
  const shadow = await page
    .locator(".pxl-card, .md-panel")
    .first()
    .evaluate((el) => getComputedStyle(el).boxShadow);
  expect(shadow).not.toBe("none");

  await page.setViewportSize({ width: 320, height: 844 });
  await page.goto("/");
  const h1 = page.locator("h1").first();
  size = await h1.evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
  expect(size, "320 宽下标题字号上限 44px").toBeLessThanOrEqual(44);
  // 标题写的是「不超过 3 行」，就量行数：高度 ÷ 行高。
  const lines = await h1.evaluate((el) => {
    const cs = getComputedStyle(el);
    const lineHeight = parseFloat(cs.lineHeight) || parseFloat(cs.fontSize) * 1.2;
    return Math.round(el.getBoundingClientRect().height / lineHeight);
  });
  expect(lines, "320 宽下标题不超过 3 行").toBeLessThanOrEqual(3);
});

test("390 首页：插画退场，首屏能看到主行动与第一个内容入口", async ({ page }) => {
  await seed(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");

  await expect(page.locator(".md-hero-art")).toBeHidden();
  await expect(page.getByRole("link", { name: /开始练习/ }).first()).toBeVisible();

  // 看的是**标题**的位置而不是 section 的盒子顶边：section 的上沿被章节线占着，
  // 真正能被认出的入口是标题。780 = 844 视口减去底部 dock（约 64px）。
  const top = await page
    .locator("#main-content .md-page > section")
    .first()
    .locator("h2")
    .evaluate((el) => el.getBoundingClientRect().top + window.scrollY);
  expect(top, "第一个内容入口的标题应露在 dock 之上").toBeLessThanOrEqual(780);
});

test("390 首页的模块入口是纵向列表（不再横向藏起 2/3）", async ({ page }) => {
  await seed(page);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  const rail = page.locator(".md-modules");
  await rail.scrollIntoViewIfNeeded();
  const overflow = await rail.evaluate((el) => el.scrollWidth - el.clientWidth);
  expect(overflow).toBeLessThanOrEqual(1);
  // 标题写的是「纵向列表」：不只是不溢出，还要真的是竖着排（各模块卡的 top 依次递增）。
  const tops = await rail.locator(".md-mod").evaluateAll((els) =>
    els.map((el) => Math.round(el.getBoundingClientRect().top)),
  );
  expect(tops.length, "应有至少两张模块卡").toBeGreaterThan(1);
  expect(tops, "模块卡应纵向依次排开").toEqual([...tops].sort((a, b) => a - b));
  expect(new Set(tops).size, "纵向列表里不应有两张卡在同一行").toBe(tops.length);
});

import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";

const SEARCH = "搜索术语 / 缩写 / 解释…";

/** 读取顶部「当前筛选」统计卡的数值（不受分页上限影响）。 */
async function filteredCount(page: Page) {
  const card = page.getByText("当前筛选", { exact: true }).locator("..");
  return Number((await card.innerText()).replace(/\D/g, ""));
}

test("术语表：?q= 定位到术语并可按分类筛选", async ({ page }) => {
  await page.goto("/glossary?q=%E9%A9%BB%E6%B3%A2%E6%AF%94");
  await expect(page.getByRole("heading", { level: 1, name: "术语表" })).toBeVisible();
  await expect(page.getByPlaceholder(SEARCH)).toHaveValue("驻波比");
  // 相关度排序：完全匹配的术语排在最前
  await expect(page.locator("main article h2").first()).toHaveText("驻波比");

  await page.getByPlaceholder(SEARCH).fill("");
  await expect(page.locator("main article").first()).toBeVisible();

  const before = await filteredCount(page);
  await page.locator("aside").getByRole("button", { name: /天线与馈线/ }).click();
  await expect.poll(() => filteredCount(page)).toBeLessThan(before);
  await expect(page.locator("main article").first()).toBeVisible();
});

test("术语表：搜索无结果给出空提示", async ({ page }) => {
  await page.goto("/glossary");
  await expect(page.getByRole("heading", { level: 1, name: "术语表" })).toBeVisible();
  await page.getByPlaceholder(SEARCH).fill("这个词肯定不存在");
  await expect(page.getByText("没有匹配的术语")).toBeVisible();
  await expect(page.locator("main article")).toHaveCount(0);
});

test("术语表：只看英文缩写会收窄结果", async ({ page }) => {
  await page.goto("/glossary");
  const rows = page.locator("main article");
  await expect(rows.first()).toBeVisible();
  const before = await filteredCount(page);

  await page.getByRole("button", { name: "只看英文缩写" }).click();
  await expect.poll(() => filteredCount(page)).toBeLessThan(before);
  expect(await filteredCount(page)).toBeGreaterThan(0);

  // 关掉开关后恢复
  await page.getByRole("button", { name: "只看英文缩写" }).click();
  await expect.poll(() => filteredCount(page)).toBe(before);
});

// 现代风没有「当前筛选」统计格（默认筛选下它恒等于总数），结果数是列表正上方的一行小字；
// 像素风的可见计数在统计卡里，这一行只是 `sr-only` 的播报。两套皮肤共用同一个 `role="status"` 节点。
for (const style of ["pixel", "modern"] as const) {
  test(`术语表（${style}）：筛选结果数随搜索、分类与「只看英文缩写」变化${style === "modern" ? "，且在列表上方可见" : "（读屏播报）"}`, async ({
    page,
  }) => {
    await page.addInitScript((s) => {
      if (window !== window.top) return;
      localStorage.setItem("ui:uiStyle", s);
    }, style);
    await page.goto("/glossary");
    const cards = page.locator("main article");
    await expect(cards.first()).toBeVisible();

    const status = page.getByRole("status").filter({ hasText: /^筛选结果 \d+ 条$/ });
    await expect(status).toHaveCount(1);
    const count = async () => Number(((await status.textContent()) ?? "").replace(/\D/g, ""));
    const total = await count();
    expect(total, "默认筛选下就是全部术语").toBeGreaterThan(0);

    if (style === "modern") {
      await expect(status).toBeVisible();
      const [s, c] = await Promise.all([status.boundingBox(), cards.first().boundingBox()]);
      expect(s && s.height, "结果数这一行应占高度").toBeGreaterThan(0);
      expect(s!.y + s!.height, "结果数应在第一张术语卡之上").toBeLessThanOrEqual(c!.y + 1);
    } else {
      await expect(status).toHaveClass(/sr-only/);
    }

    // 搜索收窄：数字变小且仍大于 0。
    await page.getByPlaceholder(SEARCH).fill("驻波比");
    await expect.poll(count).toBeLessThan(total);
    expect(await count()).toBeGreaterThan(0);

    // 筛到零条：写 0，而不是留着旧数字。
    await page.getByPlaceholder(SEARCH).fill("这个词肯定不存在");
    await expect(status).toHaveText("筛选结果 0 条");

    // 清空搜索回到全部。
    await page.getByPlaceholder(SEARCH).fill("");
    await expect.poll(count).toBe(total);

    // 切分类也算筛选：像素风在左侧栏，现代风在胶囊行（>=768 才显示，默认视口够宽）。
    const catBar = style === "modern" ? page.locator(".md-menu-chips") : page.locator("aside");
    await catBar.getByRole("button", { name: /天线与馈线/ }).click();
    await expect.poll(count).toBeLessThan(total);
    expect(await count()).toBeGreaterThan(0);
    await catBar.getByRole("button", { name: /全部术语/ }).click();
    await expect.poll(count).toBe(total);

    // 「只看英文缩写」也算筛选。
    await page.getByRole("button", { name: "只看英文缩写" }).click();
    await expect.poll(count).toBeLessThan(total);
  });
}

test("术语表：加载更多分页", async ({ page }) => {
  await page.goto("/glossary");
  const rows = page.locator("main article");
  await expect(rows.first()).toBeVisible();

  const more = page.getByRole("button", { name: /加载更多/ });
  await expect(more).toBeVisible();
  const before = await rows.count();
  await more.click();
  await expect.poll(() => rows.count()).toBeGreaterThan(before);
});

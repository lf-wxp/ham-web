import { expect, test } from "./fixtures";

/** 唤起全站搜索面板（`/` 快捷键），返回面板定位器。 */
async function openSearch(page: import("@playwright/test").Page) {
  await page.keyboard.press("/");
  const panel = page.getByRole("dialog", { name: "全站搜索" });
  await expect(panel).toBeVisible();
  return panel;
}

test("全站搜索：结果按相关度排序，标题命中的排在最前", async ({ page }) => {
  await page.goto("/");
  const panel = await openSearch(page);
  await panel.getByLabel("搜索关键词").fill("驻波比");

  const first = panel.locator("a").first();
  await expect(first).toBeVisible();
  // 相关度排序：直接以查询词为标题的条目应排在最前，而不是排在只在正文提到它的条目之后。
  await expect(first.locator("div").first()).toContainText("驻波比");
});

test("全站搜索：多词查询按 AND 匹配并逐词高亮", async ({ page }) => {
  await page.goto("/");
  const panel = await openSearch(page);
  await panel.getByLabel("搜索关键词").fill("天线 驻波比");

  // 整串（含空格）当子串去查几乎必然无结果；切成两词后应能命中。
  await expect(panel.locator("a").first()).toBeVisible();

  const marked = await panel.locator("mark").allInnerTexts();
  expect(marked.length).toBeGreaterThan(0);
  // 每个词都要被高亮，而不是只高亮整串
  expect(marked.some((m) => m.includes("天线"))).toBe(true);
  expect(marked.some((m) => m.includes("驻波比"))).toBe(true);
});

test("全站搜索：高频词结果有上限，不会一次铺开全部命中", async ({ page }) => {
  await page.goto("/");
  const panel = await openSearch(page);
  await panel.getByLabel("搜索关键词").fill("天线");

  await expect(panel.locator("a").first()).toBeVisible();
  const count = await panel.locator("a").count();
  // 上层截断到 30 条；同时确认确实有结果（不是因为没匹配才少）
  expect(count).toBeGreaterThan(0);
  expect(count).toBeLessThanOrEqual(30);
});

test("全站搜索：无结果时给出提示而非空白", async ({ page }) => {
  await page.goto("/");
  const panel = await openSearch(page);
  await panel.getByLabel("搜索关键词").fill("这个词肯定不存在于站点中");

  await expect(panel.getByText(/未找到与/)).toBeVisible();
  await expect(panel.locator("a")).toHaveCount(0);
});

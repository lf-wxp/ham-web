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

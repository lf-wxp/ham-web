import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";

const SEARCH = "搜索题干 / 答案 / 解析…";

/** 读取顶部「当前筛选」统计卡的数值（不受分页上限影响）。 */
async function filteredCount(page: Page) {
  // Stat 卡结构：<div>数值</div><div>当前筛选</div>
  const card = page.getByText("当前筛选", { exact: true }).locator("..");
  return Number((await card.innerText()).replace(/\D/g, ""));
}

test("分类浏览：搜索命中过滤，无结果给出空提示", async ({ page }) => {
  await page.goto("/browse");
  await expect(page.getByRole("heading", { level: 1, name: "题库分类浏览" })).toBeVisible();
  const rows = page.locator("main article");
  await expect(rows.first()).toBeVisible();

  const before = await filteredCount(page);
  await page.getByPlaceholder(SEARCH).fill("天线");
  await expect.poll(() => filteredCount(page)).toBeLessThan(before);
  expect(await filteredCount(page)).toBeGreaterThan(0);
  await expect(rows.first()).toBeVisible();

  await page.getByPlaceholder(SEARCH).fill("这个词肯定不存在");
  await expect(page.getByText("没有匹配的题目")).toBeVisible();
  await expect(rows).toHaveCount(0);
});

test("分类浏览：?q= 预填关键词", async ({ page }) => {
  await page.goto("/browse?q=%E9%A9%BB%E6%B3%A2%E6%AF%94");
  await expect(page.getByPlaceholder(SEARCH)).toHaveValue("驻波比");
  await expect(page.locator("main article").first()).toBeVisible();
});

test("分类浏览：只看多选后每道题都带多选徽标", async ({ page }) => {
  await page.goto("/browse");
  await expect(page.locator("main article").first()).toBeVisible();
  await page.getByRole("button", { name: "只看多选" }).click();

  const rows = page.locator("main article");
  await expect(rows.first()).toBeVisible();
  const count = await rows.count();
  expect(count).toBeGreaterThan(0);
  await expect(rows.getByText("多选", { exact: true })).toHaveCount(count);
});

test("分类浏览：一级分类筛选与只看本类新增", async ({ page }) => {
  await page.goto("/browse");
  const rows = page.locator("main article");
  await expect(rows.first()).toBeVisible();

  await page.locator("aside").getByRole("button", { name: "天线与馈线" }).click();
  await expect.poll(() => rows.count()).toBeGreaterThan(0);
  await expect(rows.first().getByText("天线与馈线")).toBeVisible();

  // 回到全部
  await page.locator("aside").getByRole("button", { name: "全部题目" }).click();
  await expect(rows.first()).toBeVisible();

  // B 类「只看本类新增」会排除与 A 类重合的题，题量不应增加
  await page.getByRole("button", { name: "B 类" }).first().click();
  await expect(rows.first()).toBeVisible();
  const allB = await rows.count();
  await page.getByRole("button", { name: "只看本类新增" }).click();
  await expect.poll(() => rows.count()).toBeLessThanOrEqual(allB);
});

test("分类浏览：加载更多分页", async ({ page }) => {
  await page.goto("/browse");
  const rows = page.locator("main article");
  await expect(rows.first()).toBeVisible();

  const more = page.getByRole("button", { name: /加载更多/ });
  await expect(more).toBeVisible();
  const before = await rows.count();
  await more.click();
  await expect.poll(() => rows.count()).toBeGreaterThan(before);
});

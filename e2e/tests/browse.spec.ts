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
  await page.getByRole("radio", { name: "B 类" }).first().click();
  await expect(rows.first()).toBeVisible();
  const allB = await rows.count();
  await page.getByRole("button", { name: "只看本类新增" }).click();
  await expect.poll(() => rows.count()).toBeLessThanOrEqual(allB);
});

test("分类浏览：卡片可就地收藏，取消后从收藏集消失", async ({ page }) => {
  // 无障碍名恒为「收藏题目 {题号}」：状态只交给 aria-pressed，
  // 所以同一个定位器在点击前后都命中；可见文字仍在「收藏 / 已收藏」间切换。
  const BOOKMARK = /^收藏题目 \S+/;
  await page.goto("/browse");
  const rows = page.locator("main article");
  const first = rows.first();
  const second = rows.nth(1);
  await expect(second).toBeVisible();

  const add = first.getByRole("button", { name: BOOKMARK });
  await expect(add).toHaveAttribute("aria-pressed", "false");
  await expect(add).toHaveText("收藏");
  await add.click();
  await expect(add).toHaveAttribute("aria-pressed", "true");
  await expect(add).toHaveText("已收藏");
  // 收藏状态按题目 id 派生：点第一题不能连带第二题变色
  await expect(second.getByRole("button", { name: BOOKMARK })).toHaveAttribute(
    "aria-pressed",
    "false",
  );

  await page.goto("/bookmarks");
  await expect(page.getByText("共 1 题", { exact: true })).toBeVisible();

  // 回到浏览页：收藏状态从存储里恢复，取消后收藏集回到空态
  await page.goto("/browse");
  const again = rows.first().getByRole("button", { name: BOOKMARK });
  await expect(again).toHaveAttribute("aria-pressed", "true");
  await again.click();
  await expect(again).toHaveAttribute("aria-pressed", "false");

  await page.goto("/bookmarks");
  await expect(page.getByText("暂无收藏")).toBeVisible();
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

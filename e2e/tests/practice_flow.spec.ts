import type { Page } from "@playwright/test";
import { expect, option, test } from "./fixtures";

/** 底部栏统计行：「题库 A 类 · 顺序练习 · 进度 1 / 100」。 */
function bar(page: Page) {
  return page.locator("main").getByText(/题库 [ABC] 类 ·/).first();
}

/** 开关类按钮没有 aria-pressed，用高亮类名判断选中状态。 */
async function isOn(locator: ReturnType<Page["getByRole"]>) {
  const cls = (await locator.getAttribute("class")) ?? "";
  return cls.includes("bg-primary");
}

test("练习：切换题库会改写地址栏并换题", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await expect(bar(page)).toContainText("题库 A 类");
  await expect(bar(page)).toContainText("顺序练习");

  await page.getByRole("button", { name: "B 类" }).first().click();
  await expect(page).toHaveURL(/bank=B/);
  await expect(bar(page)).toContainText("题库 B 类");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();

  await page.getByRole("button", { name: "C 类" }).first().click();
  await expect(page).toHaveURL(/bank=C/);
  await expect(bar(page)).toContainText("题库 C 类");
});

test("练习：只看本类新增 / 只练没做过可切换", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();

  const unseen = page.getByRole("button", { name: "只练没做过" });
  expect(await isOn(unseen)).toBe(false);
  await unseen.click();
  expect(await isOn(unseen)).toBe(true);
  // 全新用户没有做过的题，题量不变
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();

  await unseen.click();
  expect(await isOn(unseen)).toBe(false);

  const unique = page.getByRole("button", { name: "只看本类新增" });
  await unique.click();
  expect(await isOn(unique)).toBe(true);
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
});

test("练习：搜索题目可跳转，未命中时给出提示", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();

  await page.keyboard.press("Enter");
  const search = page.getByRole("dialog", { name: "搜索题目" });
  await expect(search).toBeVisible();
  await search.getByRole("textbox").first().fill("天线");
  await expect(search.getByText(/匹配 \d+ 条/)).toBeVisible();
  await search.locator("li").filter({ hasText: /第 \d+ 题/ }).first().click();
  await expect(search).toBeHidden();
  await expect(page.getByText(/第 \d+ \/ \d+ 题/).first()).toBeVisible();

  // 不存在的关键词：给出空提示而不是空白
  await page.keyboard.press("Enter");
  await search.getByRole("textbox").first().fill("这个词肯定不存在");
  await expect(search.getByText("未找到匹配")).toBeVisible();
});

test("练习：切到随机题序且有作答时先确认", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await option(page, "A").click();

  await page.getByRole("button", { name: "设置" }).first().click();
  const settings = page.getByRole("dialog", { name: "设置" });
  await expect(settings).toBeVisible();
  await settings.getByRole("radio", { name: "随机" }).click();

  const confirm = page.getByRole("dialog", { name: "切换题序将清空作答" });
  await expect(confirm).toBeVisible();
  await confirm.getByRole("button", { name: "确定切换" }).click();
  await expect(confirm).toBeHidden();
  await expect(bar(page)).toContainText("随机练习");
});

test("练习：收藏后进入收藏集，再取消收藏", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await page.getByRole("button", { name: "收藏" }).first().click();

  await page.goto("/bookmarks");
  await expect(page.getByRole("heading", { level: 1, name: "收藏集" })).toBeVisible();
  await expect(page.getByText("共 1 题")).toBeVisible();

  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await page.getByRole("button", { name: "收藏" }).first().click();

  await page.goto("/bookmarks");
  await expect(page.getByText("暂无收藏")).toBeVisible();
});

test("练习：专项链接（?multi=1）只看多选题且强制随机序", async ({ page }) => {
  await page.goto("/practice?bank=A&multi=1");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await expect(bar(page)).toContainText("随机练习");
  await expect(page.getByText("多选", { exact: true }).first()).toBeVisible();
});

test("练习：专项链接（?topic=）按分类过滤并强制随机序", async ({ page }) => {
  await page.goto("/practice?bank=A&topic=%E5%A4%A9%E7%BA%BF");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await expect(bar(page)).toContainText("随机练习");
});

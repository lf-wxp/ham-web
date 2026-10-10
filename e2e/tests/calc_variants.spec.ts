import { expect, option, test } from "./fixtures";

/** 打开练习内搜索并跳到第一道变体题。 */
async function jumpToVariant(page: import("@playwright/test").Page) {
  // 等组卷完成（变体开关切换会重载题库，重载期间的按键可能被吞掉）。
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  // 用练习头部搜索按钮（页面上还有全局导航的搜索，用 title 区分）而不是回车快捷键。
  await page.getByRole("button", { name: "搜索", description: "搜索", exact: true }).click();
  const search = page.getByRole("dialog", { name: "搜索题目" });
  await expect(search).toBeVisible();
  await search.getByRole("textbox").first().fill("变体");
  await expect(search.getByText(/匹配 \d+ 条/)).toBeVisible();
  await search.locator("li").filter({ hasText: /第 \d+ 题/ }).first().click();
  await expect(search).toBeHidden();
}

test("计算变体：设置里开启，同型题换数进练习", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  // 默认没有变体：搜索「变体」匹配 0 条。
  await page
    .getByRole("button", { name: "搜索", description: "搜索", exact: true })
    .click();
  const search = page.getByRole("dialog", { name: "搜索题目" });
  await search.getByRole("textbox").first().fill("变体");
  await expect(search.getByText(/匹配 0 条/)).toBeVisible();
  await page.keyboard.press("Escape");

  // 设置里打开「计算变体」→ 重新组卷后变体出现。
  await page.getByRole("button", { name: "设置" }).click();
  const dialog = page.getByRole("dialog", { name: "设置" });
  await expect(dialog.getByText("计算变体")).toBeVisible();
  await dialog.getByLabel("计算变体").click();
  await page.keyboard.press("Escape");

  await jumpToVariant(page);
  await expect(page.getByText(/〔变体 · /)).toBeVisible();
  await expect(option(page, "A")).toBeVisible();
  await expect(option(page, "D")).toBeVisible();
});

test("计算变体：答案与解析按同一公式给出", async ({ page }) => {
  await page.goto("/practice?variant=1");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await jumpToVariant(page);
  const stem = page.getByText(/〔变体 · /);
  await expect(stem).toBeVisible();

  // 作答后解析里出现同一公式的代入过程（核心有单测钉住每个参数组的答案）。
  await option(page, "A").click();
  await page.getByRole("button", { name: "答案解析" }).click();
  await expect(
    page.getByText(/波长 λ（米）|增益 G（dB）|衰减后功率|谐振频率 f/),
  ).toBeVisible();
});

test("计算变体：变体题不能收藏（按钮置灰），真题仍可收藏", async ({ page }) => {
  await page.goto("/practice?variant=1");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  const bookmark = page.getByRole("button", { name: "收藏", exact: true });
  // 第一题是官方真题：可以收藏。
  await expect(bookmark).toBeEnabled();
  // 跳到变体题：置灰，并说明原因。
  await jumpToVariant(page);
  await expect(page.getByText(/〔变体 · /)).toBeVisible();
  await expect(bookmark).toBeDisabled();
  await expect(bookmark).toHaveAttribute("title", /不能收藏/);
});

test("计算变体：开启时练习按子集处理，不写顺序进度", async ({ page }) => {
  await page.goto("/practice?variant=1");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  // 作答会触发「保存进度」的 Effect：变体开启时题目总数是 N+8，写进去会覆盖完整题库的进度。
  await option(page, "A").click();
  await page.waitForTimeout(800);
  const keys = await page.evaluate(() =>
    Object.keys(window.localStorage).filter(
      (k) => k.startsWith("practice:") && k !== "practice:lastMode",
    ),
  );
  expect(keys).toEqual([]);
});

import { expect, test } from "./fixtures";

test("易错知识点：无错题时给出引导", async ({ page }) => {
  await page.goto("/mistake-topics");
  await expect(page.getByRole("heading", { level: 1, name: "易错知识点" })).toBeVisible();
  await expect(page.getByText("暂无错题")).toBeVisible();
  await expect(page.getByRole("link", { name: /全部错题/ })).toBeVisible();
});

test("易错知识点：按知识点聚合错题并可跳到同类题", async ({ page }) => {
  // 用闪卡自评「不会」造一条错题
  await page.goto("/flashcards");
  await expect(page.getByRole("heading", { level: 1, name: "闪卡刷题" })).toBeVisible();
  await page.getByRole("button", { name: "显示答案" }).click();
  await page.getByRole("button", { name: "不会 ✗" }).click();

  await page.goto("/mistake-topics");
  await expect(page.getByRole("heading", { level: 1, name: "易错知识点" })).toBeVisible();
  await expect(page.getByText("暂无错题")).toHaveCount(0);

  const items = page.locator('main a[href^="/browse?sub="]');
  await expect(items.first()).toBeVisible();
  await expect(items.first()).toContainText(/1 题/);
  await expect(items.first()).toContainText(/累计答错 \d+ 次/);
});

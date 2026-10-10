import { expect, test } from "./fixtures";

test("易错知识点：无错题时给出引导", async ({ page }) => {
  await page.goto("/mistake-topics");
  await expect(page.getByRole("heading", { level: 1, name: "易错知识点" })).toBeVisible();
  await expect(page.getByText("暂无错题")).toBeVisible();
  await expect(page.getByRole("link", { name: /全部错题/ })).toBeVisible();
});

test("易错知识点：错题归因热力图，点击直达补弱练习", async ({ page }) => {
  // 用闪卡自评「不会」造一条错题
  await page.goto("/flashcards");
  await expect(page.getByRole("heading", { level: 1, name: "闪卡刷题" })).toBeVisible();
  await page.getByRole("button", { name: "显示答案" }).click();
  await page.getByRole("button", { name: "不会 ✗" }).click();

  await page.goto("/mistake-topics");
  await expect(page.getByRole("heading", { level: 1, name: "易错知识点" })).toBeVisible();
  await expect(page.getByText("暂无错题")).toHaveCount(0);

  // 知识点格子：直达该知识点的专项练习；域行：练整个域。
  const cells = page.locator('a[href^="/practice?sub="]');
  await expect(cells.first()).toBeVisible();
  await expect(cells.first()).toContainText("1");
  const domain = page.locator('a[href^="/practice?topic="]');
  await expect(domain.first()).toBeVisible();
  await expect(domain.first()).toContainText(/1 题/);
  await expect(domain.first()).toContainText(/累计答错 \d+ 次/);
  // 点击知识点 → 专项练习。
  await cells.first().click();
  await expect(page).toHaveURL(/practice\?sub=/);
});

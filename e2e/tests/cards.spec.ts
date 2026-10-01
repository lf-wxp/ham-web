import { expect, test } from "./fixtures";

test("知识卡片：翻面、自评、忘了的卡本轮再出现，首页显示进度", async ({ page }) => {
  await page.goto("/cards?deck=phonetic");
  await expect(page.getByRole("heading", { level: 1, name: "知识卡片" })).toBeVisible();
  await expect(page.getByText("第 1 / 10 张")).toBeVisible();

  await page.getByRole("button", { name: "翻面查看答案" }).click();
  await expect(page.getByTestId("card-back")).toHaveText("Alfa");
  await page.getByRole("button", { name: /^记得/ }).click();
  await expect(page.getByText("第 2 / 10 张")).toBeVisible();
  await expect(page.getByText("2 天后再复习")).toBeVisible();

  // 键盘：空格翻面，1 = 忘了；忘了的卡追加到本轮末尾
  await page.keyboard.press(" ");
  await expect(page.getByTestId("card-back")).toHaveText("Bravo");
  await page.keyboard.press("1");
  await expect(page.getByText("10 分钟后再来一次")).toBeVisible();
  await expect(page.getByText("第 3 / 11 张")).toBeVisible();

  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem("card-review") ?? "{}"));
  expect(Object.keys(saved.cards).sort()).toEqual(["p:A", "p:B"]);

  // 切换卡组
  await page.getByRole("button", { name: "Q 简语" }).click();
  await expect(page.locator("button[aria-label='翻面查看答案'] span").first()).toHaveText(/^Q[A-Z]{2}$/);

  await page.goto("/");
  await expect(page.getByText("已学 2 张")).toBeVisible();
  await expect(page.getByRole("link", { name: "学新卡" })).toBeVisible();
});

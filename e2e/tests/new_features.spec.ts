import { correctLetters, expect, option, test, wrongLetter } from "./fixtures";

test("呼号抄收：输入判定后进入下一题", async ({ page }) => {
  await page.goto("/callsign-copy");
  await expect(page.getByRole("heading", { level: 1, name: "呼号抄收训练" })).toBeVisible();

  // 输入明显错误的呼号，核对后应显示「答案是 …」
  await page.getByLabel("呼号输入").fill("XX");
  await page.getByRole("button", { name: "核对" }).click();
  await expect(page.getByText(/^答案是 /)).toBeVisible();

  // 进入下一题，回到未判定状态
  await page.getByRole("button", { name: "下一题" }).click();
  await expect(page.getByRole("button", { name: "核对" })).toBeVisible();
  await expect(page.getByText(/^答案是 /)).toHaveCount(0);
});

test("错题集：闪卡复习显示答案后自评", async ({ page }) => {
  // 先在练习里答错一题，让错题本有数据
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  const correct = await correctLetters(page);
  await option(page, wrongLetter(correct)).click();
  await page.getByRole("button", { name: "下一题" }).first().click();
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();

  await page.goto("/mistakes");
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();

  // 进入闪卡复习，先显示答案，再自评「不会」
  await page.getByRole("button", { name: "闪卡复习" }).click();
  await expect(page.getByRole("button", { name: "显示答案" })).toBeVisible();
  await page.getByRole("button", { name: "显示答案" }).click();
  await expect(page.getByRole("button", { name: "会 ✓", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "不会 ✗", exact: true }).click();
  // 只有一道错题，自评后回到列表
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();
});

test("学习周报：空数据下正常渲染关键卡片", async ({ page }) => {
  await page.goto("/weekly");
  await expect(page.getByRole("heading", { level: 1, name: "学习周报" })).toBeVisible();
  await expect(page.getByText("本周作答")).toBeVisible();
  await expect(page.getByText("连续打卡（天）")).toBeVisible();
  await expect(page.getByText("近 14 天作答趋势")).toBeVisible();
});

test("错题集：考点为可点击链接并显示正确率", async ({ page }) => {
  // 练习答错一题，让错题本有数据
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  const correct = await correctLetters(page);
  await option(page, wrongLetter(correct)).click();
  await page.getByRole("button", { name: "下一题" }).first().click();
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();

  await page.goto("/mistakes");
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();

  // 「考点：」所在的容器内应有指向专题页的链接，并附带正确率
  const topicSpan = page.locator("main").getByText("考点：").first();
  const link = topicSpan.locator("a").first();
  await expect(link).toBeVisible();
  expect(await link.getAttribute("href")).toMatch(/^\//);
  await expect(topicSpan.getByText(/正确率 \d+%/)).toBeVisible();
});

test("CW 单词抄收：输入判定后进入下一题", async ({ page }) => {
  await page.goto("/morse");
  const section = page.getByRole("heading", { name: /单词抄收/ }).locator("..");

  // 输入明显错误的单词，核对后应显示「答案是 …」
  await section.getByLabel("抄收输入").fill("ZZZ");
  await section.getByRole("button", { name: "核对" }).click();
  await expect(section.getByText(/^答案是 /)).toBeVisible();

  // 进入下一题，回到未判定状态
  await section.getByRole("button", { name: "下一题" }).click();
  await expect(section.getByRole("button", { name: "核对" })).toBeVisible();
});

test("元件识别：根据符号选元件名", async ({ page }) => {
  await page.goto("/electronics");
  const section = page.getByRole("heading", { name: /元件识别/ }).locator("..");

  // 点一个选项，应显示「正确！」或「正确答案：…」
  await section
    .locator("button")
    .filter({ hasText: /电阻|电容|电感|二极管|三极管|场效应管|变压器|晶振|保险丝/ })
    .first()
    .click();
  await expect(section.getByText(/^正确！|^正确答案：/)).toBeVisible();

  await section.getByRole("button", { name: "下一题" }).click();
  await expect(section.getByText(/^正确！|^正确答案：/)).toHaveCount(0);
});

test("Q 简语反向测验：给含义选简语", async ({ page }) => {
  await page.goto("/q-code");
  const section = page.getByRole("heading", { name: /Q 简语反向测验/ }).locator("..");

  // 点一个 Q 简语选项，应显示「正确！」或「正确答案：…」
  await section.locator("button").filter({ hasText: /^Q[A-Z]{2}$/ }).first().click();
  await expect(section.getByText(/^正确！|^正确答案：/)).toBeVisible();
});

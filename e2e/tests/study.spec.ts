import { correctLetters, expect, option, test, wrongLetter } from "./fixtures";

test("练习：答错后翻页，题号前进且错题本记录该题", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();

  const correct = await correctLetters(page);
  await option(page, wrongLetter(correct)).click();
  await page.getByRole("button", { name: "下一题" }).first().click();
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();

  await page.goto("/mistakes");
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();
  await expect(page.getByText(/共 1 道错题/)).toBeVisible();
});

test("练习：刷新后可恢复进度", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  // 至少作答 3 题才会提示恢复（RESUME_MIN_ANSWERED）
  for (const n of [2, 3, 4]) {
    await option(page, "A").click();
    await page.getByRole("button", { name: "下一题" }).first().click();
    await expect(page.getByText(new RegExp(`第 ${n} / \\d+ 题`))).toBeVisible();
  }

  await page.reload();
  const resume = page.getByRole("dialog", { name: "发现上次练习记录" });
  await expect(resume).toBeVisible();
  await resume.getByRole("button", { name: "继续上次" }).click();
  await expect(resume).toBeHidden();
  await expect(page.getByText(/第 4 \/ \d+ 题/)).toBeVisible();
});

test("模拟考试：作答并交卷后显示成绩", async ({ page }) => {
  await page.goto("/exam");
  await expect(page.getByText(/考试类别：/).first()).toBeVisible();
  await option(page, "A").click();
  await expect(page.getByText(/已作答 1 \/ \d+/).first()).toBeVisible();

  await page.getByRole("button", { name: "交卷", exact: true }).first().click();
  const confirm = page.getByRole("dialog", { name: "确认交卷？" });
  await expect(confirm).toBeVisible();
  await confirm.getByRole("button", { name: "确认交卷" }).click();

  const result = page.getByRole("dialog", { name: "成绩" });
  await expect(result).toBeVisible();
  await expect(result.getByText(/得分：\d+ \/ \d+/)).toBeVisible();
  await expect(result.getByText(/^不合格/)).toBeVisible();
  await result.getByRole("button", { name: "继续浏览题目" }).click();
  await expect(result).toBeHidden();
  await expect(page.getByRole("button", { name: "已交卷" }).first()).toBeVisible();
});

test("薄弱项组卷：按常规配额出卷，交卷后显示分类对比且不计入历史趋势", async ({ page }) => {
  await page.goto("/exam?bank=A&mode=weak");
  await expect(page.getByRole("heading", { level: 1, name: "薄弱项组卷" })).toBeAttached();
  await expect(page.getByText(/试题数：40（单选 32，多选 8）/).first()).toBeVisible();
  await option(page, "A").click();
  await page.getByRole("button", { name: "交卷", exact: true }).first().click();
  await page.getByRole("dialog", { name: "确认交卷？" }).getByRole("button", { name: "确认交卷" }).click();

  const result = page.getByRole("dialog", { name: "成绩" });
  await expect(result.getByText("分类对比（本次 / 以往）")).toBeVisible();
  await expect(result.getByText(/不计入备考状态/)).toBeVisible();
  const history = await page.evaluate(() => JSON.parse(localStorage.getItem("exam-history") ?? "[]"));
  expect(history).toHaveLength(1);
  expect(history[0].weak).toBe(true);
  expect(await page.evaluate(() => Object.keys(localStorage).some((k) => k.startsWith("exam:savedState")))).toBe(false);
});

test("错题本：重练答对后提示正确", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  const correct = await correctLetters(page);
  await option(page, wrongLetter(correct)).click();
  await page.getByRole("button", { name: "下一题" }).first().click();
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();

  await page.goto("/mistakes");
  await page.getByRole("button", { name: "全部重练" }).click();
  await expect(page.getByText(/第 1 \/ 1 题/)).toBeVisible();
  for (const letter of correct) {
    await option(page, letter).click();
  }
  await page.getByRole("button", { name: "提交" }).click();
  await expect(page.getByText(/^正确！/)).toBeVisible();
});

test.describe("首次进入", () => {
  test.use({ helpSeen: false });

  test("练习页自动弹出快捷键说明，仅一次", async ({ page }) => {
    await page.goto("/practice");
    const help = page.getByRole("dialog", { name: "设置" });
    await expect(help).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(help).toBeHidden();
    await page.reload();
    await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
    await page.waitForTimeout(800);
    await expect(help).toBeHidden();
  });
});

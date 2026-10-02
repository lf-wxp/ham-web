import { expect, test } from "./fixtures";

/** 注入一份复盘快照到 localStorage（避免为了测试真的考完一套题）。 */
const REVIEW = {
  bank: "A",
  finished_at_ms: 1_700_000_000_000,
  items: [
    {
      id: "A-1",
      code: "LK0001",
      question: "第一题题干",
      options: ["A. 甲", "B. 乙"],
      answer: ["A"],
      given: ["A"],
      explanation: "第一题解析",
      category: "1.1.1",
    },
    {
      id: "A-2",
      code: "LK0002",
      question: "第二题题干",
      options: ["A. 甲", "B. 乙"],
      answer: ["B"],
      given: ["A"],
      explanation: "第二题解析",
      category: "2.2.2",
    },
    {
      id: "A-3",
      code: "LK0003",
      question: "第三题题干",
      options: ["A. 甲", "B. 乙"],
      answer: ["B"],
      given: [],
      explanation: "",
      category: "2.2.2",
    },
  ],
};

test("考后复盘：未考试时给出引导而非空白", async ({ page }) => {
  await page.goto("/exam-review");
  await expect(page.getByRole("heading", { level: 1, name: "考后复盘" })).toBeVisible();
  await expect(page.getByText("还没有可复盘的考试记录。")).toBeVisible();
  await expect(page.getByRole("link", { name: "去做一套模拟考试" })).toBeVisible();
});

test("考后复盘：逐题展示对错、你的作答与解析", async ({ page }) => {
  await page.addInitScript((data) => {
    localStorage.setItem("exam:last-review", JSON.stringify(data));
  }, REVIEW);

  await page.goto("/exam-review");
  await expect(page.getByText(/^答对/)).toBeVisible();
  // 3 题里答对 1 题
  await expect(page.getByText("1", { exact: true }).first()).toBeVisible();

  await expect(page.getByText("第二题题干")).toBeVisible();
  await expect(page.getByText("第二题解析")).toBeVisible();
  await expect(page.getByText("错误").first()).toBeVisible();
  // 未作答单独标出，而不是笼统计为「错误」
  await expect(page.getByText("未作答")).toBeVisible();
});

test("考后复盘：分类表现按错得多的在前排序", async ({ page }) => {
  await page.addInitScript((data) => {
    localStorage.setItem("exam:last-review", JSON.stringify(data));
  }, REVIEW);

  await page.goto("/exam-review");
  const codes = await page.locator("li").filter({ hasText: /错 \d/ }).allInnerTexts();
  // 2.2.2 错了 2 题，1.1.1 错了 0 题
  expect(codes.length).toBe(2);
  expect(codes[0]).toContain("2.2.2");
  expect(codes[0]).toContain("错 2");
  expect(codes[1]).toContain("1.1.1");
});

test("考试交卷后：结果面板提供复盘入口", async ({ page }) => {
  await page.goto("/exam");
  await expect(page.getByText(/考试类别：/).first()).toBeVisible();

  await page.getByRole("button", { name: "交卷", exact: true }).first().click();
  await page
    .getByRole("dialog", { name: "确认交卷？" })
    .getByRole("button", { name: "确认交卷" })
    .click();

  const result = page.getByRole("dialog", { name: "成绩" });
  await expect(result).toBeVisible();
  const link = result.getByRole("link", { name: "逐题复盘" });
  await expect(link).toBeVisible();
  await expect(link).toHaveAttribute("href", "/exam-review");

  // 交卷应写入复盘快照（写入被推到下一个宏任务，避免挡住成绩弹窗那一帧，故轮询等待）
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("exam:last-review")))
    .toBeTruthy();
  const saved = await page.evaluate(() => localStorage.getItem("exam:last-review"));
  const parsed = JSON.parse(saved as string);
  expect(parsed.items.length).toBeGreaterThan(0);
});

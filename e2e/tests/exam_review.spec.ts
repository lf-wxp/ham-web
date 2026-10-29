import {
  type BankQuestion,
  bankQuestion,
  expandExplanations,
  expect,
  explanationHead,
  test,
} from "./fixtures";

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
  // 解析是折叠卡片：默认收起，点开才读得到；没有解析的第三题给出「暂无解析」提示
  const explain = page.getByRole("button", { name: "答案解析" });
  await expect(explain).toHaveCount(2);
  await expect(explain.first()).toHaveAttribute("aria-expanded", "false");
  // 折叠时正文不在 DOM 里（首次展开才构建），而不是只被样式藏起来
  await expect(page.getByText("第二题解析")).toHaveCount(0);
  await explain.nth(1).click();
  await expect(explain.nth(1)).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByText("第二题解析")).toBeVisible();
  await expect(page.getByText("此题暂无解析")).toHaveCount(1);
  await expect(page.getByText("错误").first()).toBeVisible();
  // 未作答单独标出，而不是笼统计为「错误」
  await expect(page.getByText("未作答")).toBeVisible();
});

/** 与题库同一道题的复盘项（题干 / 选项 / 答案逐字一致），只有存档里的解析是订正前的。 */
function reviewItem(q: BankQuestion, overrides: Record<string, unknown> = {}) {
  return {
    id: q.id,
    code: "LY0001",
    question: q.question,
    options: q.options.map((o) => `${o.key}. ${o.text}`),
    answer: q.answer_keys,
    given: ["B"],
    explanation: "交卷时的旧解析",
    category: "1.1.1",
    ...overrides,
  };
}

function reviewOf(...items: ReturnType<typeof reviewItem>[]) {
  return { bank: "A", finished_at_ms: 1_700_000_000_000, items };
}

test("考后复盘：解析读题库现在的，不用交卷时存下的旧文字", async ({ page }) => {
  const q = bankQuestion(0);
  await page.addInitScript((data) => {
    localStorage.setItem("exam:last-review", JSON.stringify(data));
  }, reviewOf(reviewItem(q)));

  await page.goto("/exam-review");
  // 解析卡首次展开前不渲染正文：先展开，再等题库异步加载完、现在的解析换上来
  await expandExplanations(page);
  await expect(page.locator("main")).toContainText(explanationHead(q));
  await expect(page.getByText("交卷时的旧解析")).toHaveCount(0);
});

test("考后复盘：答案与题库对不上的题，沿用交卷时存下的解析", async ({ page }) => {
  const mismatched = bankQuestion(0);
  const control = bankQuestion(1);
  // 编号、题干都一致，但存档里的答案与题库现在的不同（题库勘误过）：
  // 把针对新答案的解析安给旧答案会自相矛盾，所以不换
  const staleAnswer = mismatched.answer_keys.length === 1 ? ["Z"] : mismatched.answer_keys.slice(1);
  await page.addInitScript((data) => {
    localStorage.setItem("exam:last-review", JSON.stringify(data));
  }, reviewOf(reviewItem(mismatched, { answer: staleAnswer }), reviewItem(control)));

  await page.goto("/exam-review");
  await expandExplanations(page);
  // 对照题与题库完全一致：它的解析换上来，就说明题库索引已经到了
  await expect(page.locator("main")).toContainText(explanationHead(control));
  // 此时答案对不上的那题仍是存档里的，没有被换成题库的
  await expect(page.getByText("交卷时的旧解析")).toHaveCount(1);
  await expect(page.locator("main")).not.toContainText(explanationHead(mismatched));
});

test("考后复盘：分类表现按错得多的在前排序", async ({ page }) => {
  await page.addInitScript((data) => {
    localStorage.setItem("exam:last-review", JSON.stringify(data));
  }, REVIEW);

  await page.goto("/exam-review");
  const items = page.locator("li").filter({ hasText: /错 \d/ });
  // 题库是异步加载的，等分类列表渲染出来再取文本，否则会读到空列表
  await expect(items).toHaveCount(2);
  const codes = await items.allInnerTexts();
  // 2.2.2 错了 2 题，1.1.1 错了 0 题
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

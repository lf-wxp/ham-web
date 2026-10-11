import { expect, test } from "./fixtures";

test("P3 知识溯源：频谱管理页与法规页带「依据 · 核对」标注", async ({ page }) => {
  await page.goto("/spectrum-admin");
  await expect(page.getByRole("heading", { name: "干扰查处流程" })).toBeVisible();
  // 流程里引用的条例条文存在（核心有单测逐条校验），页面照录。
  await expect(page.getByText(/第五十六条/).first()).toBeVisible();
  await expect(page.getByText(/依据 .+，2026-10 核对/).first()).toBeVisible();

  await page.goto("/regulations");
  await expect(page.getByText(/依据 .+，2026-10 核对/).first()).toBeVisible();
});

test("P3 天线理论：公式由 KaTeX 渲染，Unicode 表达式作回退", async ({ page }) => {
  await page.goto("/antenna-theory");
  await expect(page.getByRole("heading", { level: 1, name: "天线理论" })).toBeVisible();
  // KaTeX 渲染结果以 .katex 容器出现（资源自托管，离线可用）。
  await expect(page.locator(".katex").first()).toBeVisible();
  // 远场边界公式的 tex 源确实在节点上。
  await expect(page.locator('[data-tex*="2D"]').first()).toBeVisible();
});

test("P3 多国题库对照：英国 Full 是 58 题（官方考纲数字）", async ({ page }) => {
  await page.goto("/exam-comparison");
  await expect(page.getByText("58 题 / 2 小时，60%（35 题）通过")).toBeVisible();
  await expect(page.getByText("35 题，答对 26 题（74%）通过").first()).toBeVisible();
  await expect(page.getByText("50 题，答对 37 题（74%）通过")).toBeVisible();
  await expect(page.getByRole("link", { name: /NCVEC/ })).toBeVisible();
});

test("P3 学习路径：答错自动回推前置专题", async ({ page }) => {
  // 直接播种错题本（key 放占位，加载时 rekey 重算）：比走闪卡流程更稳，也不依赖题库顺序。
  await page.addInitScript(() => {
    window.localStorage.setItem(
      "mistake-book",
      JSON.stringify({
        records: [
          {
            key: "0000000000000000",
            question: {
              id: "A-1",
              codes: { J: "LY0001", P: "1.1.1" },
              question: "种子错题",
              options: [
                { key: "A", text: "a" },
                { key: "B", text: "b" },
                { key: "C", text: "c" },
                { key: "D", text: "d" },
              ],
              answer_keys: ["A"],
              type: "single",
              explanation: null,
            },
            my_answer: ["B"],
            wrong_count: 1,
            streak: 0,
            last_wrong_ms: 1,
            due_ms: 1,
          },
        ],
      }),
    );
  });
  await page.goto("/learning-path");
  await expect(page.getByText("按你的错题本回推：先复习这些")).toBeVisible();
  // 1.1.1 映射到「法规与管理」节点，它没有前置 → 回推链就是它自己。
  await expect(
    page.locator('a[href="/regulations"]').filter({ hasText: "法规与管理" }).first(),
  ).toBeVisible();
});

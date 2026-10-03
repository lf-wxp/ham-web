import type { Page } from "@playwright/test";
import { expect, option, test } from "./fixtures";

/** 底部操作栏统计行（桌面 / 移动端各渲染一份），形如「已作答 3 / 40｜标记 1」。 */
function stats(page: Page) {
  return page.getByText(/已作答\s*\d+\s*\/\s*\d+/).first();
}

/** 答题卡里的题号按钮（网格内的数字按钮）。 */
function gridButtons(card: ReturnType<Page["getByRole"]>) {
  return card.locator("div.grid button");
}

test("模拟考试：A / B 类按各自规则初始化题量、限时与倒计时", async ({ page }) => {
  await page.goto("/exam?bank=A");
  await expect(page.getByText(/考试类别：/).first()).toBeVisible();
  await expect(page.getByText("试题数：40（单选 32，多选 8）").first()).toBeVisible();
  await expect(page.getByText("限时：40 分钟").first()).toBeVisible();
  await expect(page.getByText(/剩余时间：/).first()).toBeVisible();
  await expect(page.locator("main").getByText(/^\d{2}:\d{2}$/).first()).toBeVisible();
  await expect(stats(page)).toContainText("0 / 40");

  await page.goto("/exam?bank=B");
  await expect(page.getByText("试题数：60（单选 45，多选 15）").first()).toBeVisible();
  await expect(page.getByText("限时：60 分钟").first()).toBeVisible();
  // 60 分钟首秒为 HH:MM:SS，之后回落到 MM:SS
  await expect(page.locator("main").getByText(/^\d{2}:\d{2}(:\d{2})?$/).first()).toBeVisible();
});

test("模拟考试：标记与答题卡联动，可按未答 / 标记筛选并跳转", async ({ page }) => {
  await page.goto("/exam");
  await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();

  // 标记第 1 题
  await page.getByRole("button", { name: "标记", exact: true }).first().click();
  await expect(page.getByRole("button", { name: "取消标记", exact: true }).first()).toBeVisible();
  await expect(stats(page)).toContainText("标记 1");

  // 答一题：已作答 +1
  await option(page, "A").click();
  await expect(stats(page)).toContainText("1 / 40");

  await page.getByRole("button", { name: "答题卡" }).first().click();
  const card = page.getByRole("dialog", { name: "答题卡" });
  await expect(card).toBeVisible();
  await expect(gridButtons(card)).toHaveCount(40);

  // 筛选：未答 39 题
  await card.getByRole("button", { name: "未答", exact: true }).click();
  await expect(gridButtons(card)).toHaveCount(39);

  // 筛选：标记 1 题（就是第 1 题）
  await card.getByRole("button", { name: "标记", exact: true }).click();
  await expect(gridButtons(card)).toHaveCount(1);
  await expect(gridButtons(card).first()).toHaveText("1");

  // 「下一个未答」会跳到第一个还没作答的题
  await card.getByRole("button", { name: "全部" }).click();
  await card.getByRole("button", { name: "下一个未答" }).click();
  await expect(card).toBeHidden();
  await expect(page.getByText(/第 2 \/ 40 题/).first()).toBeVisible();

  // 回到全部，跳到第 10 题
  await page.getByRole("button", { name: "答题卡" }).first().click();
  await card.getByRole("button", { name: "全部" }).click();
  await gridButtons(card).nth(9).click();
  await expect(card).toBeHidden();
  await expect(page.getByText(/第 10 \/ 40 题/).first()).toBeVisible();
});

test("模拟考试：交卷确认弹窗列出未作答数量", async ({ page }) => {
  await page.goto("/exam");
  await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
  await option(page, "A").click();

  await page.getByRole("button", { name: "交卷", exact: true }).first().click();
  const confirm = page.getByRole("dialog", { name: "确认交卷？" });
  await expect(confirm).toBeVisible();
  await expect(confirm.getByText(/已作答/).first()).toBeVisible();
  await expect(confirm.getByText(/未作答/).first()).toBeVisible();

  // 取消后回到答题，未真正交卷
  await confirm.getByRole("button", { name: "取消" }).click();
  await expect(confirm).toBeHidden();
  await expect(page.getByRole("button", { name: "交卷", exact: true }).first()).toBeVisible();
  await expect(stats(page)).toContainText("1 / 40");
});

test("模拟考试：交卷后答题卡标注对错，底部栏锁定为已交卷", async ({ page }) => {
  await page.goto("/exam");
  await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
  await page.getByRole("button", { name: "交卷", exact: true }).first().click();
  await page
    .getByRole("dialog", { name: "确认交卷？" })
    .getByRole("button", { name: "确认交卷" })
    .click();
  const result = page.getByRole("dialog", { name: "成绩" });
  await expect(result).toBeVisible();
  await result.getByRole("button", { name: "继续浏览题目" }).click();

  await expect(page.getByRole("button", { name: "已交卷" }).first()).toBeDisabled();

  await page.getByRole("button", { name: "答题卡" }).first().click();
  const card = page.getByRole("dialog", { name: "答题卡" });
  await expect(card).toBeVisible();
  // 交卷后出现对错图例
  await expect(card.getByText("正确")).toBeVisible();
  await expect(card.getByText("错误")).toBeVisible();
  // 每题都有 ✓ / ✗ 角标
  await expect(card.locator('[data-slot="badge"]')).toHaveCount(40);
});

test("模拟考试：中途刷新可恢复，也可重新开始", async ({ page }) => {
  await page.goto("/exam");
  await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
  await option(page, "A").click();
  await expect(stats(page)).toContainText("1 / 40");

  await page.reload();
  const resume = page.getByRole("dialog", { name: "恢复考试" });
  await expect(resume).toBeVisible();
  await resume.getByRole("button", { name: "继续考试" }).click();
  await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
  await expect(stats(page)).toContainText("1 / 40");

  // 重新开始会清掉断点并重抽一套
  await page.reload();
  await expect(page.getByRole("dialog", { name: "恢复考试" })).toBeVisible();
  await page.getByRole("dialog", { name: "恢复考试" }).getByRole("button", { name: "重新开始" }).click();
  await expect(stats(page)).toContainText("0 / 40");
});

test("模拟考试：顶部可在常规模考与薄弱项组卷之间切换", async ({ page }) => {
  await page.goto("/exam");
  await expect(page.getByRole("heading", { level: 1, name: "模拟考试" })).toBeAttached();
  await page.getByRole("link", { name: "薄弱项组卷" }).first().click();
  await expect(page).toHaveURL(/mode=weak/);
  await expect(page.getByRole("heading", { level: 1, name: "薄弱项组卷" })).toBeAttached();

  await page.getByRole("link", { name: "常规模考" }).first().click();
  await expect(page).not.toHaveURL(/mode=weak/);
});

test("模拟考试：自定义组卷按指定配额出卷", async ({ page }) => {
  await page.goto("/exam?mode=custom");
  const dialog = page.getByRole("dialog", { name: "自定义组卷" });
  await expect(dialog).toBeVisible();
  // 三个数字输入依次为：单选题数、多选题数、限时（分钟）
  const numbers = dialog.locator('input[type="number"]');
  await numbers.nth(0).fill("2");
  await numbers.nth(1).fill("1");
  await dialog.getByRole("button", { name: "开始考试" }).click();
  await expect(dialog).toBeHidden();

  await expect(page.getByText(/第 1 \/ 3 题/).first()).toBeVisible();
  await expect(page.getByText(/试题数：3（单选 2，多选 1）/).first()).toBeVisible();
});

test("自定义组卷：多选题可逐项勾选与取消", async ({ page }) => {
  await page.goto("/exam?mode=custom");
  const dialog = page.getByRole("dialog", { name: "自定义组卷" });
  await expect(dialog).toBeVisible();
  const numbers = dialog.locator('input[type="number"]');
  await numbers.nth(0).fill("0");
  await numbers.nth(1).fill("2");
  await dialog.getByRole("button", { name: "开始考试" }).click();

  await expect(page.getByText(/第 1 \/ 2 题/).first()).toBeVisible();
  await expect(page.getByText("多选", { exact: true }).first()).toBeVisible();

  const a = option(page, "A");
  const b = option(page, "B");
  await a.click();
  await expect(a).toHaveAttribute("aria-checked", "true");
  await b.click();
  await expect(a).toHaveAttribute("aria-checked", "true");
  await expect(b).toHaveAttribute("aria-checked", "true");
  // 再点一次取消
  await a.click();
  await expect(a).toHaveAttribute("aria-checked", "false");
  await expect(b).toHaveAttribute("aria-checked", "true");
  await expect(stats(page)).toContainText("1 / 2");
});

import { clickClear, expect, futureIso, pickDate, pickTime, test } from "./fixtures";

test("倒计时：添加、持久化与删除", async ({ page }) => {
  await page.goto("/countdown");
  await expect(page.getByRole("heading", { level: 1, name: "倒计时与提醒" })).toBeVisible();
  await expect(page.getByText("暂无倒计时，添加一个目标时间吧。")).toBeVisible();

  await page.getByLabel("倒计时标题").fill("A 类操作证考试");
  await pickDate(page, "目标日期", futureIso());
  await pickTime(page, "目标时间", "09:00");
  await clickClear(page.getByRole("button", { name: "添加" }));

  await expect(page.getByText("A 类操作证考试")).toBeVisible();
  await expect(page.getByText("暂无倒计时，添加一个目标时间吧。")).toHaveCount(0);
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("countdowns")))
    .toContain("A 类操作证考试");

  await page.getByRole("button", { name: "删除" }).first().click();
  await expect(page.getByText("暂无倒计时，添加一个目标时间吧。")).toBeVisible();
});

test("倒计时：标题为空时静默失败，不产生条目", async ({ page }) => {
  await page.goto("/countdown");
  await expect(page.getByRole("heading", { level: 1, name: "倒计时与提醒" })).toBeVisible();

  await pickDate(page, "目标日期", futureIso());
  await pickTime(page, "目标时间", "09:00");
  await clickClear(page.getByRole("button", { name: "添加" }));
  await expect(page.getByText("暂无倒计时，添加一个目标时间吧。")).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem("countdowns"))).toBeNull();
});

test("倒计时：已过期的条目显示已到期", async ({ page }) => {
  await page.goto("/countdown");
  await expect(page.getByRole("heading", { level: 1, name: "倒计时与提醒" })).toBeVisible();

  // 目标时间取「上个月 1 号」而不是固定年份：判据只要求它在**过去**，而 `pickDate`
  // 是逐月点击的 —— 写 2020 年要往回翻 80 多个月、光点击就十几秒，全量并行下会超时。
  const now = new Date();
  const past = new Date(now.getFullYear(), now.getMonth() - 1, 1);
  const iso = `${past.getFullYear()}-${String(past.getMonth() + 1).padStart(2, "0")}-01`;

  await page.getByLabel("倒计时标题").fill("过去的考试");
  await pickDate(page, "目标日期", iso);
  await pickTime(page, "目标时间", "09:00");
  await clickClear(page.getByRole("button", { name: "添加" }));

  await expect(page.getByText("过去的考试")).toBeVisible();
  await expect(page.getByText("已到期")).toBeVisible();
});

test("学习进度：设定考试日期后生成备考计划并持久化", async ({ page }) => {
  await page.goto("/progress");
  await expect(page.getByRole("heading", { level: 1, name: "学习进度" })).toBeVisible();
  await expect(page.getByRole("heading", { level: 2, name: "备考计划" })).toBeVisible();
  await expect(
    page.getByText("设定考试日期后，会按还没做过的题量、待复习错题和临考阶段"),
  ).toBeVisible();

  const examDate = futureIso();
  await pickDate(page, "考试日期", examDate);
  await expect(page.getByText(/距 A 类考试还有 \d+ 天/)).toBeVisible();
  // 任务清单至少包含新题与复习两类
  await expect(page.getByText(/做新题 \d+ \/ \d+/)).toBeVisible();
  await expect(page.getByRole("link", { name: "去做" })).toBeVisible();

  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("learning-plan")))
    .toContain(examDate);
});

test("学习进度：切换报考类别即时生效", async ({ page }) => {
  await page.goto("/progress");
  await expect(page.getByRole("heading", { level: 2, name: "备考计划" })).toBeVisible();
  await pickDate(page, "考试日期", futureIso());

  await page.getByRole("button", { name: "B 类" }).first().click();
  await expect(page.getByText(/距 B 类考试还有 \d+ 天/)).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("learning-plan")))
    .toContain("B");
});

test("学习进度：空数据时各区块给出引导而非空白", async ({ page }) => {
  await page.goto("/progress");
  await expect(page.getByRole("heading", { level: 1, name: "学习进度" })).toBeVisible();
  await expect(page.getByText("还没有 A 类模拟考试记录。")).toBeVisible();
  await expect(
    page.getByText("完成练习或模拟考试后，这里会按分类展示正确率，帮你定位薄弱知识点。"),
  ).toBeVisible();
  await expect(page.getByRole("heading", { level: 2, name: "题库覆盖率" })).toBeVisible();
});

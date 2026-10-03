import { expect, test } from "./fixtures";

test("闪卡刷题：显示答案后自评，不会的进入错题本", async ({ page }) => {
  await page.goto("/flashcards");
  await expect(page.getByRole("heading", { level: 1, name: "闪卡刷题" })).toBeVisible();
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();

  await page.getByRole("button", { name: "显示答案" }).click();
  await expect(page.getByRole("button", { name: "会 ✓" })).toBeVisible();

  await page.getByRole("button", { name: "不会 ✗" }).click();
  await expect(page.getByText(/第 2 \/ \d+ 题/).first()).toBeVisible();
  await expect(page.getByText(/^会 0 · 不会 1/)).toBeVisible();

  // 自评「不会」会写入错题本
  await page.goto("/mistakes");
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();
  await expect(page.getByText(/共 1 道错题/)).toBeVisible();
});

test("闪卡刷题：跳过不计分", async ({ page }) => {
  await page.goto("/flashcards");
  await expect(page.getByRole("heading", { level: 1, name: "闪卡刷题" })).toBeVisible();
  await page.getByRole("button", { name: "跳过" }).click();
  await expect(page.getByText(/第 2 \/ \d+ 题/).first()).toBeVisible();
  await expect(page.getByText(/^会 0 · 不会 0/)).toBeVisible();
});

test("每日挑战：开始 → 交卷 → 计入打卡与成绩", async ({ page }) => {
  await page.goto("/daily-challenge");
  await expect(page.getByRole("heading", { level: 1, name: "每日挑战" })).toBeVisible();
  await expect(page.getByText("10 题 · 限时 10 分钟 · 交卷后计入打卡")).toBeVisible();

  await page.getByRole("button", { name: "开始挑战" }).click();
  await expect(page.getByText("剩余时间")).toBeVisible();
  await expect(page.getByRole("button", { name: "交卷" })).toBeVisible();

  await page.getByRole("button", { name: "交卷" }).click();
  await expect(page.getByText("挑战完成")).toBeVisible();
  await expect(page.getByText(/答对 \d+ \/ 10（\d+%）/)).toBeVisible();

  // 成绩与分类统计都落盘（10 题未作答按答错计入）
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("daily-challenge")))
    .toBeTruthy();
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("study-stats")))
    .toBeTruthy();
  const saved = await page.evaluate(() =>
    JSON.parse(localStorage.getItem("daily-challenge") ?? "{}"),
  );
  const today = Object.keys(saved.days ?? {});
  expect(today).toHaveLength(1);
  expect(saved.days[today[0]]).toMatchObject({ total: 10 });
});

test("每日挑战：同一天同一组题，可查看题目后再次挑战", async ({ page }) => {
  await page.goto("/daily-challenge?bank=A");
  await expect(page.getByRole("heading", { level: 1, name: "每日挑战" })).toBeVisible();
  await page.getByRole("button", { name: "开始挑战" }).click();
  await page.getByRole("button", { name: "交卷" }).click();
  await expect(page.getByText("挑战完成")).toBeVisible();

  await page.getByRole("button", { name: "查看题目" }).click();
  await expect(page.getByText("今日挑战")).toBeVisible();
  await expect(page.getByText(/今日已完成：答对 \d+ \/ 10/)).toBeVisible();
  await expect(page.getByRole("button", { name: "再挑战一次" })).toBeVisible();
});

test("打印版：空数据与收藏集各自的空态", async ({ page }) => {
  await page.goto("/print");
  await expect(page.getByRole("heading", { level: 1, name: "打印版" })).toBeVisible();
  await expect(page.getByText("错题集为空，先去练习积累一些题目吧。")).toBeVisible();

  await page.getByRole("button", { name: "收藏集" }).click();
  await expect(page.getByText("收藏集为空，先去练习积累一些题目吧。")).toBeVisible();
});

test("打印版：收藏一题后可打印，答案模式可切换", async ({ page }) => {
  // 先收藏一题
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await page.getByRole("button", { name: "收藏" }).first().click();

  await page.addInitScript(() => {
    (window as unknown as { print: () => void }).print = () => {};
  });
  await page.goto("/print?src=bookmarks");
  await expect(page.getByRole("heading", { level: 1, name: "打印版" })).toBeVisible();
  await expect(page.getByText(/共 1 题/)).toBeVisible();

  // 默认答案集中在末尾
  await expect(page.getByRole("heading", { level: 2, name: "参考答案" })).toBeVisible();

  await page.getByLabel("答案").selectOption("inline");
  await expect(page.getByText(/答案：/).first()).toBeVisible();

  // 不显示答案时解析勾选被禁用
  await page.getByLabel("答案").selectOption("hidden");
  await expect(page.getByText(/答案：/)).toHaveCount(0);
});

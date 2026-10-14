import { expect, test } from "./fixtures";

/** 日历类页面都按「本地今天」排，时钟固定到东八区 2026-10-14 上午（配置里 timezoneId = Asia/Shanghai）。 */
const NOW = new Date("2026-10-14T10:00:00+08:00");
const DAY = 86_400_000;

test("备考日历：14 天里标出考试日、倒数、模拟考建议、打卡与到期错题", async ({ page }) => {
  await page.clock.setFixedTime(NOW);
  const now = NOW.getTime();
  await page.addInitScript((now) => {
    localStorage.setItem("mistake-book:seeded", "1");
    localStorage.setItem("study-stats:seen-seeded", "1");
    localStorage.setItem(
      "learning-plan",
      JSON.stringify({ target_date: "2026-10-20", bank: "A", reminder_time: "" }),
    );
    localStorage.setItem(
      "study-daily",
      JSON.stringify({ days: { "2026-10-14": { answered: 12, new: 3, duration_ms: 600000 } } }),
    );
    const record = (key: string, due: number) => ({
      key,
      question: {
        id: key,
        // 题面要各不相同：错题本按题面指纹归并，同题面的两条记录会合成一条。
        question: `种子题 ${key}`,
        options: [{ key: "A", text: "选项 A" }],
        answer_keys: ["A"],
        type: "single",
      },
      my_answer: [],
      wrong_count: 1,
      streak: 0,
      last_wrong_ms: now - 86_400_000,
      due_ms: due,
      banks: ["A"],
      ease: 2.5,
      interval_days: 1,
      last_review_ms: now - 86_400_000,
      cause: null,
    });
    localStorage.setItem(
      "mistake-book",
      JSON.stringify({
        records: [
          // 已过期 → 计入今天；1.5 天后到期 → 落在后天（10-16）。
          record("A-1", now - 1000),
          record("A-2", now - 5000),
          record("A-3", now + 1.5 * 86_400_000),
        ],
      }),
    );
  }, now);
  await page.goto("/study-calendar");

  // 顶部备考计划卡里也有列表：限定在「未来 14 天」那一节。
  const days = page
    .locator("section", { has: page.getByRole("heading", { name: "未来 14 天复习日历" }) })
    .locator("ul > li");
  await expect(days).toHaveCount(14);
  const row = (mmdd: string) => days.filter({ hasText: mmdd });

  const today = row("10-14");
  await expect(today).toContainText("今天");
  await expect(today).toContainText("距考试 6 天");
  await expect(today).toContainText("建议模拟考");
  await expect(today).toContainText("已打卡");
  await expect(today).toContainText("2 道");
  await expect(row("10-16")).toContainText("1 道");
  await expect(row("10-15")).toContainText("—");
  await expect(row("10-15")).not.toContainText("已打卡");

  await expect(row("10-20")).toContainText("考试日");
  await expect(row("10-20")).not.toContainText("建议模拟考");
  // 考前 7 天内都建议模拟考，考后一律「已过」。
  await expect(days.filter({ hasText: "建议模拟考" })).toHaveCount(6);
  await expect(days.filter({ hasText: "已过" })).toHaveCount(7);
});

test("竞赛日历：按下一届开始时间排序，加入倒计时，并链到竞赛录入", async ({ page }) => {
  await page.clock.setFixedTime(NOW);
  const alerts: string[] = [];
  page.on("dialog", (d) => {
    alerts.push(d.message());
    void d.accept();
  });
  await page.goto("/contest-calendar");

  const dates = page.getByText(/^\d{4} 年 \d{1,2} 月 \d{1,2} 日 · /);
  await expect(dates).toHaveCount(18);
  const starts = (await dates.allInnerTexts()).map((s) => {
    const [, y, m, d] = s.match(/^(\d{4}) 年 (\d{1,2}) 月 (\d{1,2}) 日/)!;
    return Date.UTC(Number(y), Number(m) - 1, Number(d));
  });
  // 「下一届」：都不早于今天，且升序排列。
  expect(Math.min(...starts)).toBeGreaterThanOrEqual(Date.UTC(2026, 9, 14) - DAY);
  expect([...starts].sort((a, b) => a - b)).toEqual(starts);

  // 有竞赛 id 的条目都能直接开一场竞赛录入。
  const open = page.getByRole("link", { name: "开新场次" });
  await expect(open).toHaveCount(14);
  for (const href of await open.evaluateAll((els) => els.map((e) => e.getAttribute("href")))) {
    expect(href).toMatch(/^\/contest-log\?contest=[A-Z0-9-]+$/);
  }

  // 加入提醒：写进倒计时列表（标题带「竞赛：」前缀），再去 /countdown 能看到。
  await page.getByRole("button", { name: "加入提醒" }).first().click();
  await expect.poll(() => alerts.join("\n")).toMatch(/^已把「.+」加入倒计时提醒$/);
  const name = alerts[0].match(/已把「(.+)」/)![1];
  const stored = await page.evaluate(() => localStorage.getItem("countdowns") ?? "");
  expect(stored).toContain(`竞赛：${name}`);
  await page.getByRole("link", { name: "我的倒计时" }).click();
  await expect(page).toHaveURL(/\/countdown$/);
  await expect(page.getByText(`竞赛：${name}`)).toBeVisible();
});

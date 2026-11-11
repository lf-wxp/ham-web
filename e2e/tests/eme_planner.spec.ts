import { expect, pickOption, test } from "./fixtures";

/**
 * EME 计划器（`/eme`）：月球位置、回波多普勒、月相与窗口。
 *
 * 期望值全部来自 **JPL Horizons**（2026-11-10 取回，`COMMAND=301`，观测者表，无折射），不是本站代码的输出：
 * 站点取网格中心 —— `OM89` = 39.5°N 117.0°E、`QF56` = 33.5°S 151.0°E，海拔 0；
 * 时钟固定在 2026-11-10T00:00:00Z，表格从此刻起每隔 1 小时一行。
 * 窗口边界来自 Horizons 1 分钟步长的仰角序列（≥ 5°）。
 */

const NOW = new Date("2026-11-10T00:00:00Z");

/** `[第几小时, 方位°, 仰角°, 站心月距 km, 距离变化率 km/s]`：OM89，月球在地平线之上的 10 个时刻（此刻起 0–9 小时后）。 */
const HORIZONS_OM89: [number, number, number, number, number][] = [
  [0, 123.52, 0.81, 399623, -0.2547],
  [1, 133.6, 9.48, 398776, -0.2133],
  [2, 145.04, 16.7, 398107, -0.156],
  [3, 157.96, 21.97, 397667, -0.0865],
  [4, 172.11, 24.76, 397493, -0.0093],
  [5, 186.74, 24.76, 397603, 0.0705],
  [6, 200.85, 21.95, 397997, 0.1476],
  [7, 213.72, 16.68, 398656, 0.217],
  [8, 225.06, 9.46, 399545, 0.2742],
  [9, 235.0, 0.79, 400611, 0.3155],
];

const C_KM_S = 299792.458;

/** 回波多普勒（Hz）：`−2·f·ṙ/c`。 */
const echo = (mhz: number, rate: number) => (-2 * mhz * 1e6 * rate) / C_KM_S;

/** "11-10 08:31" → 当天的分钟序号（0–1439）；跨日的窗口用不到。 */
const minuteOfDay = (stamp: string) => {
  const m = /(\d\d):(\d\d)/.exec(stamp);
  if (!m) throw new Error(`读不出时间：${stamp}`);
  return Number(m[1]) * 60 + Number(m[2]);
};

async function openPlanner(page: import("@playwright/test").Page, at: Date = NOW) {
  await page.clock.setFixedTime(at);
  await page.goto("/eme");
  const planner = page.locator("section", { has: page.getByRole("heading", { name: "月球位置与通联窗口" }) });
  await expect(planner).toBeVisible();
  await planner.getByLabel("本台网格").fill("OM89");
  return planner;
}

test("EME 计划器：此刻的方位 / 仰角 / 月距 / 回波多普勒 / 月相与 Horizons 一致", async ({ page }) => {
  const planner = await openPlanner(page);
  const status = planner.getByRole("status");
  await expect(status).toContainText("此刻（UTC 11-10 00:00）");

  const [, az, el, range, rate] = HORIZONS_OM89[0];
  const text = (await status.innerText()).replace(/\s+/g, " ");
  const pos = /方位 ([\d.]+)°\s*仰角 (-?[\d.]+)°\s*站心月距 (\d+) km/.exec(text);
  expect(pos, text).not.toBeNull();
  expect(Math.abs(Number(pos![1]) - az)).toBeLessThan(0.25);
  expect(Math.abs(Number(pos![2]) - el)).toBeLessThan(0.25);
  expect(Math.abs(Number(pos![3]) - range) / range).toBeLessThan(0.003);

  // 默认 144 MHz：自己的回波多普勒 = −2·f·ṙ/c ≈ +244.7 Hz（月球正在靠近，回波升高）。
  const dop = /144 MHz）：([+-]\d+) Hz（月距变化率 ([+-][\d.]+) km\/s/.exec(text);
  expect(dop, text).not.toBeNull();
  expect(Math.abs(Number(dop![1]) - echo(144, rate))).toBeLessThan(6);
  expect(Math.abs(Number(dop![2]) - rate)).toBeLessThan(0.004);

  // 路径损耗相对平均月距：399 623 km → 40·log10(399623/384400) = +0.67 dB。
  await expect(status).toContainText(/\+0\.6\d dB/);
  // 刚过朔（Horizons 受照 0.82%，且在增加）：渐盈。
  await expect(status).toContainText(/受照 [01]%，渐盈/);
  // 00:00 时仰角只有 0.8°，低于默认的最低仰角 5°：现在还不是窗口；把门槛降到 0° 就高于了。
  await expect(status).toContainText("月球低于最低仰角，现在不是窗口");
  await planner.getByLabel("最低仰角 (°)").fill("0");
  await expect(status).toContainText("月球高于最低仰角");
});

test("EME 计划器：未来 24 小时表只列月球在地平线之上的时刻，每一行都对得上 Horizons", async ({ page }) => {
  const planner = await openPlanner(page);
  const table = planner.getByRole("table", { name: "未来 24 小时每隔 1 小时、月球在地平线之上的时刻" });
  const rows = table.locator("tbody tr");
  await expect(rows).toHaveCount(HORIZONS_OM89.length);

  for (const [hour, az, el, range, rate] of HORIZONS_OM89) {
    const cells = await rows.nth(hour).locator("td").allTextContents();
    expect(cells[0]).toBe(`11-10 ${String(hour).padStart(2, "0")}:00`);
    expect(Math.abs(Number(cells[1]) - az), `${hour} 时方位`).toBeLessThan(0.25);
    expect(Math.abs(Number(cells[2]) - el), `${hour} 时仰角`).toBeLessThan(0.25);
    expect(Math.abs(Number(cells[3]) - range) / range, `${hour} 时月距`).toBeLessThan(0.003);
    expect(Math.abs(Number(cells[5]) - echo(144, rate)), `${hour} 时多普勒`).toBeLessThan(6);
  }

  // 换到 1296 MHz：多普勒按频率放大 9 倍（同一时刻的月距变化率不变）。
  const before = Number((await rows.nth(1).locator("td").nth(5).innerText()).trim());
  await pickOption(page, "EME 频段", "1296 MHz");
  await expect
    .poll(async () => Number((await rows.nth(1).locator("td").nth(5).innerText()).trim()))
    .toBeCloseTo(before * 9, -1);
  await expect(planner.getByRole("status")).toContainText("1296 MHz）");
});

test("EME 计划器：本台窗口与共同窗口的边界对得上 Horizons 的 1 分钟仰角序列", async ({ page }) => {
  const planner = await openPlanner(page);

  // Horizons：OM89 在 00:28–08:31（≥ 5°）出现一个窗口，最高约 24.8°。
  const mine = planner.locator('[data-slot="eme-windows-mine"]');
  await expect(mine.locator("li")).toHaveCount(1);
  const mineText = await mine.locator("li").innerText();
  const [start, end] = [...mineText.matchAll(/\d\d-\d\d (\d\d:\d\d)/g)].map((m) => minuteOfDay(m[1]));
  expect(Math.abs(start - 28)).toBeLessThanOrEqual(2);
  expect(Math.abs(end - 511)).toBeLessThanOrEqual(2);
  expect(mineText).toMatch(/最高 2[45]°/);

  // 没填对方网格就没有共同窗口这一块。
  await expect(planner.locator('[data-slot="eme-windows-mutual"]')).toHaveCount(0);

  // 对方 QF56（悉尼一带）：Horizons 两端同时 ≥ 5° 的窗口恰好与 OM89 自己的窗口重合（00:28–08:31）。
  await planner.getByLabel("对方网格（可选）").fill("QF56");
  const mutual = planner.locator('[data-slot="eme-windows-mutual"]');
  await expect(mutual.locator("li")).toHaveCount(1);
  const mutualText = await mutual.locator("li").innerText();
  const [ms, me] = [...mutualText.matchAll(/\d\d-\d\d (\d\d:\d\d)/g)].map((m) => minuteOfDay(m[1]));
  expect(Math.abs(ms - 28)).toBeLessThanOrEqual(2);
  expect(Math.abs(me - 511)).toBeLessThanOrEqual(2);
  // 窗口是月球自己的升落，不是被查询边界截断的：不带截断说明。
  expect(mineText).not.toContain("窗口开始得更早");
  expect(mineText).not.toContain("窗口结束得更晚");

  // 最低仰角抬到 60°：月球最高才 25°，窗口消失，如实说没有。
  await planner.getByLabel("最低仰角 (°)").fill("60");
  await expect(planner.locator('[data-slot="eme-windows-mine"]')).toContainText("24 小时内没有");
  await expect(planner.getByRole("status")).toContainText("月球低于最低仰角");
});

test("EME 计划器：被查询边界截断的窗口如实标出起点 / 终点", async ({ page }) => {
  // 此刻 04:00：OM89 的月球 00:28 就升起、08:31 才落下 —— 第一个窗口的起点只是「此刻」；
  // 次日约 01:20 再升起，24 小时后（次日 04:00）还没落下 —— 第二个窗口的终点只是查询边界。
  const planner = await openPlanner(page, new Date("2026-11-10T04:00:00Z"));
  const items = planner.locator('[data-slot="eme-windows-mine"] li');
  await expect(items).toHaveCount(2);
  await expect(items.first()).toContainText("11-10 04:00");
  await expect(items.first()).toContainText("窗口开始得更早");
  await expect(items.first()).not.toContainText("窗口结束得更晚");
  await expect(items.last()).toContainText("窗口结束得更晚");
  await expect(items.last()).not.toContainText("窗口开始得更早");
});

test("EME 计划器：缺网格 / 对方网格不合法都不硬算，给出原因", async ({ page }) => {
  const planner = await openPlanner(page);

  await planner.getByLabel("对方网格（可选）").fill("ZZ99");
  await expect(planner).toContainText("对方网格不合法，暂不算共同窗口");
  await expect(planner.locator('[data-slot="eme-windows-mutual"]')).toHaveCount(0);
  // 本台那一块不受影响。
  await expect(planner.locator('[data-slot="eme-windows-mine"] li')).toHaveCount(1);

  await planner.getByLabel("本台网格").fill("");
  await expect(planner.getByRole("status")).toContainText("先填一个合法的本台网格");
  await expect(planner.getByRole("table")).toHaveCount(0);
});

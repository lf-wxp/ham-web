import { expect, seedLog, test } from "./fixtures";

/**
 * 呼号查询的「三合一」：我打过吗 / 什么方向 / 多远。
 * 三个答案都来自站内数据（日志、本台网格、DXCC 实体中心），不需要联网。
 */

/** 写入本台网格（`/callsign` 只在挂载时读一次）。 */
async function seedStation(page: import("@playwright/test").Page, grid: string) {
  await page.addInitScript((g) => {
    localStorage.setItem(
      "station-info",
      JSON.stringify({ callsign: "BG4XXX", operator: "", gridsquare: g, rig: "", antenna: "" }),
    );
  }, grid);
}

test("呼号查询：给出我的通联次数、最近一次、方位与距离", async ({ page }) => {
  await seedStation(page, "OM89");
  await seedLog(page, [
    { callsign: "JA1ABC", date: "2025-03-01", time: "10:00", freq: "14.025", mode: "CW" },
    { callsign: "JA1ABC", date: "2025-06-01", time: "22:30", freq: "7.025", mode: "CW" },
    { callsign: "K1ABC", date: "2025-07-01", time: "01:00", freq: "21.025", mode: "CW" },
  ]);
  await page.goto("/callsign?call=JA1ABC");

  await expect(page.getByText("我的通联记录")).toBeVisible();
  await expect(page.getByText("2 次 · 最近 2025-06-01 22:30 40m")).toBeVisible();

  // OM89 → 日本：方位在东侧、距离两千公里量级（具体值由 core 单测钉死）。
  await expect(page.getByText("方位（从本台）")).toBeVisible();
  await expect(page.getByText("距离（从本台）")).toBeVisible();
  await expect(page.getByText(/^\d{2,3}°$/)).toBeVisible();
  await expect(page.getByText(/^\d{3,5} km$/)).toBeVisible();
});

test("呼号查询：没通联过的呼号如实说，没有本台网格时不猜方位", async ({ page }) => {
  await seedLog(page, [
    { callsign: "JA1ABC", date: "2025-03-01", time: "10:00", freq: "14.025", mode: "CW" },
  ]);
  await page.goto("/callsign");

  // 没填本台网格：直接说去填，不给方位与距离。
  await page.getByLabel("呼号").fill("K1ZZ");
  await expect(page.getByText("还没有通联过")).toBeVisible();
  await expect(page.getByText("先在台站信息里填网格")).toBeVisible();
  await expect(page.getByText(/^\d{2,3}°$/)).toHaveCount(0);
});

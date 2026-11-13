import { expect, test } from "./fixtures";

/**
 * 流星雨时刻表（`/events` 里的 IMO 卡片）按 **UTC 日期**算峰值倒计时，时钟固定到 2026-12-12 12:00 UTC。
 * 期望值来自 IMO《Meteor Shower Calendar 2026》表 5，再按日期手算：
 *   双子座 峰值 12/14（2 天后，活动期 12/4–12/20）、小熊座 12/22（10 天后，12/17 才开始）、
 *   后发座贝伦尼斯 12/23（11 天后，活动期跨年 12/4–1/30）、象限仪座 1/3（22 天后，12/28 才开始）；
 *   船尾座–船帆座（12/7）、长蛇座 σ 与麒麟座（都是 12/9）的峰值已过（下一次是明年，360 / 362 / 362 天后），
 *   但活动期还没结束，所以仍「正在活动」，峰值那一列写「峰值已过」而不是一个像是还要等一年的天数；
 *   同为 362 天时按代码排：HYD 在 MON 之前。列哪几行、怎么排在 `meteor_showers.rs` 有同一天的单测。
 */
const NOW = new Date("2026-12-12T12:00:00Z");

function card(page: import("@playwright/test").Page, title: string) {
  return page.locator("section", { has: page.getByRole("heading", { name: title }) });
}

test("流星雨时刻表：按峰值倒计时排序，活动期打标，30 天外且未在活动的不列出", async ({ page }) => {
  await page.clock.setFixedTime(NOW);
  await page.goto("/events");
  const section = card(page, "流星雨时刻表");
  await expect(section).toBeVisible();
  const rows = section.locator(".divide-y > div");

  // 排序：峰值近的在前；已过峰值但仍在活动期的麒麟座（下一次 362 天后）排在最后。
  await expect(rows).toHaveCount(7);
  await expect(rows.nth(0)).toContainText("双子座流星雨");
  await expect(rows.nth(1)).toContainText("小熊座流星雨");
  await expect(rows.nth(2)).toContainText("后发座贝伦尼斯流星雨");
  await expect(rows.nth(3)).toContainText("象限仪座流星雨");
  await expect(rows.nth(4)).toContainText("船尾座–船帆座流星雨");
  await expect(rows.nth(5)).toContainText("长蛇座 σ 流星雨");
  await expect(rows.nth(6)).toContainText("麒麟座流星雨");

  // 双子座：正在活动、峰值 12/14（2 天后）、ZHR 150、保留 IMO 的英文名。
  await expect(rows.nth(0)).toContainText("Geminids");
  await expect(rows.nth(0)).toContainText("正在活动");
  await expect(rows.nth(0)).toContainText("峰值 12/14（2 天后）");
  await expect(rows.nth(0)).toContainText("ZHR 150");
  // 小熊座 12/17 才开始：不打「正在活动」。
  await expect(rows.nth(1)).not.toContainText("正在活动");
  await expect(rows.nth(1)).toContainText("峰值 12/22（10 天后）");
  // 峰值已过、活动期未结束 → 仍标「正在活动」，峰值列写「峰值已过」，不再出现 360 / 362 天后。
  await expect(rows.nth(4)).toContainText("峰值已过（12/7）");
  await expect(rows.nth(6)).toContainText("正在活动");
  await expect(rows.nth(6)).toContainText("峰值已过（12/9）");
  await expect(rows.nth(6)).not.toContainText("天后");

  // 夏季的英仙座既不在活动期、峰值也远在 30 天之外：不列。
  await expect(section).not.toContainText("英仙座流星雨");
  // 署名：IMO 的使用条款要求注明来源。
  await expect(section).toContainText("IMO Meteor Shower Calendar 2026");
  await expect(section).toContainText("International Meteor Organization");
});

test("流星雨时刻表：峰值当天写「峰值今天」", async ({ page }) => {
  await page.clock.setFixedTime(new Date("2026-12-14T03:00:00Z"));
  await page.goto("/events");
  const section = card(page, "流星雨时刻表");
  await expect(section.locator(".divide-y > div").first()).toContainText("峰值今天（12/14）");
});

test("流星雨时刻表：英文界面用译名，不重复显示相同的英文名", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("locale", "en"));
  await page.clock.setFixedTime(NOW);
  await page.goto("/events");
  const section = card(page, "Meteor shower timetable");
  await expect(section).toBeVisible();
  const first = section.locator(".divide-y > div").first();
  await expect(first).toContainText("Peaks 12/14 (in 2 days)");
  await expect(first).toContainText("Active now");
  // 词典里的译名与 IMO 英文名相同：只出现一次，不是 “Geminids Geminids”。
  await expect(first.getByText("Geminids")).toHaveCount(1);
  // 译名词典到位：整张卡里没有汉字残留（词典缺了会静默停在中文）。
  await expect(section.locator(".divide-y")).not.toContainText(/[\u4e00-\u9fff]/);
});

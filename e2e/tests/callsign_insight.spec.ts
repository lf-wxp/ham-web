import { expect, seedLog, stubApi, test } from "./fixtures";

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
  // 页面上另有「呼号结构」「中国呼号分区」等含「呼号」的无障碍名，必须精确匹配输入框。
  await page.getByLabel("呼号", { exact: true }).fill("K1ZZ");
  await expect(page.getByText("还没有通联过")).toBeVisible();
  await expect(page.getByText("先在台站信息里填网格")).toBeVisible();
  await expect(page.getByText(/^\d{2,3}°$/)).toHaveCount(0);
});

test("呼号知识：台站类别按工信部 2024 版分配（J 空间台、S/T/Y/Z 保留）", async ({ page }) => {
  // 类别写的是 core 里的中文，译文在知识库词典；这里同时钉住含义，防止回到「BT 特设 / BY 集体」的旧说法。
  const expectClass = async (call: string, label: string) => {
    await page.goto(`/callsign?call=${call}`);
    const section = page.locator("section").filter({ has: page.getByRole("heading", { name: "呼号解析与查询" }) });
    await expect(section.getByText("台站类别", { exact: true })).toBeVisible();
    await expect(section).toContainText(label);
  };
  await expectClass("BG4XYZ", "个人业余电台");
  await expectClass("BK1AA", "个人业余电台");
  await expectClass("BJ1AA", "空间电台");
  await expectClass("BT4D", "保留");
  await expectClass("BY4AA", "保留");
  await expect(page.locator("main")).not.toContainText("特设电台");
  await expect(page.locator("main")).not.toContainText("集体电台");
});

test("呼号知识：台湾前缀与 DXCC 数据一致，不再出现倒置的范围", async ({ page }) => {
  await page.goto("/callsign");
  await expect(page.locator("main")).toContainText("BM–BQ、BU–BX");
  await expect(page.locator("main")).not.toContainText("BV–BQ");
});

for (const [locale, calls] of [
  ["en", ["BY4AA", "BJ1AA", "BK1AA", "BG4XYZ/QRP", "BG4XYZ/7", "BG4XYZ/MM", "BG4XYZ/P"]],
  ["es", ["BY4AA", "BJ1AA", "BK1AA", "BG4XYZ/QRP", "BG4XYZ/7", "BG4XYZ/MM", "BG4XYZ/P"]],
] as const) {
  test(`呼号知识：${locale} 界面解析结果与参考表不露中文`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("locale", value), locale);
    // 在线查询与本断言无关；打桩避免 503 噪声，也不触外网。
    await stubApi(page, { "/api/callsign": {} });
    for (const call of calls) {
      await page.goto(`/callsign?call=${encodeURIComponent(call)}`);
      // 等知识库译文到达：参考表第一行是译文才算就绪。
      await expect(page.locator("main")).not.toContainText("国际电联分配的国家", { timeout: 15_000 });
      const text = await page.locator("main").innerText();
      const cjk = text.split("\n").filter((line) => /[\u4e00-\u9fff]/.test(line));
      expect(cjk, `${locale} ${call} 仍有中文行`).toEqual([]);
    }
  });
}

test("呼号知识：?call= 深链只查一次，之后逐字输入不再重复请求在线接口", async ({ page }) => {
  const asked: string[] = [];
  await stubApi(page, {
    "/api/callsign": (url) => {
      asked.push(url.searchParams.get("callsign") ?? "");
      return { name: "", grid: "", qth: "", country: "Japan", source: "DXCC" };
    },
  });
  await page.goto("/callsign?call=JA1ABC");
  await expect.poll(() => asked.length).toBe(1);

  const input = page.getByLabel("呼号", { exact: true });
  await input.click();
  await input.press("End");
  await input.pressSequentially("XYZ", { delay: 80 });
  await expect(input).toHaveValue("JA1ABCXYZ");
  // 给潜在的重复请求留出时间再断言：不得多出 JA1ABCX / JA1ABCXY / JA1ABCXYZ。
  await page.waitForTimeout(600);
  expect(asked).toEqual(["JA1ABC"]);
});

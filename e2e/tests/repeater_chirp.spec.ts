import { readFile } from "node:fs/promises";
import { expect, stubApi, test } from "./fixtures";

const HEADER =
  "Location,Name,Frequency,Duplex,Offset,Tone,rToneFreq,cToneFreq,DtcsCode,DtcsPolarity,RxDtcsCode,CrossMode,Mode,TStep,Skip,Power,Comment,URCALL,RPT1CALL,RPT2CALL,DVCODE";

/** 一批覆盖各种跳过原因的中继台（字段与 `/api/repeaters` 一致）。 */
const REPEATERS = [
  { callsign: "BR1BJ", frequency_mhz: "145.6500", offset_mhz: "-0.6000", pl: "88.5", city: "Beijing", use: "OPEN", status: "On-air" },
  { callsign: "BR3SH", frequency_mhz: "439.4000", offset_mhz: "-5.0000", pl: "", city: "Shanghai, CN", use: "OPEN", status: "On-air" },
  { callsign: "BR9OFF", frequency_mhz: "145.7000", offset_mhz: "-0.6000", pl: "", city: "Xian", use: "OPEN", status: "Off-air" },
  { callsign: "W6US", frequency_mhz: "442.0000", offset_mhz: "+5.0000", pl: "100.0", city: "San Jose", use: "OPEN", status: "On-air" },
  { callsign: "BR5DCS", frequency_mhz: "145.5500", offset_mhz: "-0.6000", pl: "DCS 023", city: "Wuhan", use: "OPEN", status: "On-air" },
  { callsign: "BR0BAD", frequency_mhz: "", offset_mhz: "", pl: "", city: "Nowhere", use: "OPEN", status: "On-air" },
];

async function lookup(page: import("@playwright/test").Page) {
  await stubApi(page, {
    "/api/repeaters": { country: "China", count: REPEATERS.length, results: REPEATERS },
  });
  await page.goto("/repeater");
  await page.getByRole("button", { name: "查询" }).click();
  await expect(page.getByRole("cell", { name: "BR1BJ" })).toBeVisible();
}

test("中继台：导出 CHIRP CSV，默认跳过停用 / 波段外 / 非 CTCSS / 坏频率", async ({ page }) => {
  await lookup(page);

  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 CHIRP CSV" }).click();
  const file = await dl;
  expect(file.suggestedFilename()).toBe("chirp-china.csv");
  const csv = await readFile((await file.path())!, "utf8");

  // 只剩 2 条：北京（带亚音）与上海（备注里的逗号要加引号）
  expect(csv).toBe(
    [
      HEADER,
      "0,BR1BJ,145.650000,-,0.600000,Tone,88.5,88.5,023,NN,023,Tone->Tone,FM,5.00,,,Beijing,,,,",
      '1,BR3SH,439.400000,-,5.000000,,88.5,88.5,023,NN,023,Tone->Tone,FM,5.00,,,"Shanghai, CN",,,,',
      "",
    ].join("\r\n"),
  );

  const status = page.getByRole("status").filter({ hasText: "已导出" });
  await expect(status).toContainText("已导出 2 条");
  await expect(status).toContainText("1 条因频率缺失或无效被跳过");
  await expect(status).toContainText("1 条因已停用被跳过");
  await expect(status).toContainText("1 条因接收或发射频率在业余波段之外被跳过");
  await expect(status).toContainText("1 条因亚音不是标准 CTCSS 被跳过");
});

test("中继台：关掉波段与停用过滤后，美国 70 cm 与停用中继也能导出", async ({ page }) => {
  await lookup(page);

  await page.getByRole("checkbox", { name: "只导出落在业余波段内的频点（按我国口径）" }).click();
  await page.getByRole("checkbox", { name: "跳过已停用（Off-air）的中继" }).click();

  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 CHIRP CSV" }).click();
  const csv = await readFile((await (await dl).path())!, "utf8");
  const rows = csv.split("\r\n").filter(Boolean);
  // 头 + 北京 + 上海 + 停用的 + 美国；DCS 与坏频率仍然跳过
  expect(rows).toHaveLength(5);
  expect(rows.some((r) => r.startsWith("3,W6US,442.000000,+,5.000000,Tone,100.0,"))).toBe(true);
  expect(rows.some((r) => r.includes("BR9OFF"))).toBe(true);
  expect(rows.some((r) => r.includes("BR5DCS"))).toBe(false);
});

test("中继台：筛选后只导出可见的，没有可导出的时不下载空文件", async ({ page }) => {
  await lookup(page);

  // 只筛出 DCS 那条：唯一一条会因亚音被跳过 → 不应触发下载
  await page.getByRole("textbox", { name: "筛选" }).fill("BR5DCS");
  let downloaded = false;
  page.on("download", () => {
    downloaded = true;
  });
  await page.getByRole("button", { name: "导出 CHIRP CSV" }).click();
  await expect(page.getByRole("status").filter({ hasText: "已导出" })).toContainText("已导出 0 条");
  await expect(page.getByRole("status").filter({ hasText: "已导出" })).toContainText("1 条因亚音不是标准 CTCSS 被跳过");
  await page.waitForTimeout(300);
  expect(downloaded).toBe(false);

  // 只筛出上海：只导出这一条，且 Location 从 0 起
  await page.getByRole("textbox", { name: "筛选" }).fill("shanghai");
  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 CHIRP CSV" }).click();
  const csv = await readFile((await (await dl).path())!, "utf8");
  const rows = csv.split("\r\n").filter(Boolean);
  expect(rows).toHaveLength(2);
  expect(rows[1].startsWith("0,BR3SH,")).toBe(true);
});

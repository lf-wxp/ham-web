import { readFile } from "node:fs/promises";
import type { Page } from "@playwright/test";
import { expect, seedLog, test, type SeedQso } from "./fixtures";

/** 本台信息：老 key `station-info` 会被迁成第一条台站档案。 */
async function seedStation(page: Page) {
  await page.addInitScript(() =>
    localStorage.setItem(
      "station-info",
      JSON.stringify({ callsign: "BG4XXX", operator: "BG4XXX", gridsquare: "OM89EW", rig: "", antenna: "" }),
    ),
  );
}

/** n 条同一天的 CW 通联（每分钟一条）。 */
function day(n: number, date: string, extra: Partial<SeedQso>): SeedQso[] {
  return Array.from({ length: n }, (_, i) => ({
    callsign: `BD7A${String(i).padStart(2, "0")}`,
    date,
    time: `09:${String(i).padStart(2, "0")}`,
    freq: "14.0620",
    mode: "CW",
    ...extra,
  }));
}

async function open(page: Page) {
  await page.goto("/log");
  const panel = page.locator("details", { hasText: "野外激活导出（SOTA / POTA）" });
  await panel.locator("summary").click();
  return panel;
}

test("野外激活：POTA 按 UTC 日拆文件，写 MY_SIG / MY_SIG_INFO，并给出跳过原因", async ({ page }) => {
  await seedStation(page);
  await seedLog(page, [
    ...day(11, "2026-10-13", { my_pota_ref: "CN-0123" }),
    ...day(1, "2026-10-14", { my_pota_ref: "CN-0123" }),
    // 模式是本站兜底分类，不是合法 ADIF 模式 → 跳过
    { callsign: "BD7DAT", date: "2026-10-13", time: "10:00", freq: "14.07", mode: "DATA", my_pota_ref: "CN-0123" },
    // 另一个公园：不导出，但会出现在候选里
    ...day(2, "2026-10-13", { my_pota_ref: "CN-0456" }),
  ]);
  const panel = await open(page);

  // 候选按通联数降序，默认填进最常用的那个
  await expect(panel.getByLabel("我的公园编号（POTA）")).toHaveValue("CN-0123");
  await expect(panel.getByRole("button", { name: "CN-0123 · 13 条" })).toBeVisible();
  await expect(panel.getByRole("button", { name: "CN-0456 · 2 条" })).toBeVisible();

  // 两个 UTC 日 → 两个文件；只有一条通联的那天提示不足
  const f13 = "BG4XXX@CN-0123-20261013.adi";
  const f14 = "BG4XXX@CN-0123-20261014.adi";
  await expect(panel.getByText(f13)).toBeVisible();
  await expect(panel.getByText(f14)).toBeVisible();
  await expect(panel.getByText("2026-10-13 · 11 个通联")).toBeVisible();
  await expect(panel.getByText("2026-10-14 · 1 个通联")).toBeVisible();
  await expect(panel.getByText("不足 10 个通联")).toHaveCount(1);
  await expect(panel.getByText(/1 条因模式是「数据 \/ 其他」兜底分类/)).toBeVisible();

  const dl = page.waitForEvent("download");
  await panel.getByRole("button", { name: `下载 ${f13}` }).click();
  const file = await dl;
  expect(file.suggestedFilename()).toBe(f13);
  const adif = await readFile((await file.path())!, "utf8");
  expect(adif.match(/<EOR>/g)).toHaveLength(11);
  expect(adif).toContain("<STATION_CALLSIGN:6>BG4XXX");
  expect(adif).toContain("<MY_SIG:4>POTA");
  expect(adif).toContain("<MY_SIG_INFO:7>CN-0123");
  expect(adif).toContain("<QSO_DATE:8>20261013");
  expect(adif).toContain("<BAND:3>20m");
  expect(adif).not.toContain("BD7DAT");
  expect(adif).not.toContain("CN-0456");

  // 点另一个候选：只剩那个公园的 2 条
  await panel.getByRole("button", { name: "CN-0456 · 2 条" }).click();
  await expect(panel.getByLabel("我的公园编号（POTA）")).toHaveValue("CN-0456");
  await expect(panel.getByText("BG4XXX@CN-0456-20261013.adi")).toBeVisible();
  await expect(panel.getByText(f13)).toHaveCount(0);
});

test("野外激活：SOTA 导出 CSV V2，无表头，日期 DD/MM/YYYY，频率带 MHz", async ({ page }) => {
  await seedStation(page);
  await seedLog(page, day(5, "2026-10-13", { my_sota_ref: "BY/BJ-001" }));
  const panel = await open(page);

  await panel.getByRole("radio", { name: "SOTA 山峰" }).click();
  await expect(panel.getByLabel("我的山峰编号（SOTA）")).toHaveValue("BY/BJ-001");
  const name = "sota-BY_BJ-001-20261013.csv";
  await expect(panel.getByText(name)).toBeVisible();
  await expect(panel.getByText("2026-10-13 · 5 个通联")).toBeVisible();
  // 5 ≥ SOTA 的最少通联数，不提示不足
  await expect(panel.getByText(/不足 \d+ 个通联/)).toHaveCount(0);

  const dl = page.waitForEvent("download");
  await panel.getByRole("button", { name: `下载 ${name}` }).click();
  const file = await dl;
  expect(file.suggestedFilename()).toBe(name);
  const csv = await readFile((await file.path())!, "utf8");
  const lines = csv.split("\r\n").filter(Boolean);
  expect(lines).toHaveLength(5);
  expect(lines[0]).toBe("V2,BG4XXX,BY/BJ-001,13/10/2026,0900,14.062MHz,CW,BD7A00,,");
  expect(lines.every((l) => l.startsWith("V2,"))).toBe(true);
});

test("野外激活：编号格式不对时报错，切换平台会换成该平台的编号，日志里没有编号时给出引导", async ({ page }) => {
  await seedStation(page);
  await seedLog(page, [
    ...day(3, "2026-10-13", { my_pota_ref: "CN-0123" }),
    ...day(3, "2026-10-13", { my_sota_ref: "BY/BJ-001" }),
  ]);
  const panel = await open(page);
  const input = panel.getByLabel("我的公园编号（POTA）");

  // 另一个平台的编号在这里不合法：明确报错、不出文件
  await input.fill("BY/BJ-001");
  await expect(panel.getByRole("alert")).toContainText("POTA 编号格式应为");
  await expect(panel.getByText(/\.adi$/)).toHaveCount(0);

  // 多个公园一起填暂不支持
  await input.fill("CN-0123,CN-0456");
  await expect(panel.getByRole("alert")).toContainText("暂不支持一次填多个公园");

  // 清空：只是提示，不是错误
  await input.fill("");
  await expect(panel.getByRole("alert")).toHaveCount(0);
  await expect(panel.getByText("输入编号，或点上面日志里出现过的编号。")).toBeVisible();

  // 切到 SOTA：输入框被换成 SOTA 的编号，而不是留着旧平台的编号
  await panel.getByRole("radio", { name: "SOTA 山峰" }).click();
  await expect(panel.getByLabel("我的山峰编号（SOTA）")).toHaveValue("BY/BJ-001");
  await panel.getByLabel("我的山峰编号（SOTA）").fill("G/LD-1");
  await expect(panel.getByRole("alert")).toContainText("SOTA 编号格式应为");
});

test("野外激活：日志里没有任何合法编号时，给出去哪里填的引导", async ({ page }) => {
  await seedStation(page);
  await seedLog(page, day(3, "2026-10-13", {}));
  const panel = await open(page);
  await expect(panel.getByText(/日志里还没有格式合法的编号/)).toBeVisible();
  await expect(panel.getByRole("button", { name: /条$/ })).toHaveCount(0);
  await expect(panel.getByRole("button", { name: /^下载 / })).toHaveCount(0);
});

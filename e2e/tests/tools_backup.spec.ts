import { readFile } from "node:fs/promises";
import { expect, test } from "./fixtures";

/** 定位某个工具的 section（导航 / 锚点用的 id 与 section 的 id 一致）。 */
function tool(page: import("@playwright/test").Page, id: string) {
  return page.locator(`section#${id}`);
}

test("小工具：频率 ↔ 波长双向换算", async ({ page }) => {
  await page.goto("/tools");
  await expect(page.getByRole("heading", { level: 1, name: "小工具" })).toBeVisible();
  const section = tool(page, "freq-wavelength");
  await expect(section).toBeVisible();

  await section.getByLabel("频率（MHz）").fill("100");
  await expect(section.getByText(/100\.00 MHz ≈ 3\.00 m/)).toBeVisible();

  // 反向：填波长反推频率
  await section.getByLabel("波长（m）").fill("2");
  await expect(section.getByText(/150\.00 MHz ≈ 2\.00 m/)).toBeVisible();
});

test("小工具：驻波比换算反射系数", async ({ page }) => {
  await page.goto("/tools#swr");
  const section = tool(page, "swr");
  await expect(section).toBeVisible();

  await section.getByLabel("驻波比 SWR").fill("3");
  await expect(section.getByText(/反射系数 \|Γ\| = 0\.5000/)).toBeVisible();
  await expect(section.getByText(/回波损耗 6\.02 dB/)).toBeVisible();
});

test("小工具：欧姆定律填两项算第三项", async ({ page }) => {
  await page.goto("/tools#ohms-law");
  const section = tool(page, "ohms-law");
  await expect(section).toBeVisible();

  // 未填够两项时给出引导
  await expect(section.getByText("填写任意两项（U、I、R）后自动计算。")).toBeVisible();

  await section.getByLabel("电压 U（V）").fill("12");
  await section.getByLabel("电流 I（A）").fill("2");
  await expect(section.getByText(/R = 6\.00/)).toBeVisible();
  await expect(section.getByText(/24\.00/)).toBeVisible();
});

test("数据备份：导出 JSON 含本地数据", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("e2e-backup-key", "e2e-value"));
  await page.goto("/tools#backup");
  await expect(page.getByRole("heading", { level: 2, name: "数据备份" })).toBeVisible();
  await expect(page.getByText("本地存储占用（约）")).toBeVisible();

  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出备份" }).click();
  const file = await dl;
  expect(file.suggestedFilename()).toMatch(/^ham-backup-\d{8}\.json$/);

  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["e2e-backup-key"]).toBe("e2e-value");
});

test("数据备份：导入恢复与合并导入", async ({ page }) => {
  await page.goto("/tools#backup");
  await expect(page.getByRole("heading", { level: 2, name: "数据备份" })).toBeVisible();
  const inputs = page.locator('section#backup input[type="file"]');

  // 覆盖导入
  const restore = {
    logbook: JSON.stringify({ entries: [] }),
    "e2e-imported": "cover",
  };
  let alerted = page.waitForEvent("dialog");
  await inputs.nth(0).setInputFiles({
    name: "backup.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(restore)),
  });
  let dlg = await alerted;
  expect(dlg.message()).toMatch(/^已恢复 \d+ 条数据$/);
  await dlg.accept();
  expect(await page.evaluate(() => localStorage.getItem("e2e-imported"))).toBe("cover");

  // 合并导入（保留本机已有的 key，并入新的）
  alerted = page.waitForEvent("dialog");
  await inputs.nth(1).setInputFiles({
    name: "backup2.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify({ "e2e-merged": "yes" })),
  });
  dlg = await alerted;
  expect(dlg.message()).toMatch(/^已合并 \d+ 条数据$/);
  await dlg.accept();
  expect(await page.evaluate(() => localStorage.getItem("e2e-imported"))).toBe("cover");
});

test("数据备份：非法 JSON 给出失败提示而不是静默", async ({ page }) => {
  await page.goto("/tools#backup");
  await expect(page.getByRole("heading", { level: 2, name: "数据备份" })).toBeVisible();

  const alerted = page.waitForEvent("dialog");
  await page.locator('section#backup input[type="file"]').first().setInputFiles({
    name: "broken.json",
    mimeType: "application/json",
    buffer: Buffer.from("这不是 JSON"),
  });
  const dlg = await alerted;
  expect(dlg.message()).toMatch(/^导入失败：/);
  await dlg.accept();
});

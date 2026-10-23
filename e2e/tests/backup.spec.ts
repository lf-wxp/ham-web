import { readFile } from "node:fs/promises";
import { expect, test } from "./fixtures";

/**
 * 数据备份（`/backup`）。
 *
 * 原先是小工具页最底部的 `/tools#backup`，埋在几十个计算器之后；现在独立成页，
 * 入口在顶栏「设置」对话框、页面页脚与移动菜单里。
 */
test("数据备份：导出 JSON 含本地数据", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("e2e-backup-key", "e2e-value"));
  await page.goto("/backup");
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();
  await expect(page.getByText("本地存储占用（约）")).toBeVisible();

  // 导出成功后有一个「已导出 N 项」提示（全局 Toast，`data-slot="toast"` 定位）。
  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出备份" }).click();
  const file = await dl;
  expect(file.suggestedFilename()).toMatch(/^ham-backup-\d{8}\.json$/);
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^已导出 \d+ 项$/);

  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["e2e-backup-key"]).toBe("e2e-value");
});

test("数据备份：按模块勾选导出（不勾选的通联数据不进备份）", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("logbook", JSON.stringify({ entries: [] }));
    localStorage.setItem("e2e-backup-key", "e2e-value");
  });
  await page.goto("/backup");
  const section = page.locator("section#backup");
  await expect(section.getByText("导出模块（按需勾选，减小备份体积）")).toBeVisible();

  // 只保留「其他本地数据」，取消通联数据与 QSL 影像。
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^QSL 影像（扫描件/).uncheck();
  const dl = page.waitForEvent("download");
  await section.getByRole("button", { name: "导出备份" }).click();
  const file = await dl;
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^已导出 \d+ 项$/);

  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["logbook"]).toBeUndefined();
  expect(data["e2e-backup-key"]).toBe("e2e-value");
});

test("数据备份：只勾选 QSL 影像也要导出影像与索引；导入拒绝不合格影像", async ({ page }) => {
  await page.goto("/backup");
  const section = page.locator("section#backup");
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();

  // 经导入路径把影像写进 IndexedDB（导入是外部输入：SVG 与非法 key 必须被拒绝，不计入条数）。
  const png = "data:image/png;base64,iVBORw0KGgo=";
  const backup = {
    "qsl-image-index": JSON.stringify([7]),
    "qsl-image:7": png,
    "qsl-image:8": "data:image/svg+xml;base64,PHN2Zz4=",
    "qsl-image:x": png,
  };
  await section.locator('input[type="file"]').first().setInputFiles({
    name: "qsl.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(backup)),
  });
  // 索引（localStorage）+ 合格的 1 张影像 = 2 条（结果走全局 Toast）。
  await expect(page.locator('[data-slot="toast"]')).toContainText("已恢复 2 条数据");

  // 取消「通联数据」与「其他」：索引不在被勾选的模块里，影像仍要导出。
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^其他本地数据/).uncheck();
  const dl = page.waitForEvent("download");
  await section.getByRole("button", { name: "导出备份" }).click();
  const file = await dl;

  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["qsl-image:7"]).toBe(png);
  expect(data["qsl-image-index"]).toBe(JSON.stringify([7]));
  expect(data["qsl-image:8"]).toBeUndefined();
  expect(data["logbook"]).toBeUndefined();
});

test("数据备份：三个模块全不勾选时不能导出", async ({ page }) => {
  await page.goto("/backup");
  const section = page.locator("section#backup");
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^QSL 影像（扫描件/).uncheck();
  await section.getByLabel(/^其他本地数据/).uncheck();
  await expect(section.getByRole("button", { name: "导出备份" })).toBeDisabled();
  await section.getByLabel(/^其他本地数据/).check();
  await expect(section.getByRole("button", { name: "导出备份" })).toBeEnabled();
});

test("数据备份：导入恢复与合并导入", async ({ page }) => {
  await page.goto("/backup");
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();
  const inputs = page.locator('section#backup input[type="file"]');

  // 覆盖导入
  const restore = {
    logbook: JSON.stringify({ entries: [] }),
    "e2e-imported": "cover",
  };
  await inputs.nth(0).setInputFiles({
    name: "backup.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(restore)),
  });
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^已恢复 \d+ 条数据$/);
  expect(await page.evaluate(() => localStorage.getItem("e2e-imported"))).toBe("cover");

  // 合并导入（保留本机已有的 key，并入新的）
  await inputs.nth(1).setInputFiles({
    name: "backup2.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify({ "e2e-merged": "yes" })),
  });
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^已合并 \d+ 条数据$/);
  expect(await page.evaluate(() => localStorage.getItem("e2e-imported"))).toBe("cover");
  expect(await page.evaluate(() => localStorage.getItem("e2e-merged"))).toBe("yes");
});

test("数据备份：跨设备合并重映射影像 ID，保留本机扫描件", async ({ page }) => {
  await page.goto("/backup");
  const inputs = page.locator('section#backup input[type="file"]');
  const entry = (id: number, callsign: string, date: string) => ({
    id, date, time: "12:00", freq: "14.074", mode: "FT8", callsign, remark: "",
  });
  const a = "data:image/png;base64,QUFB";
  const b = "data:image/png;base64,QkJC";
  const importFile = async (merge: boolean, data: Record<string, string>) => {
    await expect(page.locator('[data-slot="toast"]')).toHaveCount(0);
    await inputs.nth(merge ? 1 : 0).setInputFiles({
      name: "images.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(data)),
    });
    await expect(page.locator('[data-slot="toast"]')).toContainText(merge ? /^已合并/ : /^已恢复/);
    await expect(page.locator('section#backup input[type="file"]').first()).toBeEnabled();
    await expect(page.locator('[data-slot="toast"]')).toHaveCount(0);
  };
  await importFile(false, {
    logbook: JSON.stringify({ entries: [entry(1, "JA1AAA", "2026-01-01")] }),
    "qsl-image-index": "[1]", "qsl-image:1": a,
  });
  const backup = {
    logbook: JSON.stringify({ entries: [entry(1, "W1AW", "2026-01-02")] }),
    "qsl-image-index": "[1]", "qsl-image:1": b,
  };
  await importFile(true, backup);
  await importFile(true, backup);
  const book = await page.evaluate(() => JSON.parse(localStorage.getItem("logbook") ?? "null"));
  expect(book.entries.map((e: { id: number }) => e.id)).toEqual([1, 2]);
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem("qsl-image-index") ?? "null"))).toEqual([1, 2]);
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出备份" }).click();
  const file = await download;
  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["qsl-image:1"]).toBe(a);
  expect(data["qsl-image:2"]).toBe(b);
  await page.locator("[data-nav]").getByRole("button", { name: "工具", exact: true }).click();
  await page.locator("[data-nav]").getByRole("button", { name: "日志 · 竞赛", exact: true }).click();
  await page.locator('[data-nav] a[href="/log"]').first().click();
  await expect(page.getByRole("row").filter({ hasText: "W1AW" })).toHaveCount(1);
  await page.locator('input[placeholder="BG4XXX"]').fill("DL1AAA");
  await page.getByRole("button", { name: "添加记录" }).click();
  const after = await page.evaluate(() => JSON.parse(localStorage.getItem("logbook") ?? "null"));
  expect(after.entries.map((e: { callsign: string }) => e.callsign)).toEqual(["JA1AAA", "W1AW", "DL1AAA"]);
});

test("覆盖新日志会删除同 ID 的旧影像，随后合并正确影像不误认领", async ({ page }) => {
  await page.goto("/backup");
  const inputs = page.locator('section#backup input[type="file"]');
  const log = (callsign: string) => JSON.stringify({ entries: [{
    id: 1, date: "2026-01-01", time: "12:00", freq: "14.074", mode: "FT8", callsign, remark: "",
  }] });
  const a = "data:image/png;base64,QUFB";
  const b = "data:image/png;base64,QkJC";
  const restore = async (merge: boolean, data: Record<string, string>) => {
    await expect(page.locator('[data-slot="toast"]')).toHaveCount(0);
    await inputs.nth(merge ? 1 : 0).setInputFiles({
      name: "cover.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(data)),
    });
    await expect(page.locator('[data-slot="toast"]')).toContainText(merge ? /^已合并/ : /^已恢复/);
    await expect(page.locator('section#backup input[type="file"]').first()).toBeEnabled();
    await expect(page.locator('[data-slot="toast"]')).toHaveCount(0);
  };
  await restore(false, { logbook: log("JA1AAA"), "qsl-image-index": "[1]", "qsl-image:1": a });
  await restore(false, { logbook: log("W1AW"), "qsl-image-index": "[]" });
  await restore(true, { logbook: log("W1AW"), "qsl-image-index": "[1]", "qsl-image:1": b });
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出备份" }).click();
  const file = await download;
  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(JSON.parse(data.logbook).entries[0].callsign).toBe("W1AW");
  expect(data["qsl-image:1"]).toBe(b);
});

test("数据备份：非法 JSON 给出失败提示而不是静默", async ({ page }) => {
  await page.goto("/backup");
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();

  await page.locator('section#backup input[type="file"]').first().setInputFiles({
    name: "broken.json",
    mimeType: "application/json",
    buffer: Buffer.from("这不是 JSON"),
  });
  await expect(page.locator('[data-slot="toast"]')).toContainText(/^导入失败：/);
});

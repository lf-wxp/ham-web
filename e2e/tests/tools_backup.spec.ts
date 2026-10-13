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
  // 文案在 i18n 重构里改成了「回波损耗取正值为 {} dB」，这里只钉数值
  await expect(section.getByText(/回波损耗.*6\.02 dB/)).toBeVisible();
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

  // 导出成功后有一个「已导出 N 项」提示 —— 监听器必须在点击前注册，
  // 否则 alert 会在等待下载的间隙被 Playwright 自动关掉。
  const dl = page.waitForEvent("download");
  const dlg = page.waitForEvent("dialog");
  // alert 会挂起页面所有动作 —— 点击不等待完成，直接等下载与对话框两个事件。
  page.getByRole("button", { name: "导出备份" })
    .click({ noWaitAfter: true })
    .catch(() => {});
  const file = await dl;
  expect(file.suggestedFilename()).toMatch(/^ham-backup-\d{8}\.json$/);
  const d = await dlg;
  expect(d.message()).toMatch(/^已导出 \d+ 项$/);
  await d.accept();

  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["e2e-backup-key"]).toBe("e2e-value");
});

test("数据备份：按模块勾选导出（不勾选的通联数据不进备份）", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("logbook", JSON.stringify({ entries: [] }));
    localStorage.setItem("e2e-backup-key", "e2e-value");
  });
  await page.goto("/tools#backup");
  const section = page.locator("section#backup");
  await expect(section.getByText("导出模块（按需勾选，减小备份体积）")).toBeVisible();

  // 只保留「其他本地数据」，取消通联数据与 QSL 影像。
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^QSL 影像（扫描件/).uncheck();
  const dl = page.waitForEvent("download");
  const dlg = page.waitForEvent("dialog");
  section
    .getByRole("button", { name: "导出备份" })
    .click({ noWaitAfter: true })
    .catch(() => {});
  const file = await dl;
  const d = await dlg;
  expect(d.message()).toMatch(/^已导出 \d+ 项$/);
  await d.accept();

  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["logbook"]).toBeUndefined();
  expect(data["e2e-backup-key"]).toBe("e2e-value");
});

test("数据备份：只勾选 QSL 影像也要导出影像与索引；导入拒绝不合格影像", async ({ page }) => {
  await page.goto("/tools#backup");
  const section = page.locator("section#backup");
  await expect(page.getByRole("heading", { level: 2, name: "数据备份" })).toBeVisible();

  // 经导入路径把影像写进 IndexedDB（导入是外部输入：SVG 与非法 key 必须被拒绝，不计入条数）。
  const png = "data:image/png;base64,iVBORw0KGgo=";
  const backup = {
    "qsl-image-index": JSON.stringify([7]),
    "qsl-image:7": png,
    "qsl-image:8": "data:image/svg+xml;base64,PHN2Zz4=",
    "qsl-image:x": png,
  };
  const imported = page.waitForEvent("dialog");
  await section.locator('input[type="file"]').first().setInputFiles({
    name: "qsl.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(backup)),
  });
  const importDlg = await imported;
  // 索引（localStorage）+ 合格的 1 张影像 = 2 条。
  expect(importDlg.message()).toBe("已恢复 2 条数据");
  await importDlg.accept();

  // 取消「通联数据」与「其他」：索引不在被勾选的模块里，影像仍要导出。
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^其他本地数据/).uncheck();
  const dl = page.waitForEvent("download");
  const dlg = page.waitForEvent("dialog");
  section
    .getByRole("button", { name: "导出备份" })
    .click({ noWaitAfter: true })
    .catch(() => {});
  const file = await dl;
  await (await dlg).accept();

  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["qsl-image:7"]).toBe(png);
  expect(data["qsl-image-index"]).toBe(JSON.stringify([7]));
  expect(data["qsl-image:8"]).toBeUndefined();
  expect(data["logbook"]).toBeUndefined();
});

test("数据备份：三个模块全不勾选时不能导出", async ({ page }) => {
  await page.goto("/tools#backup");
  const section = page.locator("section#backup");
  await section.getByLabel(/^通联数据/).uncheck();
  await section.getByLabel(/^QSL 影像（扫描件/).uncheck();
  await section.getByLabel(/^其他本地数据/).uncheck();
  await expect(section.getByRole("button", { name: "导出备份" })).toBeDisabled();
  await section.getByLabel(/^其他本地数据/).check();
  await expect(section.getByRole("button", { name: "导出备份" })).toBeEnabled();
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

test("数据备份：跨设备合并重映射影像 ID，保留本机扫描件", async ({ page }) => {
  await page.goto("/tools#backup");
  const inputs = page.locator('section#backup input[type="file"]');
  const entry = (id: number, callsign: string, date: string) => ({
    id, date, time: "12:00", freq: "14.074", mode: "FT8", callsign, remark: "",
  });
  const a = "data:image/png;base64,QUFB";
  const b = "data:image/png;base64,QkJC";
  const importFile = async (merge: boolean, data: Record<string, string>) => {
    const alerted = page.waitForEvent("dialog");
    await inputs.nth(merge ? 1 : 0).setInputFiles({
      name: "images.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(data)),
    });
    const dlg = await alerted;
    expect(dlg.message()).not.toMatch(/失败/);
    await dlg.accept();
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
  const alerted = page.waitForEvent("dialog");
  const click = page.getByRole("button", { name: "导出备份" }).click();
  const file = await download;
  await (await alerted).accept();
  await click;
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
  await page.goto("/tools#backup");
  const inputs = page.locator('section#backup input[type="file"]');
  const log = (callsign: string) => JSON.stringify({ entries: [{
    id: 1, date: "2026-01-01", time: "12:00", freq: "14.074", mode: "FT8", callsign, remark: "",
  }] });
  const a = "data:image/png;base64,QUFB";
  const b = "data:image/png;base64,QkJC";
  const restore = async (merge: boolean, data: Record<string, string>) => {
    const dialog = page.waitForEvent("dialog");
    await inputs.nth(merge ? 1 : 0).setInputFiles({
      name: "cover.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(data)),
    });
    const shown = await dialog;
    expect(shown.message()).not.toMatch(/失败/);
    await shown.accept();
  };
  await restore(false, { logbook: log("JA1AAA"), "qsl-image-index": "[1]", "qsl-image:1": a });
  await restore(false, { logbook: log("W1AW"), "qsl-image-index": "[]" });
  await restore(true, { logbook: log("W1AW"), "qsl-image-index": "[1]", "qsl-image:1": b });
  const download = page.waitForEvent("download");
  const dialog = page.waitForEvent("dialog");
  const click = page.getByRole("button", { name: "导出备份" }).click();
  const file = await download;
  await (await dialog).accept();
  await click;
  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(JSON.parse(data.logbook).entries[0].callsign).toBe("W1AW");
  expect(data["qsl-image:1"]).toBe(b);
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

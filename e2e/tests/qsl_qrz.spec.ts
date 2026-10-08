import { readFile } from "node:fs/promises";

import { expect, pickOption, test, type Page } from "./fixtures";

/** 本地那条通联（与报告同键，不带任何 QSL 标记）。 */
const QSO_ADIF = `Generated for e2e
<ADIF_VER:5>3.1.4 <EOH>
<CALL:5>JA1AA <QSO_DATE:8>20240501 <TIME_ON:4>1230 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.200 <EOR>
`;

/**
 * 带本工具扩展字段的 QRZ 报告 —— 本工具自己导出的 ADIF 就是这种。
 *
 * QRZ 的站内确认在 ADIF 里没有标准字段，所以本工具写自己的 `APP_HAMEXAMWEB_QRZ_RCVD`。
 */
const QRZ_AWARE_REPORT = `Generated for e2e
<ADIF_VER:5>3.1.4 <EOH>
<CALL:5>JA1AA <QSO_DATE:8>20240501 <TIME_ON:4>1230 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.200 <APP_HAMEXAMWEB_QRZ_RCVD:1>Y <EOR>
`;

/** QRZ 官方导出那样的报告：只有通用 `QSL_RCVD`，没有我们的扩展字段。 */
const QRZ_PLAIN_REPORT = `Generated for e2e
<ADIF_VER:5>3.1.4 <EOH>
<CALL:5>JA1AA <QSO_DATE:8>20240501 <TIME_ON:4>1230 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.200 <QSL_RCVD:1>Y <EOR>
`;

async function importLocal(page: Page) {
  // 导入完成的提示框由测试顶部那个持续处理器接受（这里再 accept 一次会报
  // 「dialog already handled」）。
  await page
    .locator('input[type="file"][accept*=".adi"]')
    .setInputFiles({
      name: "local.adi",
      mimeType: "text/plain",
      buffer: Buffer.from(QSO_ADIF),
    });
  // 等列表真的刷新出来再往下走：导入会触发一次重渲染。
  await expect(logRow(page)).toHaveCount(1);
}

/** 日志列表里 JA1AA 那一行（对话框的差异表也是 `<tr>`，所以限定在 `main` 里）。 */
function logRow(page: Page) {
  return page.locator("main").getByRole("row").filter({ hasText: "JA1AA" });
}

async function openSync(page: Page) {
  await page.getByRole("button", { name: "同步 QSL" }).click();
  const dialog = page.getByRole("dialog", { name: "同步 QSL 确认" });
  await expect(dialog).toBeVisible();
  return dialog;
}

test("QRZ 确认：勾上走电子档位，导出带上扩展字段", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");

  // 表单里勾「QRZ 已确认」再加一条通联（带的频率让波段/距离有意义）。
  const form = page.locator("section", { hasText: "添加通联记录" });
  await form.locator('input[placeholder="BG4XXX"]').fill("JA1AA");
  await page.getByLabel("QRZ 已确认").check();
  await page.getByRole("button", { name: "添加记录" }).click();
  await expect(page.getByText("共 1 条")).toBeVisible();
  await expect(logRow(page)).toContainText("QRZ 已确认");

  // 导出 ADIF：QRZ 确认没有标准字段，走本工具的扩展字段（别的软件会忽略它）。
  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 ADIF" }).click();
  const text = await readFile(await (await dl).path(), "utf8");
  expect(text).toContain("<APP_HAMEXAMWEB_QRZ_RCVD:1>Y");

  // 档位是「电子已确认」（蓝色）：能被这一档筛出来，且不在「未寄出」里。
  await pickOption(page, "QSL 筛选", "电子已确认");
  await expect(logRow(page)).toHaveCount(1);
  await pickOption(page, "QSL 筛选", "未寄出");
  await expect(logRow(page)).toHaveCount(0);
});

test("QRZ 报告：带扩展字段能落库，官方导出只做匹配分析", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await page.goto("/log");
  await importLocal(page);

  const dialog = await openSync(page);
  await dialog.getByRole("radio", { name: "QRZ Logbook" }).click();
  // 先讲清这个来源的来龙去脉，否则用户会奇怪「为什么那份报告什么也没改」。
  await expect(dialog.getByText(/QRZ 的确认走本工具自己的扩展字段/)).toBeVisible();

  // 带扩展字段的报告（本工具自己的导出）：QRZ 确认能补上。
  await dialog.locator("textarea").fill(QRZ_AWARE_REPORT);
  await dialog.getByRole("button", { name: "分析差异" }).click();
  await expect(
    dialog.getByText("报告 1 条 · 对上 1 条 · 需处理 1 条 · 本地缺失 0 条"),
  ).toBeVisible();
  const ja1aa = dialog.getByRole("row").filter({ hasText: "JA1AA" });
  await expect(ja1aa).toContainText("QRZ 已确认");
  await dialog.getByRole("button", { name: "应用确认" }).click();
  await expect(dialog.getByText("已补上 1 个标志位，清除 0 个。")).toBeVisible();

  // 换成 QRZ 官方导出那种：只做匹配分析，而且**不能**把已记的确认判成冲突
  // （那会在「以远端为准」时清掉用户的确认）。
  await dialog.locator("textarea").fill(QRZ_PLAIN_REPORT);
  await dialog.getByRole("button", { name: "分析差异" }).click();
  await expect(
    dialog.getByText("报告 1 条 · 对上 1 条 · 需处理 0 条 · 本地缺失 0 条"),
  ).toBeVisible();
  await expect(dialog.getByText(/没有可采纳的确认标记/)).toBeVisible();

  // 关掉对话框看徽章：QRZ 确认已经落库。
  await dialog.getByRole("button", { name: "关闭" }).click();
  await expect(page.getByRole("dialog", { name: "同步 QSL 确认" })).toBeHidden();
  await expect(logRow(page)).toContainText("QRZ 已确认");
});

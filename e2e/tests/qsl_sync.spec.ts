import { expect, test, type Page } from "./fixtures";

/** 本地那条通联（与报告同键，只是没有任何 QSL 标记）。 */
const QSO_ADIF = `Generated for e2e
<ADIF_VER:5>3.1.4 <EOH>
<CALL:5>JA1AA <QSO_DATE:8>20240501 <TIME_ON:4>1230 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.200 <EOR>
`;

/** LoTW 报告：只有 LoTW 确认，一条通用 `QSL_*` 都没有。 */
const LOTW_REPORT = `Generated for e2e
<ADIF_VER:5>3.1.4 <EOH>
<CALL:5>JA1AA <QSO_DATE:8>20240501 <TIME_ON:4>1230 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.200 <LOTW_QSL_RCVD:1>Y <EOR>
`;

/** 打开「同步 QSL 确认」对话框。 */
async function openSync(page: Page) {
  await page.getByRole("button", { name: "同步 QSL" }).click();
  const dialog = page.getByRole("dialog", { name: "同步 QSL 确认" });
  await expect(dialog).toBeVisible();
  return dialog;
}

/**
 * 日志列表里 JA1AA 那一行。
 *
 * 必须限定在 `main` 里：对话框走 `<Portal>` 挂到 `body`，它的差异表也是 `<tr>`，
 * 不限定范围就会同时命中两张表。
 */
function logRow(page: Page) {
  return page.locator("main").getByRole("row").filter({ hasText: "JA1AA" });
}

/** 差异表里 JA1AA 那一行。 */
function row(dialog: ReturnType<Page["getByRole"]>) {
  return dialog.getByRole("row").filter({ hasText: "JA1AA" });
}

test("QSL 同步：三向差异、来源决定哪些字段算数、逐条裁决", async ({ page }) => {
  // 本地这条要与报告同键（呼号 + 日期 + 时间 + 波段 + 模式），所以用 ADIF 导入而不是
  // 手填表单（表单默认是今天、且没有频率 → 对不上）。
  await page.goto("/log");
  const imported = page.waitForEvent("dialog");
  await page
    .locator('input[type="file"][accept*=".adi"]')
    .setInputFiles({
      name: "local.adi",
      mimeType: "text/plain",
      buffer: Buffer.from(QSO_ADIF),
    });
  (await imported).accept();
  await expect(logRow(page)).toHaveCount(1);

  // 本地手抄「已经收到纸卡」—— 报告里没有这一位，于是它会成为冲突。
  await page.getByRole("button", { name: "编辑" }).first().click();
  await page.getByLabel("QSL 已收到").check();
  await page.getByRole("button", { name: "保存修改" }).click();
  await expect(logRow(page)).toContainText("QSL 已收到");

  const dialog = await openSync(page);

  // 来源选「其他 / 手抄清单」：三个渠道都算数，于是纸质确认会成为冲突。
  await dialog.getByRole("radio", { name: "其他 / 手抄清单" }).click();
  await expect(dialog.getByText(/本报告只代表/)).toBeVisible();
  await dialog.locator("textarea").fill(LOTW_REPORT);
  await dialog.getByRole("button", { name: "分析差异" }).click();

  // 三向：报告 1 条 / 对上 1 条 / 需处理 1 条 / 本地缺失 0 条。
  await expect(dialog.getByText("报告 1 条 · 对上 1 条 · 需处理 1 条 · 本地缺失 0 条")).toBeVisible();
  const ja1aa = row(dialog);
  // 远端新增：LoTW 已确认；冲突：本地有「QSL 已收到」而报告里没有。
  await expect(ja1aa).toContainText("LoTW 已确认");
  await expect(ja1aa).toContainText("QSL 已收到");

  // 默认只勾「补上」，「以远端为准」不勾（它会清掉本地确认）。
  const add = ja1aa.getByLabel("补上");
  const mirror = ja1aa.getByLabel("以远端为准");
  await expect(add).toBeChecked();
  await expect(mirror).not.toBeChecked();

  // 换成 LoTW 来源：纸质卡不归它管，冲突必须消失 —— 这是「来源决定哪些字段算数」。
  await dialog.getByRole("radio", { name: "LoTW", exact: true }).click();
  await dialog.getByRole("button", { name: "分析差异" }).click();
  await expect(dialog.getByText(/本报告只代表 LoTW/)).toBeVisible();
  await expect(row(dialog)).not.toContainText("QSL 已收到");
  await expect(row(dialog)).toContainText("LoTW 已确认");

  // 换回「其他」并应用：LoTW 确认补上，本地的纸卡确认必须原样保留。
  await dialog.getByRole("radio", { name: "其他 / 手抄清单" }).click();
  await dialog.getByRole("button", { name: "分析差异" }).click();
  await dialog.getByRole("button", { name: "应用确认" }).click();
  await expect(dialog.getByText("已补上 1 个标志位，清除 0 个。")).toBeVisible();

  // 「补上」这一列变灰 = 没有可补的了，证明 LoTW 确认已经落库；冲突那一行还在，
  // 因为刚才没有勾「以远端为准」。
  await expect(row(dialog).getByLabel("补上")).toBeDisabled();
  await expect(row(dialog)).toContainText("QSL 已收到");

  // 第二次：只剩那一位冲突，这次勾「以远端为准」→ 清掉本地的纸卡确认。
  await row(dialog).getByLabel("以远端为准").check();
  await dialog.getByRole("button", { name: "应用确认" }).click();
  await expect(dialog.getByText("已补上 0 个标志位，清除 1 个。")).toBeVisible();
  await expect(dialog.getByText("没有需要处理的差异。")).toBeVisible();

  // QRZ 报告只做匹配分析：不猜归属，因此没有可应用的差异。
  await dialog.getByRole("radio", { name: "QRZ Logbook" }).click();
  await expect(dialog.getByText(/没有可落库的渠道/)).toBeVisible();

  // 关掉对话框再看徽章：纸卡确认被清掉后，LoTW 确认升为徽章上那一档。
  await dialog.getByRole("button", { name: "关闭" }).click();
  await expect(page.getByRole("dialog", { name: "同步 QSL 确认" })).toBeHidden();
  await expect(logRow(page)).toContainText("LoTW 已确认");
  await expect(logRow(page)).not.toContainText("QSL 已收到");
});

test("QSL 同步：本地缺失的记录列出来", async ({ page }) => {
  await page.goto("/log");
  const dialog = await openSync(page);
  await dialog.locator("textarea").fill(
    `Generated for e2e
<ADIF_VER:5>3.1.4 <EOH>
<CALL:4>DL1A <QSO_DATE:8>20240502 <TIME_ON:4>0100 <BAND:3>40m <MODE:2>CW <FREQ:5>7.030 <LOTW_QSL_RCVD:1>Y <EOR>
`,
  );
  await dialog.getByRole("button", { name: "分析差异" }).click();
  await expect(dialog.getByText("报告 1 条 · 对上 0 条 · 需处理 0 条 · 本地缺失 1 条")).toBeVisible();
  await expect(dialog.getByText(/在本地日志里没有对应通联/)).toBeVisible();
  // 明细带上定位信息，才能判断为什么没匹配上。
  await expect(dialog.getByText(/DL1A · 2024-05-02 01:00 · 40m · CW/)).toBeVisible();
});

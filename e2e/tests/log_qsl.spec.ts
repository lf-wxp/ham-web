import { readFile } from "node:fs/promises";
import { expect, test, type Locator, type Page } from "@playwright/test";

import { pickOption } from "./fixtures";

/// 四条通联覆盖四色档位：未寄出 / 已寄出（卡片局）/ LoTW 已确认 / 纸质已收。
const ADIF = `Generated for e2e
<ADIF_VER:5>3.1.4 <EOH>
<CALL:5>JA1AA <QSO_DATE:8>20240501 <TIME_ON:4>1230 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.200 <EOR>
<CALL:4>W1AW <QSO_DATE:8>20240502 <TIME_ON:4>0100 <BAND:3>40m <MODE:2>CW <FREQ:5>7.030 <QSL_SENT:1>Y <QSL_SENT_VIA:1>B <EOR>
<CALL:6>DL1ABC <QSO_DATE:8>20240503 <TIME_ON:4>0200 <BAND:3>15m <MODE:3>FT8 <FREQ:6>21.074 <LOTW_QSL_RCVD:1>Y <EOR>
<CALL:5>K1TTT <QSO_DATE:8>20240504 <TIME_ON:4>0300 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.250 <QSL_RCVD:1>Y <EOR>
`;

/// QSL 列：按 `data-slot` 取，不数第几列（列增删时序号会整体错位）。
function qslCell(page: Page, call: string): Locator {
  return page
    .getByRole("row")
    .filter({ hasText: call })
    .locator('[data-slot="qsl-cell"]');
}

/// 徽章文案（去掉元素之间的空白，方便整串比对）。
const badgeText = (page: Page, call: string) =>
  expect.poll(async () => (await qslCell(page, call).innerText()).replace(/\s+/g, ""));

/// 徽章配色类名：四色档位靠它区分。
const badgeClass = async (page: Page, call: string) =>
  (await qslCell(page, call).locator("span").first().getAttribute("class")) ?? "";

async function importAdif(page: Page, text: string) {
  const dialog = page.waitForEvent("dialog");
  await page
    .locator('input[type="file"][accept*=".adi"]')
    .setInputFiles({ name: "qsl.adi", mimeType: "text/plain", buffer: Buffer.from(text) });
  (await dialog).accept();
}

test("QSL 状态：四色徽章按确认途径着色，筛选按档位收敛", async ({ page }) => {
  await page.goto("/log");
  await importAdif(page, ADIF);
  await expect(page.getByText("共 4 条")).toBeVisible();

  // 未寄出：灰。
  await badgeText(page, "JA1AA").toBe("未寄出");
  expect(await badgeClass(page, "JA1AA")).toContain("bg-muted");

  // 已寄出（卡片局）：琥珀，且徽章带上寄出方式后缀。
  await badgeText(page, "W1AW").toBe("已寄出·卡片局");
  expect(await badgeClass(page, "W1AW")).toContain("amber");

  // LoTW 已确认：蓝。这条曾显示成「—」（旧列表只看 qsl_rcvd），是本次修掉的核心问题。
  await badgeText(page, "DL1ABC").toBe("LoTW已确认");
  expect(await badgeClass(page, "DL1ABC")).toContain("sky");

  // 纸质已收：绿。
  await badgeText(page, "K1TTT").toBe("QSL已收到");
  expect(await badgeClass(page, "K1TTT")).toContain("emerald");

  // 按档位筛选：电子已确认只留下 LoTW 那条。
  await pickOption(page, "QSL 筛选", "电子已确认");
  await expect(page.getByText(/筛选出 1 \/ 4 条/)).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "DL1ABC" })).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "K1TTT" })).toHaveCount(0);

  // 纸质已收：只剩收到实体卡的那条（LoTW 确认不算）。
  await pickOption(page, "QSL 筛选", "QSL 已收到");
  await expect(page.getByText(/筛选出 1 \/ 4 条/)).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "K1TTT" })).toBeVisible();

  // 回到「全部」。
  await pickOption(page, "QSL 筛选", "全部 QSL");
  await expect(page.getByText("共 4 条")).toBeVisible();
});

test("QSL 状态：表单与标签页记录寄出方式，并写进 ADIF", async ({ page }) => {
  await page.goto("/log");

  // 表单里选「直寄」：一条记录同时表达「已寄出」与方式，不需要再勾一个「已寄出」。
  await page.locator('input[placeholder="BG4XXX"]').fill("JA1AA");
  await pickOption(page, "QSL 寄出方式", "直寄");
  await page.getByRole("button", { name: "添加记录" }).click();
  await expect(page.getByText("共 1 条")).toBeVisible();
  await badgeText(page, "JA1AA").toBe("已寄出·直寄");

  // 改回「未寄出」：方式与寄出标志一起收敛，不会留下「未寄出 + 直寄」这种矛盾组合。
  await page.getByRole("button", { name: "编辑" }).first().click();
  await pickOption(page, "QSL 寄出方式", "未寄出");
  await page.getByRole("button", { name: "保存修改" }).click();
  await badgeText(page, "JA1AA").toBe("未寄出");

  // 标签页批量标记：先选好方式，再标记为已寄出（默认卡片局，这里换成卡片局验证下拉生效）。
  await page.goto("/qsl-labels");
  await pickOption(page, "QSL 寄出方式", "卡片局");
  page.once("dialog", (d) => d.accept());
  await page.getByRole("button", { name: /标记为已寄出/ }).click();
  await expect(page.getByText("没有待寄出的通联")).toBeVisible();

  // 用页面内的「返回日志」链接回到日志（SPA 内跳转，走同一份响应式 store）。
  await page.getByRole("link", { name: "返回日志" }).click();
  await expect(page).toHaveURL(/\/log$/);
  await badgeText(page, "JA1AA").toBe("已寄出·卡片局");

  // 导出 ADIF：寄出方式落到 `QSL_SENT_VIA`（没记方式时不会凭空多出这一行）。
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 ADIF" }).click();
  const text = await readFile(await (await download).path(), "utf8");
  expect(text).toContain("<QSL_SENT:1>Y");
  expect(text).toContain("<QSL_SENT_VIA:1>B");
});

import { readFile } from "node:fs/promises";
import { expect, test } from "./fixtures";

/** 呼号输入框没有 <label> 包裹，用 placeholder 定位。 */
const callInput = (page: import("@playwright/test").Page) => page.locator('input[placeholder="BG4XXX"]');

test("山峰 / 公园：我的与对方的分开记，ADIF 按规范写 MY_* 与 SOTA_REF / POTA_REF，导回来仍在原位", async ({ page }) => {
  await page.goto("/log");
  await callInput(page).fill("JA1ZZZ");
  await page.getByRole("button", { name: /^更多字段/ }).click();

  await page.getByLabel("我的 SOTA 山峰（MY_SOTA_REF）").fill("by/bj-001");
  await page.getByLabel("我的 POTA 公园（MY_POTA_REF）").fill("cn-0123");
  await page.getByLabel("对方的 SOTA 山峰（SOTA_REF，山峰对山峰）").fill("ja/tk-001");
  await page.getByLabel("对方的 POTA 公园（POTA_REF，公园对公园）").fill("ja-0456");
  await page.getByRole("button", { name: "添加记录" }).click();

  // 列表：自己的写 SOTA / POTA，对方的写 S2S / P2P，都已转成大写
  const row = page.getByRole("row", { name: /JA1ZZZ/ });
  await expect(row).toContainText("SOTA BY/BJ-001");
  await expect(row).toContainText("POTA CN-0123");
  await expect(row).toContainText("S2S JA/TK-001");
  await expect(row).toContainText("P2P JA-0456");

  // 对方的编号也能被搜到
  await page.getByPlaceholder(/搜索/).first().fill("JA/TK");
  await expect(row).toBeVisible();
  await page.getByPlaceholder(/搜索/).first().fill("");

  // ADIF：自己的是 MY_*，对方的才是 SOTA_REF / POTA_REF
  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 ADIF" }).click();
  const file = await dl;
  const adif = await readFile(await file.path(), "utf8");
  expect(adif).toContain("<MY_SOTA_REF:9>BY/BJ-001");
  expect(adif).toContain("<MY_POTA_REF:7>CN-0123");
  expect(adif).toContain("<SOTA_REF:9>JA/TK-001");
  expect(adif).toContain("<POTA_REF:7>JA-0456");

  // 清空后把刚导出的文件导回来：四个字段原样回到各自的位置，没有串位
  page.once("dialog", (d) => d.accept()); // 清空确认
  await page.getByRole("button", { name: /清空/ }).click();
  await expect(page.getByRole("row", { name: /JA1ZZZ/ })).toHaveCount(0);
  const imported = page.waitForEvent("dialog");
  await page
    .locator('input[type="file"][accept*=".adi"]')
    .setInputFiles({ name: "back.adi", mimeType: "text/plain", buffer: Buffer.from(adif) });
  await (await imported).accept();
  const back = page.getByRole("row", { name: /JA1ZZZ/ });
  await expect(back).toContainText("SOTA BY/BJ-001");
  await expect(back).toContainText("POTA CN-0123");
  await expect(back).toContainText("S2S JA/TK-001");
  await expect(back).toContainText("P2P JA-0456");
});

test("山峰 / 公园：旧存档里的 sota_ref / pota_ref 读成「我的」，其余软件导出的 SOTA_REF 读成对方的", async ({ page }) => {
  // 旧版本的存档只有 `sota_ref` / `pota_ref`，那时界面把它们当「自己所在的山峰 / 公园」用。
  await page.addInitScript(() => {
    localStorage.setItem(
      "logbook",
      JSON.stringify({
        entries: [
          { id: 1, date: "2026-10-13", time: "09:00", freq: "14.062", mode: "CW", callsign: "BD7OLD", remark: "", sota_ref: "BY/BJ-001", pota_ref: "CN-0123" },
        ],
      }),
    );
  });
  await page.goto("/log");
  const row = page.getByRole("row", { name: /BD7OLD/ });
  await expect(row).toContainText("SOTA BY/BJ-001");
  await expect(row).toContainText("POTA CN-0123");
  await expect(row).not.toContainText("S2S");
  await expect(row).not.toContainText("P2P");

  // 别的软件导出的文件：SOTA_REF 是对方的
  const foreign =
    "<EOH>\n<CALL:6>JA1HUN<QSO_DATE:8>20261013<TIME_ON:4>1000<MODE:2>CW<FREQ:6>14.062<SOTA_REF:9>JA/TK-001<EOR>";
  const imported = page.waitForEvent("dialog");
  await page
    .locator('input[type="file"][accept*=".adi"]')
    .setInputFiles({ name: "hunter.adi", mimeType: "text/plain", buffer: Buffer.from(foreign) });
  await (await imported).accept();
  const hunter = page.getByRole("row", { name: /JA1HUN/ });
  await expect(hunter).toContainText("S2S JA/TK-001");
  await expect(hunter).not.toContainText("SOTA JA/TK-001");
});

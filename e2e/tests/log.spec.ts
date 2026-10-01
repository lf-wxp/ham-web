import { readFile } from "node:fs/promises";
import { expect, test } from "./fixtures";

const ADIF = `Generated for e2e
<ADIF_VER:5>3.1.4 <EOH>
<CALL:5>JA1AA <QSO_DATE:8>20240501 <TIME_ON:4>1230 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.200 <RST_SENT:2>59 <RST_RCVD:2>57 <GRIDSQUARE:4>PM95 <EOR>
<CALL:4>W1AW <QSO_DATE:8>20240502 <TIME_ON:4>0100 <BAND:3>40m <MODE:2>CW <FREQ:5>7.030 <RST_SENT:3>599 <RST_RCVD:3>579 <EOR>
`;

test("通联日志：导入 ADIF、去重、导出 ADIF / CSV", async ({ page }) => {
  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();

  const input = page.locator('input[type="file"][accept*=".adi"]');
  const file = { name: "e2e.adi", mimeType: "text/plain", buffer: Buffer.from(ADIF) };

  const imported = page.waitForEvent("dialog");
  await input.setInputFiles(file);
  const dlg = await imported;
  expect(dlg.message()).toBe("已导入 2 条记录");
  await dlg.accept();

  await expect(page.getByText("JA1AA").first()).toBeVisible();
  await expect(page.getByText("W1AW").first()).toBeVisible();

  const again = page.waitForEvent("dialog");
  await input.setInputFiles(file);
  const dupDlg = await again;
  expect(dupDlg.message()).toBe("已导入 0 条记录，跳过重复 2 条");
  await dupDlg.accept();

  const adifDl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 ADIF" }).click();
  const adif = await adifDl;
  expect(adif.suggestedFilename()).toMatch(/^logbook-\d{4}-\d{2}-\d{2}\.adi$/);
  const adifText = await readFile(await adif.path(), "utf8");
  expect(adifText).toContain("<CALL:5>JA1AA");
  expect(adifText).toContain("<CALL:4>W1AW");
  expect(adifText).toMatch(/<EOH>/i);

  const csvDl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 CSV" }).click();
  const csv = await csvDl;
  const csvText = await readFile(await csv.path(), "utf8");
  expect(csvText.split(/\r?\n/).filter(Boolean)).toHaveLength(3);
  expect(csvText).toContain("JA1AA");

  await page.reload();
  await expect(page.getByText("W1AW").first()).toBeVisible();
});

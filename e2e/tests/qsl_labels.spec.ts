import { expect, pickOption, test } from "./fixtures";

const ADIF = `e2e
<EOH>
<CALL:5>JA1AA <QSO_DATE:8>20240501 <TIME_ON:4>1230 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.200 <RST_SENT:2>59 <EOR>
<CALL:4>W1AW <QSO_DATE:8>20240502 <TIME_ON:4>0100 <BAND:3>40m <MODE:2>CW <FREQ:5>7.030 <RST_SENT:3>599 <EOR>
<CALL:4>W1AW <QSO_DATE:8>20240503 <TIME_ON:4>0200 <BAND:3>20m <MODE:3>FT8 <FREQ:6>14.074 <RST_SENT:3>-10 <EOR>
`;

test("QSL 标签：按呼号合并、标记已寄出、切换版式", async ({ page }) => {
  await page.goto("/log");
  const imported = page.waitForEvent("dialog");
  await page
    .locator('input[type="file"][accept*=".adi"]')
    .setInputFiles({ name: "e2e.adi", mimeType: "text/plain", buffer: Buffer.from(ADIF) });
  await (await imported).accept();

  await page.getByRole("link", { name: "打印 QSL 标签" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "QSL 标签打印" })).toBeVisible();
  await expect(page.getByText("2 张标签 · 3 条通联 · 1 页")).toBeVisible();

  const w1aw = page.locator('[data-label="W1AW"]');
  await expect(w1aw).toHaveCount(1);
  await expect(w1aw.getByRole("row")).toHaveCount(3);
  await expect(w1aw).toContainText("7.030");
  await expect(w1aw).toContainText("14.074");

  // 跳过前 3 张：第一页前 3 个位置留空
  await page.getByLabel("跳过前").fill("3");
  // 按 `data-slot` 取标签槽，不数直接子节点（插一层包裹 div 就会整体错位）。
  const slots = page.locator('[data-slot="label-page"]').first().locator('[data-slot="label-slot"]');
  await expect(slots.nth(3)).toHaveAttribute("data-label", "JA1AA");

  await pickOption(page, "标签纸", "Avery 5160 · Letter · 3×10（66.7×25.4 mm）");
  await expect(slots).toHaveCount(30);

  page.once("dialog", (d) => d.accept());
  await page.getByRole("button", { name: "标记为已寄出（3）" }).click();
  await expect(page.getByText("没有待寄出的通联")).toBeVisible();

  // 通联范围是 `ChipGroup` + `Chip`（互斥选择）：role 是 `radio`，不是 `button`。
  await page.getByRole("radio", { name: "全部" }).click();
  await expect(page.locator('[data-label="W1AW"]')).toHaveCount(1);
});

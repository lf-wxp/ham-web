import { expect, seedLog, test } from "./fixtures";

test("奖状进度：DXCC 申请记录表每个实体一条并优先已确认", async ({ page }) => {
  await seedLog(page, [
    // 同一实体（日本）两条：早的未确认、晚的已确认 → 表里留晚的那条。
    { callsign: "JA1ABC", date: "2025-03-01", freq: "14.025", mode: "CW" },
    { callsign: "JA1XYZ", date: "2025-06-01", freq: "7.025", mode: "CW", lotw_rcvd: true },
    { callsign: "K1ABC", date: "2025-04-01", freq: "21.025", mode: "CW" },
  ]);
  await page.goto("/log");

  const sheet = page.locator('[data-slot="dxcc-apply"]');
  await sheet.getByRole("button", { name: /DXCC 申请记录表/ }).click();
  await expect(sheet.getByText(/ARRL 在线申请没有公开的上传格式/)).toBeVisible();

  // 表头 + 两个实体（日本、美国）。
  await expect(sheet.getByRole("row")).toHaveCount(3);
  await expect(sheet.getByRole("row", { name: /JA1XYZ/ })).toBeVisible();
  await expect(sheet.getByRole("row", { name: /JA1ABC/ })).toHaveCount(0);
  await expect(sheet.getByRole("row", { name: /JA1XYZ/ })).toContainText("LoTW");
  await expect(sheet.getByRole("row", { name: /K1ABC/ })).toContainText("尚未确认");

  const download = page.waitForEvent("download");
  await sheet.getByRole("button", { name: "导出 CSV" }).click();
  expect((await download).suggestedFilename()).toBe("dxcc-apply.csv");
});

test("奖状进度：没有实体的日志显示空态", async ({ page }) => {
  await seedLog(page, [{ callsign: "", date: "2025-01-01", freq: "" }]);
  await page.goto("/log");

  const sheet = page.locator('[data-slot="dxcc-apply"]');
  await sheet.getByRole("button", { name: /DXCC 申请记录表/ }).click();
  await expect(sheet.getByText("日志里还没有识别出实体的通联。")).toBeVisible();
});

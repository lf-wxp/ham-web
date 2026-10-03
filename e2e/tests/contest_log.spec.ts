import { expect, test } from "./fixtures";

test("竞赛录入：呼号过短不记录，正常呼号入库并提示实体", async ({ page }) => {
  await page.goto("/contest-log");
  await expect(page.getByRole("heading", { level: 1, name: "竞赛录入" })).toBeVisible();
  await expect(page.getByText("还没有通联，输入呼号开始吧")).toBeVisible();

  const call = page.getByLabel("对方呼号");
  await call.fill("J");
  await page.getByRole("button", { name: "记录" }).click();
  await expect(page.getByText("还没有通联，输入呼号开始吧")).toBeVisible();

  await call.fill("ja1aa");
  // 呼号自动大写，并给出实体提示
  await expect(call).toHaveValue("JA1AA");
  await expect(page.locator("#contest-call-hint")).toContainText(/CQ \d+/);

  await page.getByRole("button", { name: "记录" }).click();
  await expect(page.locator("#contest-call-hint")).toContainText("已记录 JA1AA");
  await expect(page.getByRole("row").filter({ hasText: "JA1AA" })).toHaveCount(1);
});

test("竞赛录入：重复呼号标为重复且不计分", async ({ page }) => {
  await page.goto("/contest-log");
  await expect(page.getByRole("heading", { level: 1, name: "竞赛录入" })).toBeVisible();

  const call = page.getByLabel("对方呼号");
  await call.fill("JA1AA");
  await page.getByRole("button", { name: "记录" }).click();
  await expect(page.locator("#contest-call-hint")).toContainText("已记录 JA1AA");

  // 同一波段再来一次：先给出重复预警
  await call.fill("JA1AA");
  await expect(page.locator("#contest-call-hint")).toContainText("重复：本波段已通联过");
  await page.getByRole("button", { name: "记录" }).click();
  await expect(page.locator("#contest-call-hint")).toContainText("（重复，不计分）");
});

test("竞赛录入：空格跳到交换信息，Esc 清空输入", async ({ page }) => {
  await page.goto("/contest-log");
  await expect(page.getByRole("heading", { level: 1, name: "竞赛录入" })).toBeVisible();

  const call = page.getByLabel("对方呼号");
  await call.fill("JA1AA");
  await call.press(" ");
  // 焦点跳到「收到的…」输入框
  const rcvd = page.getByLabel(/^收到的/);
  await expect(rcvd).toBeFocused();

  await call.focus();
  await page.keyboard.press("Escape");
  await expect(call).toHaveValue("");
  await expect(rcvd).toHaveValue("");
});

test("竞赛录入：可导出 Cabrillo 并可开始新一场", async ({ page }) => {
  await page.goto("/contest-log?contest=CQ-WW-CW");
  await expect(page.getByRole("heading", { level: 1, name: "竞赛录入" })).toBeVisible();

  await page.getByLabel("对方呼号").fill("JA1AA");
  await page.getByRole("button", { name: "记录" }).click();
  await expect(page.getByRole("row").filter({ hasText: "JA1AA" })).toHaveCount(1);

  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 Cabrillo" }).click();
  const file = await dl;
  // 未设置本台呼号时文件名以 log 结尾
  expect(file.suggestedFilename()).toMatch(/^.+-\w+\.cbr$/);

  await page.getByRole("button", { name: "开始新一场" }).click();
  await expect(page.locator("#contest-call-hint")).toContainText("已开始新一场");
});

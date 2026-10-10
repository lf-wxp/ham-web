import { expect, pickOption, test } from "./fixtures";

test("DIY 项目：按难度与所需仪表筛选", async ({ page }) => {
  await page.goto("/diy-projects");
  await expect(page.getByRole("heading", { level: 2, name: "半波偶极天线" })).toBeVisible();
  // 默认六张卡片。
  await expect(page.locator("article")).toHaveCount(6);

  // 难度 = 进阶 → 磁环小环 + 陷波器。
  await pickOption(page, "难度", "进阶");
  await expect(page.locator("article")).toHaveCount(2);
  await expect(page.getByRole("heading", { level: 2, name: "磁环小环天线" })).toBeVisible();

  // 再叠加仪表 = VNA → 仍是这两张（都是进阶且需要 VNA）。
  await pickOption(page, "所需仪表", "天线分析仪 / VNA");
  await expect(page.locator("article")).toHaveCount(2);

  // 仪表 = 万用表（不限难度）→ 1:1 巴伦 + CW 练习器。
  await pickOption(page, "难度", "全部");
  await pickOption(page, "所需仪表", "万用表");
  await expect(page.locator("article")).toHaveCount(2);
  await expect(page.getByRole("heading", { level: 2, name: "1:1 电流巴伦" })).toBeVisible();
  await expect(page.getByRole("heading", { level: 2, name: "CW 练习器" })).toBeVisible();
});

test("DIY 项目：进度与采购勾选只存本机，互不影响", async ({ page }) => {
  await page.goto("/diy-projects");
  const card = page.locator("article").filter({ hasText: "半波偶极天线" }).first();
  await card.getByLabel("已采购：铜导线").check();
  await card.getByLabel("完成这个项目").check();
  await expect(card.getByLabel("已采购：铜导线")).toBeChecked();

  // 刷新后还在（localStorage）。
  await page.reload();
  const again = page.locator("article").filter({ hasText: "半波偶极天线" }).first();
  await expect(again.getByLabel("已采购：铜导线")).toBeChecked();
  await expect(again.getByLabel("完成这个项目")).toBeChecked();

  // 别的项目不受影响。
  const other = page.locator("article").filter({ hasText: "1:1 电流巴伦" }).first();
  await expect(other.getByLabel("完成这个项目")).not.toBeChecked();
});

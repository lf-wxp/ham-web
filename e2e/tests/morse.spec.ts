import { expect, test } from "./fixtures";

test("摩尔斯页：训练区与参考表完整渲染", async ({ page }) => {
  await page.goto("/morse");

  await expect(page.getByRole("heading", { name: "莫尔斯电码" })).toBeVisible();
  for (const name of [
    "解码练习",
    "Koch 法抄收训练",
    "呼号抄收 · 竞赛模拟",
    "发报练习",
    "缩语速答",
    "CW 解码（麦克风）",
  ]) {
    await expect(page.getByRole("heading", { name })).toBeVisible();
  }

  // Koch 默认第 2 级（已学 K、M）
  await expect(page.getByText("第 2 / 41 级")).toBeVisible();

  // 字母表与数字参考卡片
  await expect(page.getByRole("heading", { name: "字母表" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "数字" })).toBeVisible();

  // CW 专用符号参考
  await expect(page.getByRole("heading", { name: /CW 专用符号/ })).toBeVisible();
  await expect(page.getByText("报文结束")).toBeVisible();
});

test("发报练习：长按出划并可用重拍清空", async ({ page }) => {
  await page.goto("/morse");
  const key = page.getByRole("button", { name: "按住发报" });
  const marks = page.getByTestId("send-marks");

  await expect(marks).toContainText("…");

  // 长按（≥0.2s）→ 划（点/划阈值 200ms 依赖真实时延，e2e 只用确定性长按验证）
  await key.dispatchEvent("pointerdown");
  await page.waitForTimeout(300);
  await key.dispatchEvent("pointerup");
  await expect(marks).toContainText("—");

  // 重拍清空
  await page.getByRole("button", { name: "重拍" }).click();
  await expect(marks).toContainText("…");
});

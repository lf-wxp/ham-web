import { expect, test } from "./fixtures";

/**
 * 执照类别 × 频率 → 合规结论：可用性、功率上限、是否需协调、法条依据。
 * 期望值来自《业余无线电台管理办法》第三十 / 十五 / 三十二条与 ITU 脚注（5.67A / 5.133B）。
 */
test("合规检查：选类别与频率，给出可用性、功率上限与法条依据", async ({ page }) => {
  await page.goto("/compliance");

  // 结论区按 `data-slot` 限定作用域：页面上还有全局的 `role="status"`（toast / PWA 通知 /
  // 备份提示），不限定作用域时严格模式会因为命中多个元素而失败在这里。
  const result = page.locator('[data-slot="compliance-result"]');
  const status = result.getByRole("status");
  const freq = page.getByLabel("频率（MHz）");

  // 默认 A 类 + 145.0 MHz（2m，唯一主要）：可用，≤25 W，无需协调。
  await expect(status).toHaveText(/^可用$/);
  await expect(result.getByText("波段：2m · 144-146MHz · 使用状态：唯一主要")).toBeVisible();
  await expect(result.getByText("功率上限 ≤ 25 W")).toBeVisible();
  await expect(result.getByText("无需频率协调")).toBeVisible();
  await expect(
    result.getByRole("link", { name: "《业余无线电台管理办法》第30条" }),
  ).toBeVisible();

  // A 类在 40m（7.05）：频率在业余波段内，但类别无权使用。
  await freq.fill("7.05");
  await expect(status).toHaveText(/^不可用$/);
  await expect(result.getByText("A 类无权使用该频率")).toBeVisible();

  // B 类在 40m：可用，≤15 W，专用（无需协调）。
  await page.getByRole("radio", { name: "B 类" }).click();
  await expect(status).toHaveText(/^可用$/);
  await expect(result.getByText("波段：40m · 7-7.1MHz · 使用状态：专用")).toBeVisible();
  await expect(result.getByText("功率上限 ≤ 15 W")).toBeVisible();

  // C 类在 60m（5.36）：可用，但被 5.133B 脚注压到 ≤15 W，次要业务需协调。
  await page.getByRole("radio", { name: "C 类" }).click();
  await freq.fill("5.36");
  await expect(result.getByText("功率上限 ≤ 15 W")).toBeVisible();
  await expect(result.getByText("受 ITU 脚注 5.133B 限制：≤ 15 W e.i.r.p.")).toBeVisible();
  await expect(result.getByText("需开展频率协调")).toBeVisible();

  // 不在业余频段内。
  await freq.fill("10.5");
  await expect(status).toHaveText("该频率不在业余业务频段内");

  // 非法输入给出提示。
  await freq.fill("abc");
  await expect(result.getByRole("alert")).toHaveText("请输入有效的频率（MHz）");
});

import { expect, test } from "./fixtures";

test("波段规划：LF / MF 两段只安排 CW 与窄带数据", async ({ page }) => {
  await page.goto("/bandplan");

  const lf = page.locator("article").filter({ has: page.getByRole("heading", { name: "2200m" }) });
  // 频段范围与子段范围是同一串文字，两处都在卡片里。
  await expect(lf.getByText("135.7–137.8 kHz").first()).toBeVisible();
  await expect(lf.getByText(/仅 CW \/ 窄带数据（LF/)).toBeVisible();

  const mf = page.locator("article").filter({ has: page.getByRole("heading", { name: "630m" }) });
  await expect(mf.getByText("472–479 kHz").first()).toBeVisible();
  await expect(mf.getByText(/仅 CW \/ 窄带数据（MF）/)).toBeVisible();
});

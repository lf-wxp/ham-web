import { expect, test } from "./fixtures";

test("数字语音页：模拟中继联网三种方式与接入要点", async ({ page }) => {
  await page.goto("/dv-network");

  await expect(
    page.getByRole("heading", { name: "模拟中继联网（语音网关）" }),
  ).toBeVisible();
  await expect(page.getByRole("row", { name: /EchoLink/ })).toBeVisible();
  await expect(page.getByRole("row", { name: /IRLP/ })).toBeVisible();
  await expect(page.getByRole("row", { name: /AllStar Link/ })).toBeVisible();
  // 接入要点里最要紧的一条：网关仍然占用本地频率。
  await expect(page.getByText(/先听再做/)).toBeVisible();
});

test("数字语音页：模拟联网与数字网络分表呈现", async ({ page }) => {
  await page.goto("/dv-network");

  // DMR / D-STAR 仍在「主要网络」，不应混进模拟联网表。
  const analog = page.locator("section", { hasText: "模拟中继联网" });
  await expect(analog.getByText("DMR", { exact: true })).toHaveCount(0);
  await expect(analog.getByText("D-STAR", { exact: true })).toHaveCount(0);
});

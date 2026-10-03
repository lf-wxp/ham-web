import { expect, test } from "./fixtures";

test("主导航：展开考试中心下拉可跳转到错题集", async ({ page }) => {
  await page.goto("/");
  const nav = page.locator('[data-nav]');
  await expect(nav).toBeVisible();

  await nav.getByRole("button", { name: "考试中心" }).click();
  await nav.locator('a[href="/mistakes"]').click();
  await expect(page).toHaveURL(/\/mistakes$/);
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();
});

test("主导航：展开工具下拉可跳转到小工具", async ({ page }) => {
  await page.goto("/");
  const nav = page.locator("[data-nav]");
  await nav.getByRole("button", { name: /工具/ }).first().click();
  await nav.locator('a[href="/tools"]').first().click();
  await expect(page).toHaveURL(/\/tools$/);
  await expect(page.getByRole("heading", { level: 1, name: "小工具" })).toBeVisible();
});

test("明暗主题：切换后写入本地偏好并作用于 <html>", async ({ page }) => {
  await page.goto("/");
  const toggle = page.getByRole("button", { name: "切换到深色模式" });
  await expect(toggle).toBeVisible();
  await toggle.click();

  await expect(page.locator("html")).toHaveClass(/dark/);
  await expect.poll(() => page.evaluate(() => localStorage.getItem("theme"))).toBe("dark");

  await page.getByRole("button", { name: "切换到浅色模式" }).click();
  await expect(page.locator("html")).not.toHaveClass(/dark/);
  await expect.poll(() => page.evaluate(() => localStorage.getItem("theme"))).toBe("light");

  // 偏好在刷新后保留
  await page.getByRole("button", { name: "切换到深色模式" }).click();
  await page.reload();
  await expect(page.locator("html")).toHaveClass(/dark/);
});

test("界面语言：切换后同步 <html lang> 并持久化", async ({ page }) => {
  await page.goto("/");
  // 语言切换后 aria-label 也会跟着翻译，这里用结构定位而非文案
  const select = page.locator("[data-nav] select").first();
  await expect(select).toBeVisible();

  await select.selectOption("en");
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect.poll(() => page.evaluate(() => localStorage.getItem("locale"))).toBe("en");

  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");

  await page.locator("[data-nav] select").first().selectOption("zh");
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
});

test("通联统计：空日志时引导到通联日志页", async ({ page }) => {
  await page.goto("/stats");
  await expect(page.getByRole("heading", { level: 1, name: "通联统计" })).toBeVisible();
  await expect(page.getByText("暂无通联日志。")).toBeVisible();

  await page.getByRole("link", { name: "去添加通联日志 →" }).click();
  await expect(page).toHaveURL(/\/log$/);
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
});

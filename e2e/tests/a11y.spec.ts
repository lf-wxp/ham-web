import { expect, option, test } from "./fixtures";

test("键盘：跳转链接聚焦主内容", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  await page.keyboard.press("Tab");
  const skip = page.getByRole("link", { name: "跳到主要内容" });
  await expect(skip).toBeFocused();
  await expect(skip).toBeInViewport();
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
});

test("键盘：点选选项后方向键仍可切题", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  await option(page, "A").click();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();
  await page.keyboard.press("ArrowLeft");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
});

test("键盘：对话框命名、Tab 循环、Esc 关闭并还原焦点", async ({ page }) => {
  await page.goto("/practice");
  const opener = page.locator("main").getByRole("button", { name: "设置" });
  await opener.click();
  const dialog = page.getByRole("dialog", { name: "设置" });
  await expect(dialog).toBeVisible();

  const close = dialog.getByRole("button", { name: "关闭" });
  await close.focus();
  for (let i = 0; i < 30; i++) {
    await page.keyboard.press("Tab");
    const inside = await dialog.evaluate((d) => d.contains(document.activeElement));
    expect(inside).toBe(true);
  }

  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(opener).toBeFocused();
});

test("键盘：Tabs 方向键换页、Home / End 跳首尾", async ({ page }) => {
  await page.goto("/morse");
  const tabs = page.getByRole("tablist", { name: "训练" });
  const koch = tabs.getByRole("tab", { name: "Koch 训练" });
  const callsign = tabs.getByRole("tab", { name: "呼号抄收" });

  await koch.focus();
  await page.keyboard.press("ArrowRight");
  // 自动激活：焦点先过去，随之切到那块面板
  await expect(callsign).toBeFocused();
  await expect(callsign).toHaveAttribute("aria-selected", "true");
  await expect(page.getByRole("heading", { name: "呼号抄收 · 竞赛模拟" })).toBeVisible();

  await page.keyboard.press("End");
  await expect(tabs.getByRole("tab", { name: "发报练习" })).toBeFocused();

  await page.keyboard.press("Home");
  await expect(koch).toBeFocused();
  await expect(koch).toHaveAttribute("aria-selected", "true");
});

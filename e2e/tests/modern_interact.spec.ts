import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";

// 现代风的**交互面**回归：结构对了不等于能用。
// 覆盖调谐台、下拉、确认弹层、移动端菜单四条主链路。

async function seed(page: Page) {
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "modern");
    localStorage.setItem("theme", "dark");
  });
}

test("调谐台：⌘K 打开 → 搜索 → ↑↓ 选择 → Enter 跳转 → Esc 关闭", async ({ page }) => {
  await seed(page);
  await page.goto("/");
  await expect(page.locator(".md-nav").first()).toBeVisible();

  await page.keyboard.press("Meta+k");
  await expect(page.locator(".md-palette")).toBeVisible();

  await page.keyboard.type("工具");
  await expect(page.locator(".md-palette").first()).toContainText("工具");

  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/tools/);

  await page.keyboard.press("Escape");
  await expect(page.locator(".md-palette")).toHaveCount(0);
});

test("下拉：可展开、↑↓ 可移动、Enter 选中并收起", async ({ page }) => {
  await seed(page);
  await page.goto("/log");
  const combo = page.locator('main [role="combobox"]').nth(5);
  await combo.scrollIntoViewIfNeeded();
  const before = (await combo.textContent())?.trim() ?? "";

  await combo.click();
  await expect(page.locator('[role="listbox"]')).toBeVisible();

  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page.locator('[role="listbox"]')).toHaveCount(0);

  const after = (await combo.textContent())?.trim() ?? "";
  expect(after).not.toBe(before);
});

test("确认弹层：打开后 Esc 关闭，DOM 移除", async ({ page }) => {
  await seed(page);
  await page.goto("/bookmarks");
  await page.getByRole("textbox").first().fill("临时组");
  await page.getByRole("button", { name: "新建分组" }).first().click();
  await page.getByRole("button", { name: "删除该分组" }).first().click();

  const panel = page.locator(".md-modal-panel");
  await expect(panel).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
});

test("移动端菜单：dock 打开 → 点条目跳转 → 面板自动收起", async ({ page }) => {
  await seed(page);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await page.locator('.md-dock button:has-text("更多")').click();
  await expect(page.locator(".md-menu-panel")).toBeVisible();

  await page.locator('.md-menu-panel a[href="/browse"]').first().click();
  await expect(page).toHaveURL(/\/browse$/);
  await expect(page.locator(".md-menu-panel")).toHaveCount(0);
});

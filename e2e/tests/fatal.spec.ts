import { readFile } from "node:fs/promises";
import { expect, test } from "./fixtures";

test("wasm 加载失败时显示兜底页而非白屏", async ({ page }) => {
  await page.route(/_bg\.wasm$/, (route) => route.abort());
  await page.goto("/");
  const fatal = page.getByRole("alertdialog", { name: "加载失败" });
  await expect(fatal).toBeVisible();
  await expect(fatal.getByRole("button", { name: "重新加载" })).toBeFocused();
  await expect(page.locator("#boot")).toHaveCount(0);
});

test("兜底页可导出数据备份", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("main h1").first()).toBeAttached();
  await page.evaluate(() => {
    localStorage.setItem("e2e-key", "e2e-value");
    (window as unknown as { __hamFatal: (t: string, d: string) => void }).__hamFatal("页面出错了", "panicked at test");
  });
  const fatal = page.getByRole("alertdialog", { name: "页面出错了" });
  await expect(fatal).toBeVisible();
  await fatal.getByText("错误详情").click();
  await expect(fatal.getByText(/panicked at test/)).toBeVisible();

  const dl = page.waitForEvent("download");
  await fatal.getByRole("button", { name: "导出数据备份" }).click();
  const file = await dl;
  expect(file.suggestedFilename()).toMatch(/^ham-backup-\d{8}\.json$/);
  const data = JSON.parse(await readFile(await file.path(), "utf8"));
  expect(data["e2e-key"]).toBe("e2e-value");
});

test("兜底页跟随保存的界面语言（wasm 未加载时不走词典）", async ({ page }) => {
  await page.route(/_bg\.wasm$/, (route) => route.abort());
  await page.addInitScript(() => localStorage.setItem("locale", "en"));
  await page.goto("/");
  const fatal = page.getByRole("alertdialog", { name: "Failed to load" });
  await expect(fatal).toBeVisible();
  await expect(fatal.getByRole("button", { name: "Reload" })).toBeFocused();
  await expect(fatal.getByRole("button", { name: "Export data backup" })).toBeVisible();
  await expect(fatal.getByText("Error details")).toBeVisible();
});

// 兜底页由 index.html 的内联样式渲染（wasm 已崩，用不到两套 Kit），所以风格派发要在那里
// 按 `data-ui-style` 分支：漏了就是「现代风界面 + 像素墨框弹层」。
test("现代风下兜底页也是现代样式", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("ui:uiStyle", "modern");
    localStorage.setItem("theme", "dark");
  });
  await page.route(/_bg\.wasm$/, (route) => route.abort());
  await page.goto("/");
  const fatal = page.getByRole("alertdialog", { name: "加载失败" });
  await expect(fatal).toBeVisible();
  const panel = fatal.locator("section");
  await expect(panel).toHaveCSS("border-radius", "14px");
  await expect(panel).toHaveCSS("border-top-width", "1px");
  await expect(panel).toHaveCSS("background-color", "rgb(11, 13, 19)");
  await expect(fatal.getByRole("button", { name: "重新加载" })).toHaveCSS("border-radius", "999px");
});

// 浏览器语言设成英文：兜底页仍应回中文（与应用默认一致）。用 `test.use` 而不是手动
// `newContext`：断言失败时手动创建的 context 不会被关闭，会泄漏到后面的用例。
test.describe("浏览器语言为英文", () => {
  test.use({ locale: "en-US" });
  test("兜底页在没有保存语言时回中文（与应用默认一致，不看浏览器语言）", async ({ page }) => {
    await page.route(/_bg\.wasm$/, (route) => route.abort());
    await page.goto("/");
    await expect(page.getByRole("alertdialog", { name: "加载失败" })).toBeVisible();
  });
});

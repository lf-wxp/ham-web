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

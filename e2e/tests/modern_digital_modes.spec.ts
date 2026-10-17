import { expect, test } from "./fixtures";

/**
 * 现代数字模式专题：JS8Call / FreeDV / VARA / ARDOP / M17。
 * 模式名是专名（不译），三语下都原样显示；核心概念、模式对比表、要点走知识库词典。
 * 正文翻译（core 数据）只在 release 的 postbuild 后加载，其正确性由 knowledge-i18n-check 保证；
 * 这里只断言界面部分（标题 / section 标题）的三语与专名、表格结构。
 */

test("专题页：现代数字模式列出五个模式、模式对比表与要点", async ({ page }) => {
  await page.goto("/modern-digital-modes");
  await expect(page.getByRole("heading", { name: "现代数字模式" })).toBeVisible();
  await expect(page.getByText("JS8Call / FreeDV / VARA / ARDOP / M17")).toBeVisible();

  // 五个模式名与共同概念。
  for (const name of ["JS8Call", "FreeDV", "VARA", "ARDOP", "M17", "Codec2 声码器", "ARQ"]) {
    await expect(page.getByText(name, { exact: true }).first()).toBeVisible();
  }
  // 关键事实：VARA 闭源、ARDOP / JS8Call 开源。
  await expect(page.getByText(/闭源但免费、仅 Windows/)).toBeVisible();
  await expect(page.getByText(/开源 ARQ 数据协议/)).toBeVisible();

  // 模式对比表与要点。
  await expect(page.getByRole("heading", { name: "模式对比" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "数据 · 约 50 Hz" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "数字语音 · 约 1.1 kHz" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "要点" })).toBeVisible();
  await expect(page.getByText(/它们不互通/)).toBeVisible();
});

test("专题页：现代数字模式三语界面标题与 section 标题有译文", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("locale", "en"));
  await page.goto("/modern-digital-modes");
  await expect(page.getByRole("heading", { name: "Modern digital modes" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Mode comparison" })).toBeVisible();
  // 专名原样（不译）。
  await expect(page.getByText("JS8Call", { exact: true }).first()).toBeVisible();
  // 表头译文。
  await expect(page.getByRole("columnheader", { name: "Type and bandwidth" })).toBeVisible();
});

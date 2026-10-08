import { expect, test, type Page } from "@playwright/test";

/** 1×1 的 PNG：省掉一个 fixture 文件，也足够验证「解码 → 缩放 → 转 JPEG」这条链路。 */
const PNG_1X1 = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==",
  "base64",
);

/** 呼号输入框没有 <label> 包裹，用 placeholder 定位。 */
function callInput(page: Page) {
  return page.locator('input[placeholder="BG4XXX"]');
}

/** 表单里的卡片影像缩略图（大图预览是另一个 img，用 first 取缩略图）。 */
function thumbnail(page: Page) {
  return page.locator('img[alt="卡片影像"]').first();
}

/**
 * 影像面板上的可见按钮。
 * 不能按 role 取：`FileInput` 里那个隐藏的 `<input type="file">` 也带同样的 aria-label，
 * `getByRole("button", { name })` 会同时命中两者。
 */
function panelButton(page: Page, text: string) {
  return page.locator("button").filter({ hasText: text });
}

test("QSL 卡片影像：上传、列表标记、刷新后仍在、可删除", async ({ page }) => {
  await page.goto("/log");
  await callInput(page).fill("JA1AA");
  await page.getByRole("button", { name: "添加记录" }).click();
  await expect(page.getByText("共 1 条")).toBeVisible();

  // 新记录还没有 id（影像按 id 关联），先给一句提示而不是空白。
  await expect(page.getByText("保存这条通联后即可上传扫描件")).toBeVisible();

  // 编辑已有记录 → 出现上传入口。
  await page.getByRole("button", { name: "编辑" }).first().click();
  await expect(page.getByText("保存这条通联后即可上传扫描件")).toHaveCount(0);
  await page.locator('input[type="file"][accept*="image"]').setInputFiles({
    name: "card.png",
    mimeType: "image/png",
    buffer: PNG_1X1,
  });

  // 落盘成功后才亮图（保存期间显示「正在保存…」），且统一转成 JPEG。
  await expect(thumbnail(page)).toBeVisible();
  await expect(thumbnail(page)).toHaveAttribute("src", /^data:image\/jpeg/);
  await expect(panelButton(page, "换一张")).toBeVisible();

  // 列表里出现「有影像」标记。
  await expect(page.getByRole("img", { name: "已存有卡片影像" })).toHaveCount(1);

  // 刷新后仍在：影像是 IndexedDB 里的，索引是 localStorage 里的小数据。
  await page.reload();
  await expect(page.getByRole("img", { name: "已存有卡片影像" })).toHaveCount(1);
  await page.getByRole("button", { name: "编辑" }).first().click();
  await expect(thumbnail(page)).toBeVisible();

  // 删除：确认后回到「上传扫描件」入口，标记也一起消失，刷新后不复现。
  page.once("dialog", (d) => d.accept());
  await panelButton(page, "删除影像").click();
  await expect(panelButton(page, "上传扫描件")).toBeVisible();
  await page.reload();
  await expect(page.getByRole("img", { name: "已存有卡片影像" })).toHaveCount(0);
});

test("QSL 卡片影像：非图片文件当场给出提示", async ({ page }) => {
  await page.goto("/log");
  await callInput(page).fill("W1AW");
  await page.getByRole("button", { name: "添加记录" }).click();
  await page.getByRole("button", { name: "编辑" }).first().click();

  await page.locator('input[type="file"][accept*="image"]').setInputFiles({
    name: "notes.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("not an image"),
  });
  await expect(page.getByText("只支持 JPEG / PNG / WebP 图片")).toBeVisible();
  await expect(page.getByRole("img", { name: "已存有卡片影像" })).toHaveCount(0);
});

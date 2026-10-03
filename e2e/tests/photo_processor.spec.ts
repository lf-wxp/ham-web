import { expect, test } from "./fixtures";

/** 1×1 的合法 PNG（会被放大到证件照最小尺寸）。 */
const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==",
  "base64",
);

test("照片处理：非图片文件被拦下并给出提示", async ({ page }) => {
  await page.goto("/photo-processor");
  await expect(page.getByRole("heading", { level: 1, name: "业余无线电报名照片处理" })).toBeVisible();

  // 校验在原生 alert 里：先挂上会自动确认的监听器，否则渲染进程被弹窗阻塞，setInputFiles 无法返回
  const seen: string[] = [];
  page.on("dialog", async (d) => {
    seen.push(d.message());
    await d.accept();
  });
  await page.locator("#id-file").setInputFiles({
    name: "not-an-image.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("hello"),
  });
  await expect.poll(() => seen).toContain("请选择有效的图片文件");

  // 被拦下时不进入处理态
  await expect(page.getByText(/正在处理照片/)).toHaveCount(0);
  await expect(page.getByText("选取证件照")).toBeVisible();
});

test("照片处理：上传图片后在本地完成处理并可下载", async ({ page }) => {
  await page.goto("/photo-processor");
  await expect(page.getByRole("heading", { level: 1, name: "业余无线电报名照片处理" })).toBeVisible();

  await page.locator("#id-file").setInputFiles({
    name: "photo.png",
    mimeType: "image/png",
    buffer: PNG,
  });

  await expect(page.getByText("处理完成！请右键或长按保存下方处理后的图片。")).toBeVisible();
  await expect(page.locator('img[alt="处理后的照片"]')).toBeVisible();
  // 结果信息四项
  await expect(page.getByText(/文件大小: \d+\.\d{2} KB/)).toBeVisible();
  await expect(page.getByText(/宽度: \d+px/)).toBeVisible();

  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "下载处理后的照片" }).click();
  const file = await dl;
  expect(file.suggestedFilename()).toMatch(/^photo_processed\.(jpeg|png)$/);

  await page.getByRole("button", { name: "处理其他照片" }).click();
  await expect(page.getByText("选取证件照")).toBeVisible();
});

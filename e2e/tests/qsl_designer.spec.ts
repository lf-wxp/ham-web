import { expect, pickOption, test, type Page } from "./fixtures";

/** 呼号输入框没有 <label> 包裹，用 placeholder 定位。 */
function callInput(page: Page) {
  return page.locator('input[placeholder="BG4XXX"]');
}

/** 屏幕上的预览图（打印页里还有一份同样的图，取第一张）。 */
function previewSrc(page: Page) {
  return page.locator('img[alt="QSL 卡片预览"]').first().getAttribute("src");
}

test("QSL 卡片设计：换版式、从日志带入、导出 PNG、打印页就绪", async ({ page }) => {
  // 打印按钮只断言「真的调了 window.print()」，不真的弹打印对话框。
  await page.addInitScript(() => {
    (window as unknown as { __printed: boolean }).__printed = false;
    window.print = () => {
      (window as unknown as { __printed: boolean }).__printed = true;
    };
  });

  // 先造一条通联（带对方网格），供设计器带入。
  await page.goto("/log");
  await callInput(page).fill("JA1AA");
  await page.locator('input[placeholder="OM89EW"]').fill("PM95AA");
  await page.getByRole("button", { name: "添加记录" }).click();
  await expect(page.getByText("共 1 条")).toBeVisible();

  await page.goto("/qsl-designer");
  await expect(page.getByRole("heading", { level: 1, name: "QSL 卡片设计" })).toBeVisible();

  // 预览已经画好（canvas → PNG dataURL）。
  await expect(page.locator('img[alt="QSL 卡片预览"]').first()).toHaveAttribute(
    "src",
    /^data:image\/png/,
  );
  const classic = await previewSrc(page);

  // 换版式：预览必须跟着变，且选中态落在新选项上（版式是互斥选择，用 radio 表达）。
  await page.getByRole("radio", { name: "醒目" }).click();
  await expect(page.getByRole("radio", { name: "醒目" })).toBeChecked();
  await expect
    .poll(async () => previewSrc(page))
    .not.toBe(classic);

  // 从日志带入一条通联：呼号 / 网格 / RST 一次填好。
  // 下拉是自定义弹层（`ui::NativeSelect`），选项文案带日期时间，所以用正则匹配呼号。
  await pickOption(page, "从日志填入一条通联", /JA1AA/);
  await expect(page.locator('input[placeholder="JA1XXX"]')).toHaveValue("JA1AA");
  // 网格也跟着带过来（不只是呼号）。
  await expect(page.locator('input[placeholder="PM95"]')).toHaveValue("PM95AA");

  // 导出 PNG：文件名带上对方呼号，方便一次导出多张。
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 PNG" }).click();
  expect((await download).suggestedFilename()).toBe("qsl-card-JA1AA.png");

  // 打印页：A4 居中一张 140×89mm 标准卡，内容与预览同一张图。
  const sheetCard = page.locator(".print-sheet img");
  await expect(sheetCard).toHaveAttribute("style", /140mm/);
  await expect
    .poll(async () => sheetCard.getAttribute("src"))
    .toBe(await previewSrc(page));

  const printed = page.waitForFunction(
    () => (window as unknown as { __printed: boolean }).__printed,
  );
  await page.getByRole("button", { name: "打印" }).click();
  await printed;
});

test("QSL 卡片设计：版式选择会记住", async ({ page }) => {
  await page.goto("/qsl-designer");
  await page.getByRole("radio", { name: "极简" }).click();
  await expect(page.getByRole("radio", { name: "极简" })).toBeChecked();

  await page.reload();
  await expect(page.getByRole("radio", { name: "极简" })).toBeChecked();
});

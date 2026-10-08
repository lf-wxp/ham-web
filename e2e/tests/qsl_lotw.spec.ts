import { expect, test, type Page } from "./fixtures";

/** 一条通联（与 LoTW 报告同键：呼号 + 日期 + 时间 + 波段 + 模式）。 */
const QSO = `Generated for e2e
<ADIF_VER:5>3.1.4 <EOH>
<CALL:5>JA1AA <QSO_DATE:8>20240501 <TIME_ON:4>1230 <BAND:3>20m <MODE:3>SSB <FREQ:6>14.200 <EOR>
`;

async function importQso(page: Page) {
  await page
    .locator('input[type="file"][accept*=".adi"]')
    .setInputFiles({
      name: "local.adi",
      mimeType: "text/plain",
      buffer: Buffer.from(QSO),
    });
  await expect(page.locator("main").getByRole("row").filter({ hasText: "JA1AA" })).toHaveCount(1);
}

test("LoTW：上传成功后回写「已上传」，计数与标志位跟着变", async ({ page }) => {
  // 确认框与成功提示都自动接受，顺便把文案收集起来断言。
  const dialogs: string[] = [];
  page.on("dialog", (d) => {
    dialogs.push(d.message());
    d.accept();
  });

  await page.goto("/log");
  await importQso(page);

  const mark = page.getByRole("button", { name: /标记已上传 LoTW/ });
  await expect(mark).toHaveText("标记已上传 LoTW（1）");

  await mark.click();
  await expect
    .poll(() => dialogs.some((m) => m.includes("已把 1 条通联标记为已上传 LoTW")))
    .toBe(true);
  // 没有可标记的了：计数归零、按钮禁用。
  await expect(mark).toHaveText("标记已上传 LoTW（0）");
  await expect(mark).toBeDisabled();

  // 状态真的落到记录上：编辑页里「LoTW 已上传」应已勾上。
  await page.getByRole("button", { name: "编辑" }).first().click();
  await expect(page.locator("main").getByLabel("LoTW 已上传")).toBeChecked();
});

test("LoTW 指引：/eqsl 上有完整步骤与「不代签」边界，同步框里能跳过去", async ({ page }) => {
  await page.goto("/eqsl");
  await expect(page.getByRole("heading", { name: "LoTW 签名上传流程" })).toBeVisible();
  // 六个环节都在：证书 → Station Location → 导出 → 签名 → .tq8 → 回写。
  await expect(page.getByText("申请呼号证书")).toBeVisible();
  await expect(page.getByText("在 TQSL 里建立 Station Location")).toBeVisible();
  await expect(page.getByText(/签名后的 \.tq8 文件/)).toBeVisible();
  await expect(page.getByText(/把状态回写进日志/)).toBeVisible();

  await expect(page.getByRole("heading", { name: "LoTW 的边界与常见坑" })).toBeVisible();
  // 服务端边界：只合并状态，不代签、不碰私钥。
  await expect(page.getByText(/本站不做代签/)).toBeVisible();
  await expect(page.getByText(/私钥只存在你自己的电脑上/)).toBeVisible();

  // 从日志页的同步对话框能直接跳到这份指引。
  await page.goto("/log");
  await page.getByRole("button", { name: "同步 QSL" }).click();
  const dialog = page.getByRole("dialog", { name: "同步 QSL 确认" });
  await expect(dialog.getByText(/还没有确认报告/)).toBeVisible();
  await expect(dialog.getByRole("link", { name: "TQSL 签名上传步骤" })).toHaveAttribute(
    "href",
    "/eqsl",
  );
});

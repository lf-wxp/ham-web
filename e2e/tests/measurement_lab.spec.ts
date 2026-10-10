import { expect, test } from "./fixtures";

test("测量实验室：馈线掩盖演示的数字随损耗联动", async ({ page }) => {
  await page.goto("/measurement-lab");
  await expect(page.getByRole("heading", { level: 1, name: "测量实验室" })).toBeVisible();

  // 演示 A：表头 3:1 时天线端真实值随馈线损耗变化（数学在核心有单测，这里钉联动）。
  // 默认损耗 0.5 dB → 表头 3:1 → 天线端 3.6。
  await expect(page.getByText(/表头读到 3.0 : 1 时，天线端约 3.6 : 1/)).toBeVisible();
  // 用键盘步进到 1.0 dB（range 输入的浮点步进用 fill 会撞 stepMismatch）。
  const loss = page.getByLabel("馈线单程损耗");
  await loss.focus();
  for (let i = 0; i < 5; i += 1) {
    await page.keyboard.press("ArrowRight");
  }
  await expect(page.getByText(/天线端约 4.4 : 1/)).toBeVisible();

  // 曲线图与方向性说明都在。
  await expect(page.getByRole("img", { name: "表头读数与天线端真实驻波" })).toBeVisible();
  await expect(page.getByText(/1.2 : 1 以下的读数不可信/)).toBeVisible();
  // 陷阱速查表。
  await expect(page.getByText("驻波 1:1 ≠ 好天线")).toBeVisible();
});

test("测量实验室：SOLT 校准分步演示", async ({ page }) => {
  await page.goto("/measurement-lab");
  // 第一步（Short）。
  await expect(page.getByText(/定标反射系数 -1 的参考点/)).toBeVisible();
  await page.getByRole("button", { name: "下一步" }).click();
  // 第二步（Open）。
  await expect(page.getByText(/定标 \+1 的参考点/)).toBeVisible();
  await page.getByRole("button", { name: "下一步" }).click();
  // 第三步（Load）。
  await expect(page.getByText(/直接测出方向性误差/)).toBeVisible();
});

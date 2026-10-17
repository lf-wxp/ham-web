import { expect, test } from "./fixtures";

test("专题页：国内业余卫星频率表与 409 MHz 合规对比", async ({ page }) => {
  await page.goto("/china-ham");
  await expect(page.getByRole("heading", { name: "国内业余卫星与 409 MHz 对讲机" })).toBeVisible();

  // 核心概念。
  await expect(page.getByText("希望二号（XW-2）", { exact: true })).toBeVisible();
  await expect(page.getByText("公众对讲机", { exact: true })).toBeVisible();

  // 希望二号转发器频率表：卫星名 + 上下行 + 信标。
  await expect(page.getByText("XW-2A（CAS-3A）")).toBeVisible();
  await expect(page.getByText("145.675 MHz")).toBeVisible();
  await expect(page.getByText("435.040 MHz")).toBeVisible();
  await expect(page.getByText("145.660 MHz")).toBeVisible();

  // 公众对讲机 vs 业余手台：频段、免执照、设备限制。
  await expect(page.getByText("409.7500–409.9875 MHz（12.5 kHz 间隔 20 个信道）")).toBeVisible();
  await expect(page.getByText("免执照、免频率使用许可")).toBeVisible();
  await expect(page.getByText("不得具备使用业余电台频段的能力")).toBeVisible();
  await expect(page.getByText("需《业余无线电台操作证书》+ 电台执照")).toBeVisible();
});

test("专题页：目录项存在且锚点直达", async ({ page }) => {
  await page.goto("/china-ham");
  await expect(page.getByRole("heading", { name: "国内业余卫星与 409 MHz 对讲机" })).toBeVisible();
  // 两个主要 section 标题。
  await expect(page.getByRole("heading", { name: "希望二号（XW-2）转发器频率" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "公众对讲机 vs 业余手台" })).toBeVisible();
});

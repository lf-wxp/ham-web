import { expect, test } from "./fixtures";

/**
 * 三条围绕目标 (39.9, 116.4) 的方位线，各距目标 5 km、间隔 120°，
 * 方位角都指向目标（正北 0°、顺时针）。无噪声时解应回到目标本身。
 */
const BEARINGS = [
  { lat: "39.944969", lon: "116.400000", az: "180" },
  { lat: "39.877520", lon: "116.450750", az: "300" },
  { lat: "39.877520", lon: "116.349250", az: "60" },
];

async function fillRow(
  page: import("@playwright/test").Page,
  row: number,
  value: { lat: string; lon: string; az: string },
) {
  await page.getByLabel(`第 ${row} 次测向的纬度`).fill(value.lat);
  await page.getByLabel(`第 ${row} 次测向的经度`).fill(value.lon);
  await page.getByLabel(`第 ${row} 次测向的方位角`).fill(value.az);
}

test("测向交汇：三条方位线交回目标点并给出网格", async ({ page }) => {
  await page.goto("/ardf");
  await expect(
    page.getByRole("heading", { level: 2, name: "测向交汇定位" }),
  ).toBeVisible();

  for (const [i, bearing] of BEARINGS.entries()) {
    await fillRow(page, i + 1, bearing);
  }

  // 目标纬度 39.9，解算值是 39.89999（平面近似的残差），钉到小数点后两位以内。
  // 锚在数字与单位上，不写死词条里的标点与量词：文案改动不该让功能用例变红。
  await expect(page.getByText(/纬度 39\.89/)).toBeVisible();
  await expect(page.getByText(/经度 116\.40/)).toBeVisible();
  await expect(page.getByText(/OM89/)).toBeVisible();
  await expect(page.getByText(/95%/)).toBeVisible();
  await expect(page.getByText(/残差 [\d.]+/)).toBeVisible();
});

test("测向交汇：只有一条时不硬算", async ({ page }) => {
  await page.goto("/ardf");
  await fillRow(page, 1, BEARINGS[0]);
  await expect(page.getByText("至少填两条方位线才能交汇。")).toBeVisible();
});

test("测向交汇：平行方位如实报无解", async ({ page }) => {
  await page.goto("/ardf");
  await fillRow(page, 1, { lat: "39.9", lon: "116.4", az: "45" });
  await fillRow(page, 2, { lat: "39.91", lon: "116.4", az: "45" });

  await expect(page.getByText(/没有唯一交点/)).toBeVisible();
});

test("测向交汇：补第 4 条时不会把已有结果换成错误", async ({ page }) => {
  await page.goto("/ardf");
  for (const [i, bearing] of BEARINGS.entries()) {
    await fillRow(page, i + 1, bearing);
  }
  await expect(page.getByText(/纬度 39\.89/)).toBeVisible();

  // 只在第 4 行填了纬度：这一行「还没填完」，不该把已经算出的交汇点顶成「输入无效」。
  await page.getByLabel("第 4 次测向的纬度").fill("39.9");
  await expect(page.getByText(/纬度 39\.89/)).toBeVisible();
});

test("测向交汇：越界坐标给出输入提示", async ({ page }) => {
  await page.goto("/ardf");
  await fillRow(page, 1, { lat: "100", lon: "116.4", az: "180" });
  await fillRow(page, 2, BEARINGS[1]);

  await expect(page.getByText(/纬度 -90～90/)).toBeVisible();
});

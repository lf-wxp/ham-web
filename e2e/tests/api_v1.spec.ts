import { expect, test } from "./fixtures";

test("开放 API：/developers 契约页列出计算端点，且示例请求真的可用", async ({ page }) => {
  await page.goto("/developers");
  // 端点的路径与示例链接都会出现，用 first 避开严格模式。
  for (const path of ["/api/v1/units/convert", "/api/v1/filter/design", "/api/v1/link-budget"]) {
    await expect(page.getByText(path).first()).toBeVisible();
  }
  // 示例链接指向带参数的真实端点。
  const example = "/api/v1/link-budget?tx_dbm=20&tx_gain_dbi=2&rx_gain_dbi=2&freq_mhz=146&dist_km=100";
  await expect(
    page.getByRole("link", { name: /api\/v1\/link-budget\?tx_dbm=20/ }),
  ).toBeVisible();

  // 契约页只是文档：示例里的请求必须真的能打通（e2e 的 webServer 就是这个服务端）。
  // 少了这一步，端点没挂上路由时页面照样是绿的。
  const response = await page.request.get(example);
  expect(response.ok()).toBe(true);
  expect(response.headers()["content-type"]).toContain("application/json");
  const body = await response.json();
  expect(body.meta.source).toBe("computed");
  expect(body.data.tx_dbm).toBe(20);
  expect(typeof body.data.rx_dbm).toBe("number");
});

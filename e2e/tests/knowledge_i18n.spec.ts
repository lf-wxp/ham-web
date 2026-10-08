import { expect, test } from "./fixtures";

// 已翻译模块（`data/knowledge-i18n/en/*.json`：wspr / antennas / bandplan / connectors）
// 随界面语言切换；尚未翻译的模块（如 qrp）自动回退中文，
// 不能因为词典加载失败而变成空白。
test("知识库正文：已翻译模块随界面语言切换", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("locale", "en"));
  await page.goto("/wspr");

  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  // WSPR_CONCEPTS 首条说明已译为英文
  await expect(page.getByText(/milliwatt levels/)).toBeVisible();
  // WSPR_NOTES 首条也已翻译
  await expect(page.getByText(/Transmit power as low as/)).toBeVisible();
});

test("知识库正文：未翻译模块回退中文", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("locale", "en"));
  // qrp 模块尚未翻译，页面应照常显示中文正文（而不是空白或报错）
  await page.goto("/qrp");

  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  const text = await page.locator("main").innerText();
  expect(text.length).toBeGreaterThan(50);
  expect(/[一-鿿]/.test(text)).toBe(true);
});

test("知识库正文：中文界面不触发译文加载", async ({ page }) => {
  const requests: string[] = [];
  page.on("request", (r) => {
    if (r.url().includes("knowledge-i18n")) requests.push(r.url());
  });
  await page.addInitScript(() => localStorage.setItem("locale", "zh"));
  await page.goto("/wspr");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await page.waitForTimeout(500);
  expect(requests).toEqual([]);
});

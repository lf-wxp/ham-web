import { expect, test } from "./fixtures";

test("主导航：展开考试中心下拉可跳转到错题集", async ({ page }) => {
  await page.goto("/");
  const nav = page.locator('[data-nav]');
  await expect(nav).toBeVisible();

  await nav.getByRole("button", { name: "考试中心" }).click();
  // 考试中心已改为分组下拉（练题 / 模拟考 / 复习 / 学习数据 / 闯关），错题集在「复习」里。
  await nav.getByRole("button", { name: "复习", exact: true }).click();
  await nav.locator('a[href="/mistakes"]').click();
  await expect(page).toHaveURL(/\/mistakes$/);
  await expect(page.getByRole("heading", { level: 1, name: "错题集" })).toBeVisible();
});

test("主导航：考试中心五个分组各自列出全部条目，共 17 项", async ({ page }) => {
  await page.goto("/");
  const nav = page.locator("[data-nav]");
  await nav.getByRole("button", { name: "考试中心" }).click();
  // 分组与条目取自注册表：新增 / 挪动页面时这里要同步，防止某个页面从导航里悄悄消失。
  const expected: Record<string, string[]> = {
    练题: ["/practice", "/daily-challenge", "/browse", "/flashcards", "/listen"],
    模拟考: ["/exam", "/exam-review"],
    复习: ["/cards", "/mistakes", "/mistake-topics", "/bookmarks"],
    学习数据: ["/weekly", "/progress", "/study-calendar", "/report"],
    闯关: ["/map", "/bestiary"],
  };
  let total = 0;
  for (const [group, hrefs] of Object.entries(expected)) {
    await nav.getByRole("button", { name: group, exact: true }).click();
    const links = nav.locator("#nav-exam-panel a");
    await expect(links).toHaveCount(hrefs.length);
    for (const href of hrefs) {
      await expect(nav.locator(`#nav-exam-panel a[href="${href}"]`)).toBeVisible();
    }
    total += hrefs.length;
  }
  expect(total).toBe(17);
});

test("数据备份入口：页脚、设置对话框都能直达 /backup", async ({ page }) => {
  // 页脚
  await page.goto("/glossary");
  await page.locator("footer").getByRole("link", { name: "数据备份" }).click();
  await expect(page).toHaveURL(/\/backup$/);
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();

  // 设置对话框（桌面顶栏的齿轮按钮无障碍名是「外观与动效」），点完应关闭对话框
  await page.goto("/");
  await page.locator("[data-nav]").getByRole("button", { name: "外观与动效" }).first().click();
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("link", { name: "数据备份" }).click();
  await expect(page).toHaveURL(/\/backup$/);
  await expect(dialog).toHaveCount(0);
});

test("数据备份入口：移动菜单可直达 /backup", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await page.locator("[data-nav]").getByRole("button", { name: /菜单|导航/ }).first().click();
  await page.locator('nav[aria-label*="导航"]').getByRole("link", { name: "数据备份" }).click();
  await expect(page).toHaveURL(/\/backup$/);
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();
});

test("旧链接 /tools#backup 转到 /backup，且回退键不会被弹回重定向", async ({ page }) => {
  await page.goto("/glossary");
  await page.goto("/tools#backup");
  await expect(page).toHaveURL(/\/backup$/);
  await expect(page.getByRole("heading", { level: 1, name: "数据备份" })).toBeVisible();
  // `replace` 导航：历史里不留 /tools#backup，回退直接回到上一页。
  await page.goBack();
  await expect(page).toHaveURL(/\/glossary$/);
});

test("主导航：展开工具下拉可跳转到小工具", async ({ page }) => {
  await page.goto("/");
  const nav = page.locator("[data-nav]");
  await nav.getByRole("button", { name: /工具/ }).first().click();
  await nav.locator('a[href="/tools"]').first().click();
  await expect(page).toHaveURL(/\/tools$/);
  await expect(page.getByRole("heading", { level: 1, name: "小工具" })).toBeVisible();
});

test("明暗主题：切换后写入本地偏好并作用于 <html>", async ({ page }) => {
  await page.goto("/");
  const toggle = page.getByRole("button", { name: "切换到深色模式" });
  await expect(toggle).toBeVisible();
  await toggle.click();

  await expect(page.locator("html")).toHaveClass(/dark/);
  await expect.poll(() => page.evaluate(() => localStorage.getItem("theme"))).toBe("dark");

  await page.getByRole("button", { name: "切换到浅色模式" }).click();
  await expect(page.locator("html")).not.toHaveClass(/dark/);
  await expect.poll(() => page.evaluate(() => localStorage.getItem("theme"))).toBe("light");

  // 偏好在刷新后保留
  await page.getByRole("button", { name: "切换到深色模式" }).click();
  await page.reload();
  await expect(page.locator("html")).toHaveClass(/dark/);
});

test("界面语言：切换后同步 <html lang> 并持久化", async ({ page }) => {
  await page.goto("/");
  // 语言切换后 aria-label 也会跟着翻译，这里用结构定位而非文案
  const combo = page.locator("[data-nav] [role=combobox]").first();
  await expect(combo).toBeVisible();
  // 选项文案（中文 / English / Español）本身不随界面语言变化
  const pick = async (name: string) => {
    await combo.click();
    await page.getByRole("option", { name, exact: true }).click();
  };

  await pick("English");
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect.poll(() => page.evaluate(() => localStorage.getItem("locale"))).toBe("en");

  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");

  await pick("中文");
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
});

test("页脚许可：站内点击无需刷新，刷新后保留页面骨架与主题", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await expect(page.locator("[data-nav]")).toBeVisible();
  await page.getByRole("button", { name: "切换到深色模式" }).click();
  const documentRequests: string[] = [];
  page.on("request", (request) => {
    if (request.isNavigationRequest() && request.frame() === page.mainFrame()) {
      documentRequests.push(request.url());
    }
  });
  await page.locator('footer a[href="/license.html"]').click();
  await expect(page).toHaveURL(/\/license\.html$/);
  await expect(page.getByRole("heading", { level: 1, name: "许可与对应源码" })).toBeVisible();
  await expect(page.locator("main .pxl-window")).toHaveCount(3);
  await expect(page.locator("html")).toHaveClass(/dark/);
  expect(documentRequests).toEqual([]);
  const cardColor = await page.locator("main .pxl-window").first().evaluate((element) => getComputedStyle(element).backgroundColor);
  await page.reload();
  await expect(page.getByRole("heading", { level: 1, name: "许可与对应源码" })).toBeVisible();
  await expect(page.locator("[data-nav]")).toBeVisible();
  await expect(page.locator("html")).toHaveClass(/dark/);
  expect(await page.locator("main .pxl-window").first().evaluate((element) => getComputedStyle(element).backgroundColor)).toBe(cardColor);
  // 第三方转写代码的许可文本必须随发布物提供（ft8_lib 为 MIT，KISS FFT 为 BSD-3-Clause）。
  for (const path of [
    "/licenses/LICENSE",
    "/licenses/GPL-3.0-only.txt",
    "/licenses/MIT-ft8_lib.txt",
    "/licenses/BSD-3-Clause-kissfft.txt",
  ]) {
    const link = page.locator(`main a[href="${path}"]`);
    await expect(link).toHaveAttribute("rel", "external");
    const response = await page.request.get(path);
    expect(response.ok()).toBe(true);
    expect(await response.text()).toMatch(
      /MIT License|GNU GENERAL PUBLIC LICENSE|Redistribution and use in source and binary forms/,
    );
  }
  expect(errors).toEqual([]);
});

for (const [locale, title] of [
  ["en", "Licenses and corresponding source"],
  ["es", "Licencias y código fuente correspondiente"],
]) {
  test(`许可页：直接访问的移动布局与 ${locale} 译文`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("locale", value), locale);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/license.html");
    await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
    await expect(page.locator("main .pxl-window")).toHaveCount(3);
    await expect(page.locator("main")).not.toContainText("组件许可");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
}

test("通联统计：空日志时引导到通联日志页", async ({ page }) => {
  await page.goto("/stats");
  await expect(page.getByRole("heading", { level: 1, name: "通联统计" })).toBeVisible();
  await expect(page.getByText("暂无通联日志。")).toBeVisible();

  await page.getByRole("link", { name: "去添加通联日志 →" }).click();
  await expect(page).toHaveURL(/\/log$/);
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
});

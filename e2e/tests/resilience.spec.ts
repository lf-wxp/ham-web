import { expect, test } from "./fixtures";

test("题库加载失败：给出可关闭的提示，而不是空白页", async ({ page }) => {
  await page.route(/\/questions\/.*\.json/, (route) => route.abort());
  await page.goto("/practice?bank=A");

  const dialog = page.getByRole("dialog", { name: "加载失败" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText("题库 A 暂不可用")).toBeVisible();
  await dialog.getByRole("button", { name: "知道了" }).click();
  await expect(dialog).toBeHidden();
  // 不应落到兜底页
  await expect(page.locator("#fatal")).toHaveCount(0);
});

test("本地数据损坏：各页面仍能渲染，不触发兜底页", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("mistake-book", "{ 这不是 JSON");
    localStorage.setItem("logbook", "oops");
    localStorage.setItem("study-stats", "[1,2,3]");
    localStorage.setItem("exam-history", "null");
    localStorage.setItem("countdowns", "{}");
    localStorage.setItem("bookmarks", "42");
  });

  for (const route of ["/mistakes", "/log", "/progress", "/weekly", "/stats", "/bookmarks"]) {
    await page.goto(route);
    await expect(page.locator("main h1").first()).toBeAttached({ timeout: 20_000 });
    await expect(page.getByText("未找到页面")).toHaveCount(0);
    await expect(page.locator("#fatal")).toHaveCount(0);
  }
});

test("实时接口失败：页面降级而不是崩溃", async ({ page }) => {
  await page.route("**/api/**", (route) =>
    route.fulfill({ status: 500, contentType: "text/plain", body: "upstream down" }),
  );
  await page.goto("/solar");
  await expect(page.locator("main h1").first()).toBeAttached();
  await expect(page.locator("#fatal")).toHaveCount(0);
  await expect(page.getByText("未找到页面")).toHaveCount(0);
});

test("存储写入失败：底部弹出提示条并可跳到备份页", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("main h1").first()).toBeAttached();

  await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent("ham-storage-write-failed", { detail: "logbook" }));
  });
  const banner = page.getByRole("alert").filter({ hasText: "本地存储空间已满" });
  await expect(banner).toBeVisible();

  await banner.getByRole("link", { name: "查看占用并备份" }).click();
  await expect(page).toHaveURL(/\/tools#backup$/);

  // 「知道了」可关闭
  await page.evaluate(() => {
    window.dispatchEvent(new CustomEvent("ham-storage-write-failed", { detail: "logbook" }));
  });
  await page.getByRole("alert").getByRole("button", { name: "知道了" }).click();
  await expect(page.getByRole("alert").filter({ hasText: "本地存储空间已满" })).toHaveCount(0);
});

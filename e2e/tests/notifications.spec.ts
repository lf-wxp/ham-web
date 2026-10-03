import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";

/** 开关行：<div class="flex items-center justify-between"><span>标题</span><button>已开启/已关闭</button></div> */
function row(page: Page, title: string) {
  return page.locator("main div").filter({ hasText: new RegExp(`^${title}已(开启|关闭)$`) }).last();
}

test("通知中心：权限被拒绝时显示已拒绝，仍保留请求入口", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(Notification, "permission", {
      value: "denied",
      configurable: true,
    });
  });
  await page.goto("/notifications");
  await expect(page.getByRole("heading", { level: 1, name: "通知中心" })).toBeVisible();
  await expect(page.getByRole("heading", { level: 2, name: "浏览器通知权限" })).toBeVisible();
  await expect(page.getByText("已拒绝")).toBeVisible();
  await expect(page.getByRole("button", { name: "请求权限" })).toBeVisible();
});

test("通知中心：已授权时不再显示请求按钮", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(Notification, "permission", {
      value: "granted",
      configurable: true,
    });
  });
  await page.goto("/notifications");
  await expect(page.getByRole("heading", { level: 2, name: "浏览器通知权限" })).toBeVisible();
  await expect(page.getByText("已授权")).toBeVisible();
  await expect(page.getByRole("button", { name: "请求权限" })).toHaveCount(0);
});

test("通知中心：DX 热点与卫星提醒开关即时持久化", async ({ page }) => {
  await page.goto("/notifications");
  await expect(page.getByRole("heading", { level: 2, name: "DX 热点提醒" })).toBeVisible();

  const dxcc = row(page, "新 DXCC 实体").getByRole("button");
  await expect(dxcc).toHaveText("已关闭");
  await dxcc.click();
  await expect(dxcc).toHaveText("已开启");
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("dx-alerts")))
    .toContain("\"new_dxcc\":true");

  const sat = row(page, "收藏卫星过境提醒").getByRole("button");
  await expect(sat).toHaveText("已关闭");
  await sat.click();
  await expect(sat).toHaveText("已开启");
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("sat-watch")))
    .toContain("\"alerts\":true");
});

test("通知中心：浏览器不支持 Web Push 时降级为说明文案", async ({ page }) => {
  // 去掉 PushManager，模拟不支持后台推送的环境
  await page.addInitScript(() => {
    delete (window as unknown as Record<string, unknown>).PushManager;
  });
  await page.goto("/notifications");
  await expect(page.getByRole("heading", { level: 2, name: "后台推送" })).toBeVisible();
  await expect(page.getByText("当前浏览器不支持 Web Push。")).toBeVisible();
  await expect(page.getByRole("button", { name: "订阅后台推送" })).toHaveCount(0);
});

test("通知中心：倒计时数量与倒计时页联动", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem(
      "countdowns",
      JSON.stringify({ events: [{ id: 1, title: "考试", target_ms: 4e12 }] }),
    );
  });
  await page.goto("/notifications");
  await expect(page.getByRole("heading", { level: 2, name: "倒计时提醒" })).toBeVisible();
  await expect(page.getByText("1 个倒计时")).toBeVisible();
  await expect(page.getByRole("link", { name: "管理倒计时 →" })).toBeVisible();
});

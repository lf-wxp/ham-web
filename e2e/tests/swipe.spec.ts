import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";

test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

/** 用 CDP 发送真实触摸序列，从 (x0, y) 划到 (x1, y)。 */
async function swipe(page: Page, x0: number, x1: number, y = 420) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: x0, y }] });
  for (let i = 1; i <= 5; i++) {
    const x = x0 + ((x1 - x0) * i) / 5;
    await cdp.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: [{ x, y }] });
  }
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await cdp.detach();
}

test("练习：左划下一题、右划上一题，短距离不触发", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  await swipe(page, 320, 80);
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();
  await swipe(page, 80, 320);
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  await swipe(page, 200, 170);
  await page.waitForTimeout(300);
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
});

test("闪卡：右划显示答案，再右划记为会", async ({ page }) => {
  await page.goto("/flashcards");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  await swipe(page, 80, 320, 300);
  await expect(page.getByRole("button", { name: "会 ✓" })).toBeVisible();
  await swipe(page, 80, 320, 300);
  await expect(page.getByText(/第 2 \/ \d+ 题/)).toBeVisible();
  await expect(page.getByText(/^会 1/)).toBeVisible();
});

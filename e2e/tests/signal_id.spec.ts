import { expect, test } from "./fixtures";

test("信号听辨：默认列出全部候选模式", async ({ page }) => {
  await page.goto("/signal-id");
  await expect(
    page.getByRole("heading", { level: 1, name: "信号听辨向导" }),
  ).toBeVisible();
  await expect(page.getByRole("heading", { level: 2 })).toContainText(
    "候选模式（17 个）",
  );
  await expect(
    page.getByRole("heading", { level: 3, name: "等幅电报 CW" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { level: 3, name: "LoRa" }),
  ).toBeVisible();
});

test("信号听辨：只留语音模式并链到模式专题", async ({ page }) => {
  await page.goto("/signal-id");
  await page.getByRole("radio", { name: "像人说话" }).click();

  await expect(page.getByRole("heading", { level: 2 })).toContainText(
    "候选模式（2 个）",
  );
  await expect(
    page.getByRole("heading", { level: 3, name: "SSB 单边带语音" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { level: 3, name: "FM 调频语音" }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "打开对应页面" }).first(),
  ).toHaveAttribute("href", "/modes");
});

test("信号听辨：周期突发 + 窄带只剩 WSJT 三个模式", async ({ page }) => {
  await page.goto("/signal-id");
  await page.getByRole("radio", { name: "周期性突发" }).click();
  await page.getByRole("radio", { name: "小于 250 Hz", exact: true }).click();

  await expect(page.getByRole("heading", { level: 2 })).toContainText(
    "候选模式（3 个）",
  );
  await expect(page.getByRole("heading", { level: 3, name: "FT8" })).toBeVisible();
  await expect(page.getByRole("heading", { level: 3, name: "WSPR" })).toBeVisible();
});

test("信号听辨：条件矛盾时给空态而不是猜一个", async ({ page }) => {
  await page.goto("/signal-id");
  await page.getByRole("radio", { name: "像人说话" }).click();
  await page.getByRole("radio", { name: "周期性突发" }).click();

  await expect(page.getByText(/当前条件没有匹配的模式/)).toBeVisible();
  await expect(page.getByRole("heading", { level: 3 })).toHaveCount(0);
});

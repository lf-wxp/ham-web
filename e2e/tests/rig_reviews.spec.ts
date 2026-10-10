import { expect, test } from "./fixtures";

test("评测文章：条件在前、指标与机型库同源、术语可互链", async ({ page }) => {
  await page.goto("/rig-reviews");
  await expect(page.getByRole("heading", { level: 1, name: "设备评测文章" })).toBeVisible();
  // 立场必须写在明处（条件前置 / 同源 / 主观与实测分开）。
  await expect(page.getByText(/条件在前/).first()).toBeVisible();

  // 第一篇评测是 FT-710。
  const ft710 = page.locator("article").filter({ hasText: "Yaesu FT-710" }).first();
  await expect(ft710).toBeVisible();
  // 指标实测与机型库同源：数值直接来自 Sherwood 表（107 dB），评测不抄一份自己的数字。
  await expect(ft710).toContainText("107 dB（2 kHz）");
  await expect(ft710.getByText(/Rob Sherwood \(NC0B\).*2026-10-08/)).toBeVisible();

  // 条件在前：文章里的前两个小节必须是「测试条件」→「指标实测」。
  const headings = await ft710.locator("h3").allTextContents();
  expect(headings[0]).toContain("测试条件");
  expect(headings[1]).toContain("指标实测");

  // 指标名互链到术语表，并带悬停释义（title = 术语表里那条的「含义」）。
  const metric = ft710.getByRole("link", { name: "窄间隔三阶互调动态范围（RMDR）" });
  await expect(metric).toHaveAttribute("href", "#glossary-0");
  await expect(metric).toHaveAttribute("title", /相位噪声/);
  await metric.click();
  await expect(page.locator("#glossary-0")).toBeVisible();
});

test("评测文章：没进实测表的机型如实留空，不找数字顶上", async ({ page }) => {
  await page.goto("/rig-reviews");
  const uv5r = page.locator("article").filter({ hasText: "Baofeng UV-5R" }).first();
  await expect(uv5r).toBeVisible();
  // 数值栏如实写「未收录」，条件栏也先说明这一点。
  await expect(uv5r).toContainText("未收录");
  await expect(uv5r.getByText(/不找别的数字顶上/)).toBeVisible();
  await expect(uv5r.getByRole("link", { name: "去机型库录入自己的实测值" })).toBeVisible();
});

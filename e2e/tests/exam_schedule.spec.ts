import { expect, test } from "./fixtures";

test("考试日程：渠道、核对清单与站内入口", async ({ page }) => {
  await page.goto("/exam-schedule");
  await expect(page.getByRole("heading", { level: 1, name: "考试日程" })).toBeVisible();

  // 页面首先要说清楚「本站没有日期」，否则用户会把渠道表当成排期表。
  await expect(page.locator("main")).toContainText("本站不发布考试排期");

  // 三个渠道都在，且只有 CRAC 带外链（省级机构与考点没有统一入口，靠文字说明怎么找）。
  for (const name of ["CRAC 官网", "省级无线电管理机构", "考点与本地协会"]) {
    await expect(page.getByText(name, { exact: true })).toBeVisible();
  }
  const crac = page.getByRole("link", { name: "打开官网" });
  await expect(crac).toHaveAttribute("href", "https://www.crac.org.cn/");
  await expect(crac).toHaveAttribute("target", "_blank");
  await expect(page.getByRole("link", { name: "打开官网" })).toHaveCount(1);

  // 核对清单：五项都要渲染出译文，漏挂词典会直接把 key 漏到界面上。
  await expect(page.getByText("公告对应的类别", { exact: true })).toBeVisible();
  await expect(page.getByText("报名截止与材料", { exact: true })).toBeVisible();
  await expect(page.getByText("考试方式与时长", { exact: true })).toBeVisible();
  await expect(page.getByText("考点与场次", { exact: true })).toBeVisible();
  await expect(page.getByText("成绩与证书", { exact: true })).toBeVisible();
  await expect(page.locator("main")).not.toContainText("learning.exam-schedule");

  // 三个站内入口：地址写死在这里，改核心数据表时这条会红。
  await expect(page.getByRole("link", { name: "去备考计划" })).toHaveAttribute(
    "href",
    "/study-calendar",
  );
  await expect(page.getByRole("link", { name: "去倒计时" })).toHaveAttribute("href", "/countdown");
  await expect(page.getByRole("link", { name: "去模拟考试" })).toHaveAttribute("href", "/exam");

  // 入口要真的能走：点「去倒计时」落到倒计时页，而不是只渲染出一个死链接。
  await page.getByRole("link", { name: "去倒计时" }).click();
  await expect(page).toHaveURL(/\/countdown$/);
  await expect(page.getByRole("heading", { level: 1, name: "倒计时与提醒" })).toBeVisible();
});

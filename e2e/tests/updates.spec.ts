import { expect, test } from "./fixtures";

test("更新提示：版本更新后展示本次更新内容，只展示一次", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toBeAttached();
  // 首次访问只记录版本，不打扰新用户
  await expect(page.getByText("已更新到新版本")).toHaveCount(0);

  await page.evaluate(() => localStorage.setItem("app:changelog-seen", "2000-01-01"));
  await page.reload();
  const notice = page.getByRole("status").filter({ hasText: "已更新到新版本" });
  await expect(notice).toBeVisible();
  await expect(notice.getByRole("listitem").first()).toBeVisible();
  await notice.getByRole("button", { name: "知道了" }).click();
  await expect(notice).toHaveCount(0);

  await page.reload();
  await expect(page.getByRole("heading", { level: 1 })).toBeAttached();
  await expect(page.getByText("已更新到新版本")).toHaveCount(0);
});

test("题库更新提示：内容与上次记录不同时列出改动", async ({ page }) => {
  // 必须在页面脚本运行前预置旧摘要：首页会预取 A 类题库，若先 goto 再写入，
  // 首页的加载会抢先把「当前内容」当成新摘要写回，等进练习页时已无差异。
  // 用哨兵键保证只播种一次，reload 时不会把旧摘要又塞回去。
  await page.addInitScript(() => {
    if (localStorage.getItem("e2e:bank-digest-seeded")) return;
    localStorage.setItem(
      "bank-digest:A",
      JSON.stringify({ rev: "", digest: { hashes: [1, 2, 3], explained: 0 } }),
    );
    localStorage.setItem("e2e:bank-digest-seeded", "1");
  });

  await page.goto("/practice?bank=A");
  const notice = page.getByRole("status").filter({ hasText: "题库已更新" });
  await expect(notice).toBeVisible();
  await expect(notice).toContainText(/A 类：修改 3 题，新增 \d+ 题.*现共 \d+ 题/);
  await notice.getByRole("button", { name: "知道了" }).click();

  // 记录已更新为当前内容，再次进入不再提示
  await page.reload();
  await expect(page.getByText(/第 \d+ \/ \d+ 题/).first()).toBeVisible();
  await expect(page.getByText("题库已更新")).toHaveCount(0);
});

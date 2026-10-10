import { expect, test } from "./fixtures";

test("法规原文库：条文可直达、题目反向链接到专项练习", async ({ page }) => {
  await page.goto("/radio-law");
  await expect(page.getByRole("heading", { level: 1, name: "法规原文库" })).toBeVisible();
  // 出处必须写出来：令号 + 官方来源链接（条文是法律文本，没有出处就不该收录）。
  await expect(page.getByText(/第67号/)).toBeVisible();
  // 两部法律各带一条官方来源链接。
  await expect(page.getByRole("link", { name: "官方来源" })).toHaveCount(2);
  // 原文照录声明。
  await expect(page.getByText(/原文照录/).first()).toBeVisible();

  // 条款级锚点直达第三十条（A / B / C 类的频段与功率）。
  await page.goto("/radio-law#办法-30");
  const a30 = page.locator("#办法-30");
  await expect(a30).toContainText("1000瓦");

  // 反向关联：题库里引用它的题目（人工维护的映射，A-57/58/59）。
  await expect(a30.getByText("题库里引用这条的题目：")).toBeVisible();
  const link = a30.getByRole("link", { name: /A-58/ });
  await expect(link).toHaveAttribute("href", "/practice?sub=1.3.2");
  await link.click();
  await expect(page).toHaveURL(/practice\?sub=1\.3\.2/);
});

test("法规原文库：没有映射题目的条文不显示题目栏", async ({ page }) => {
  await page.goto("/radio-law");
  // 办法第 2 条没有人工映射（第 1 条已经有「制定机构」那题了）。
  const a2 = page.locator("#办法-2");
  await expect(a2).toBeVisible();
  await expect(a2.getByText("题库里引用这条的题目：")).toHaveCount(0);
});

test("法规原文库：《条例》已收录，罚则条文可直达并反向链接", async ({ page }) => {
  await page.goto("/radio-law");
  await expect(page.getByRole("heading", { name: "《中华人民共和国无线电管理条例》" })).toBeVisible();
  await expect(page.getByText(/第672号/)).toBeVisible();

  // 第七十条（擅自设台罚则）锚点直达 + 反向关联的题目。
  await page.goto("/radio-law#条例-70");
  const a70 = page.locator("#条例-70");
  await expect(a70).toContainText("20万元以上50万元以下");
  const link = a70.getByRole("link", { name: /A-149/ });
  await expect(link).toHaveAttribute("href", "/practice?sub=1.6.2");

  // 两部法律的条号锚点互不干扰：两本都有「第30条」，但锚点各归各的（id 不重复）。
  await page.goto("/radio-law#办法-30");
  await expect(page.locator("#办法-30")).toContainText("1000瓦");
  await expect(page.locator("#办法-30")).toHaveCount(1);
  await expect(page.locator("#条例-30")).toHaveCount(1);
  await expect(page.locator("#条例-30")).toContainText("15瓦以上的短波无线电台");
});

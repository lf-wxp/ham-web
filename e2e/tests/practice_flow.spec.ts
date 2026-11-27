import type { Page } from "@playwright/test";
import { expect, focusRingSlack, option, test } from "./fixtures";

/** 底部栏统计行：「题库 A 类 · 顺序练习 · 进度 1 / 100」。
 *  操作栏经 `Portal` 挂到 `body`（避免被路由容器的入场 `transform` 影响 `fixed` 定位），
 *  因此不再位于 `<main>` 内，不能再用 `locator("main")` 限定。 */
function bar(page: Page) {
  return page.getByText(/题库 [ABC] 类 ·/).first();
}

/** 筛选开关是 `ChipToggle`，选中态由 `aria-pressed` 表达。 */
async function isOn(locator: ReturnType<Page["getByRole"]>) {
  return (await locator.getAttribute("aria-pressed")) === "true";
}

test("练习：切换题库会改写地址栏并换题", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await expect(bar(page)).toContainText("题库 A 类");
  await expect(bar(page)).toContainText("顺序练习");

  await page.getByRole("radio", { name: "B 类" }).first().click();
  await expect(page).toHaveURL(/bank=B/);
  await expect(bar(page)).toContainText("题库 B 类");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();

  await page.getByRole("radio", { name: "C 类" }).first().click();
  await expect(page).toHaveURL(/bank=C/);
  await expect(bar(page)).toContainText("题库 C 类");
});

test("练习：只看本类新增 / 只练没做过可切换", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();

  const unseen = page.getByRole("button", { name: "只练没做过" });
  expect(await isOn(unseen)).toBe(false);
  await unseen.click();
  expect(await isOn(unseen)).toBe(true);
  // 全新用户没有做过的题，题量不变
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();

  await unseen.click();
  expect(await isOn(unseen)).toBe(false);

  const unique = page.getByRole("button", { name: "只看本类新增" });
  await unique.click();
  expect(await isOn(unique)).toBe(true);
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
});

test("练习：搜索题目可跳转，未命中时给出提示", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();

  await page.keyboard.press("Enter");
  const search = page.getByRole("dialog", { name: "搜索题目" });
  await expect(search).toBeVisible();
  // 搜索面板不挂右上角关闭按钮：点遮罩 / Esc / 「跳转」都能关，多一个 × 只是多一条路。
  await expect(search.getByRole("button", { name: "关闭" })).toHaveCount(0);
  await search.getByRole("textbox").first().fill("天线");
  await expect(search.getByText(/匹配 \d+ 条/)).toBeVisible();
  await search.locator("li").filter({ hasText: /第 \d+ 题/ }).first().click();
  await expect(search).toBeHidden();
  await expect(page.getByText(/第 \d+ \/ \d+ 题/).first()).toBeVisible();

  // 不存在的关键词：给出空提示而不是空白
  await page.keyboard.press("Enter");
  await search.getByRole("textbox").first().fill("这个词肯定不存在");
  await expect(search.getByText("未找到匹配")).toBeVisible();
});

// 回归：输入框被弹层里的 `overflow-auto` 容器包着，焦点环（2px 虚线 + 2px 偏移 = 盒子外 4px）
// 此前容器只留 4px 内距 —— 环外沿正好压在裁剪边界上，分数 DPR 的手机上左/上沿被切掉。
// 现改为 6px（与 `ui::DialogBody` 同一手法），必须留有盈余（不是「刚好贴边」）。
test("练习：搜索题目的输入框焦点环不被滚动容器裁掉", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", { name: "搜索题目" });
  await expect(dialog).toBeVisible();
  const input = dialog.getByRole("textbox").first();
  await input.focus();
  await expect(input).toBeFocused();
  const ring = await focusRingSlack(input);
  expect(ring.ring).toBeGreaterThan(0);
  expect(ring.slack).toBeGreaterThanOrEqual(1);
});

test("练习：切到随机题序且有作答时先确认", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await option(page, "A").click();

  // 页内「设置」而不是顶栏的「通用设置」：`main` 限定 + `exact` 才唯一。
  await page.locator("main").getByRole("button", { name: "设置", exact: true }).click();
  const settings = page.getByRole("dialog", { name: "设置", exact: true });
  await expect(settings).toBeVisible();
  await settings.getByRole("radio", { name: "随机" }).click();

  const confirm = page.getByRole("dialog", { name: "切换题序将清空作答" });
  await expect(confirm).toBeVisible();
  await confirm.getByRole("button", { name: "确定切换" }).click();
  await expect(confirm).toBeHidden();
  await expect(bar(page)).toContainText("随机练习");
});

test("练习：收藏后进入收藏集，再取消收藏", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await page.getByRole("button", { name: "收藏" }).first().click();

  await page.goto("/bookmarks");
  await expect(page.getByRole("heading", { level: 1, name: "收藏集" })).toBeVisible();
  await expect(page.getByText("共 1 题")).toBeVisible();

  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await page.getByRole("button", { name: "收藏" }).first().click();

  await page.goto("/bookmarks");
  await expect(page.getByText("暂无收藏")).toBeVisible();
});

test("练习：?multi=1 只看多选题且强制随机序", async ({ page }) => {
  await page.goto("/practice?bank=A&multi=1");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await expect(bar(page)).toContainText("随机练习");
  await expect(page.getByText("多选", { exact: true }).first()).toBeVisible();
});

test("练习：顶部「只练多选」开关可切换多选专项", async ({ page }) => {
  await page.goto("/practice?bank=A");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await page.getByRole("button", { name: "只练多选" }).click();
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await expect(page.getByText("多选", { exact: true }).first()).toBeVisible();
});

test("练习：专项链接（?topic=）按分类过滤并强制随机序", async ({ page }) => {
  await page.goto("/practice?bank=A&topic=%E5%A4%A9%E7%BA%BF");
  await expect(page.getByText(/第 1 \/ \d+ 题/).first()).toBeVisible();
  await expect(bar(page)).toContainText("随机练习");
});

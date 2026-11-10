import type { Page } from "@playwright/test";
import { expect, test } from "./fixtures";

const SEARCH = "搜索题干 / 答案 / 解析…";

/** 读取顶部「当前筛选」统计卡的数值（不受分页上限影响）。 */
async function filteredCount(page: Page) {
  // Stat 卡结构：<div>数值</div><div>当前筛选</div>
  const card = page.getByText("当前筛选", { exact: true }).locator("..");
  return Number((await card.innerText()).replace(/\D/g, ""));
}

test("分类浏览：搜索命中过滤，无结果给出空提示", async ({ page }) => {
  await page.goto("/browse");
  await expect(page.getByRole("heading", { level: 1, name: "题库分类浏览" })).toBeVisible();
  const rows = page.locator("main article");
  await expect(rows.first()).toBeVisible();

  const before = await filteredCount(page);
  await page.getByPlaceholder(SEARCH).fill("天线");
  await expect.poll(() => filteredCount(page)).toBeLessThan(before);
  expect(await filteredCount(page)).toBeGreaterThan(0);
  await expect(rows.first()).toBeVisible();

  await page.getByPlaceholder(SEARCH).fill("这个词肯定不存在");
  await expect(page.getByText("没有匹配的题目")).toBeVisible();
  await expect(rows).toHaveCount(0);
});

test("分类浏览：?q= 预填关键词", async ({ page }) => {
  await page.goto("/browse?q=%E9%A9%BB%E6%B3%A2%E6%AF%94");
  await expect(page.getByPlaceholder(SEARCH)).toHaveValue("驻波比");
  await expect(page.locator("main article").first()).toBeVisible();
});

test("分类浏览：只看多选后每道题都带多选徽标", async ({ page }) => {
  await page.goto("/browse");
  await expect(page.locator("main article").first()).toBeVisible();
  await page.getByRole("button", { name: "只看多选" }).click();

  const rows = page.locator("main article");
  await expect(rows.first()).toBeVisible();
  const count = await rows.count();
  expect(count).toBeGreaterThan(0);
  await expect(rows.getByText("多选", { exact: true })).toHaveCount(count);
});

test("分类浏览：一级分类筛选与只看本类新增", async ({ page }) => {
  await page.goto("/browse");
  const rows = page.locator("main article");
  await expect(rows.first()).toBeVisible();

  await page.locator("aside").getByRole("button", { name: "天线与馈线" }).click();
  await expect.poll(() => rows.count()).toBeGreaterThan(0);
  await expect(rows.first().getByText("天线与馈线")).toBeVisible();

  // 回到全部
  await page.locator("aside").getByRole("button", { name: "全部题目" }).click();
  await expect(rows.first()).toBeVisible();

  // B 类「只看本类新增」会排除与 A 类重合的题，题量不应增加
  await page.getByRole("radio", { name: "B 类" }).first().click();
  await expect(rows.first()).toBeVisible();
  const allB = await rows.count();
  await page.getByRole("button", { name: "只看本类新增" }).click();
  await expect.poll(() => rows.count()).toBeLessThanOrEqual(allB);
});

test("分类浏览：卡片可就地收藏，取消后从收藏集消失", async ({ page }) => {
  // 无障碍名恒为「收藏题目 {题号}」：状态只交给 aria-pressed，
  // 所以同一个定位器在点击前后都命中；可见文字仍在「收藏 / 已收藏」间切换。
  const BOOKMARK = /^收藏题目 \S+/;
  await page.goto("/browse");
  const rows = page.locator("main article");
  const first = rows.first();
  const second = rows.nth(1);
  await expect(second).toBeVisible();

  const add = first.getByRole("button", { name: BOOKMARK });
  await expect(add).toHaveAttribute("aria-pressed", "false");
  await expect(add).toHaveText("收藏");
  await add.click();
  await expect(add).toHaveAttribute("aria-pressed", "true");
  await expect(add).toHaveText("已收藏");
  // 收藏状态按题目 id 派生：点第一题不能连带第二题变色
  await expect(second.getByRole("button", { name: BOOKMARK })).toHaveAttribute(
    "aria-pressed",
    "false",
  );

  await page.goto("/bookmarks");
  await expect(page.getByText("共 1 题", { exact: true })).toBeVisible();

  // 回到浏览页：收藏状态从存储里恢复，取消后收藏集回到空态
  await page.goto("/browse");
  const again = rows.first().getByRole("button", { name: BOOKMARK });
  await expect(again).toHaveAttribute("aria-pressed", "true");
  await again.click();
  await expect(again).toHaveAttribute("aria-pressed", "false");

  await page.goto("/bookmarks");
  await expect(page.getByText("暂无收藏")).toBeVisible();
});

// 题目卡（分类浏览与收藏集共用）在两套皮肤下各跑一遍；`E2E_UI_STYLE` 对本文件不生效，必须显式切皮肤。
for (const style of ["pixel", "modern"] as const) {
  test(`分类浏览（${style}）：标题里写出正确选项字母，与标出的正确项一一对应`, async ({
    page,
  }) => {
    await page.addInitScript((s) => {
      if (window !== window.top) return;
      localStorage.setItem("ui:uiStyle", s);
    }, style);
    await page.goto("/browse");
    const rows = page.locator("main article");
    await expect(rows.first()).toBeVisible();

    // 颜色之外必须有文字：读屏 / 色觉障碍 / 灰度打印只能靠标题里的字母认出哪几项是对的
    let multi = 0;
    for (let i = 0; i < 8; i++) {
      const card = rows.nth(i);
      const right = card.locator('[data-right="true"]');
      const letters = await right.evaluateAll((els) =>
        els.map((e) => (e.textContent ?? "").trim().charAt(0)),
      );
      expect(letters.length, `第 ${i + 1} 张卡没有标出正确项`).toBeGreaterThan(0);
      if (letters.length > 1) multi += 1;

      const head = (await card.getByText(/正确答案/).first().innerText()).replace(/\s+/g, "");
      expect(head, `第 ${i + 1} 张卡标题里的字母`).toContain(letters.join("、"));
      expect(head.match(/[A-D]/g)?.length ?? 0).toBe(letters.length);
    }
    expect(multi, "前 8 张里至少有一道多选，才覆盖到「共 N 项」的写法").toBeGreaterThan(0);
  });

  test(`分类浏览（${style}）：收藏按钮的无障碍名带题号`, async ({ page }) => {
    await page.addInitScript((s) => {
      if (window !== window.top) return;
      localStorage.setItem("ui:uiStyle", s);
    }, style);
    await page.goto("/browse");
    const rows = page.locator("main article");
    await expect(rows.nth(1)).toBeVisible();
    // 同屏有很多张卡：名字不带题号，读屏听到的就是一排一模一样的「收藏本题」
    const names = await rows
      .locator("button[aria-pressed]")
      .evaluateAll((els) => els.slice(0, 5).map((e) => e.getAttribute("aria-label")));
    expect(names.every((n) => /^收藏题目 \S+/.test(n ?? ""))).toBe(true);
    expect(new Set(names).size).toBe(names.length);
  });

  test(`收藏（${style}）：其它标签页改了收藏，本页的题目卡与收藏集列表跟着变`, async ({
    page,
  }) => {
    const context = page.context();
    await context.addInitScript((s) => {
      if (window !== window.top) return;
      localStorage.setItem("ui:uiStyle", s);
    }, style);
    const BOOKMARK = /^收藏题目 \S+/;

    // 本页停在收藏集（空），另一个标签页在浏览页收藏第一题
    await page.goto("/bookmarks");
    await expect(page.getByText("暂无收藏")).toBeVisible();
    const other = await context.newPage();
    await other.goto("/browse");
    const otherRow = other.locator("main article").first();
    await expect(otherRow).toBeVisible();
    await otherRow.getByRole("button", { name: BOOKMARK }).click();
    await expect(page.locator("main article")).toHaveCount(1);

    // 本页换到浏览页：第一题已是收藏态；另一个标签页再取消，本页要跟着翻回去
    await page.goto("/browse");
    const mine = page.locator("main article").first().getByRole("button", { name: BOOKMARK });
    await expect(mine).toHaveAttribute("aria-pressed", "true");
    await otherRow.getByRole("button", { name: BOOKMARK }).click();
    await expect(mine).toHaveAttribute("aria-pressed", "false");
  });

  test(`分类浏览（${style}）：知识点与参考依据默认收起，展开才出现`, async ({ page }) => {
    await page.addInitScript((s) => {
      if (window !== window.top) return;
      localStorage.setItem("ui:uiStyle", s);
    }, style);
    // 首题是 A 类 1.1.1（有知识点与参考依据），折叠头必然存在
    await page.goto("/browse");
    const card = page.locator("main article").first();
    await expect(card).toBeVisible();
    const toggle = card.getByRole("button", { name: "知识点与参考依据" });
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    // `aria-controls` 必须指向一个真实存在、且全页唯一的面板（像素风曾漏写面板的 id，
    // 悬空引用在收起态下不会被任何别的断言碰到）
    const panelId = await toggle.getAttribute("aria-controls");
    expect(panelId, "折叠头没有声明它控制的面板").toBeTruthy();
    await expect(page.locator(`[id="${panelId}"]`)).toHaveCount(1);

    // 收起时正文根本没构建（不是靠样式裁掉）：列表一屏几十张卡，不该为用户不看的文字付渲染成本
    const refsLabel = card.getByText("参考依据", { exact: true });
    await expect(refsLabel).toHaveCount(0);

    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    await expect(refsLabel).toBeVisible();

    // 收起后正文留在 DOM 里给高度过渡，但必须真的不可见（`visibility: hidden`），
    // 否则被裁掉的法条链接还能被 Tab 聚焦
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    await expect(refsLabel).toBeHidden();
  });
}

test("分类浏览：加载更多分页", async ({ page }) => {
  await page.goto("/browse");
  const rows = page.locator("main article");
  await expect(rows.first()).toBeVisible();

  const more = page.getByRole("button", { name: /加载更多/ });
  await expect(more).toBeVisible();
  const before = await rows.count();
  await more.click();
  await expect.poll(() => rows.count()).toBeGreaterThan(before);
});

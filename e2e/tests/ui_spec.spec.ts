import type { Page } from "@playwright/test";
import { expect, option, test } from "./fixtures";

// 界面评审整改的回归：用例标题里的编号对应 `docs/ui-components.md` §2 的硬规则，改动时能对上。

/** 打开只出多选题的练习页（`?multi=1` 强制随机序，第一题必是多选）。 */
async function openMultiOnlyPractice(page: Page) {
  await page.goto("/practice?multi=1");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  await expect(page.getByText("多选", { exact: true }).first()).toBeVisible();
}

test("A2 多选题：勾第一个框不亮出答案，点「检查答案」才判分", async ({ page }) => {
  await openMultiOnlyPractice(page);
  await expect(page.getByText(/^正确答案：/)).toHaveCount(0);

  await option(page, "A").click();
  // 勾上了，但还没检查：答案行不能出现，否则后面的框照抄即可。
  await expect(page.getByText(/^正确答案：/)).toHaveCount(0);
  const check = page.getByRole("button", { name: "检查答案" });
  await expect(check).toBeVisible();

  await check.click();
  await expect(page.getByText(/^正确答案：/)).toBeVisible();
  await expect(check).toHaveCount(0);
  // 判分结论有文字通道（不只是颜色）：要么「已答对」要么「答错了」。
  await expect(page.getByText(/已答对|答错了/).first()).toBeVisible();
});

test("B1 判分后：答案行在 live region 里，且选项行带 ✓ 图标标出正确项", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText(/第 1 \/ \d+ 题/)).toBeVisible();
  await page.getByRole("button", { name: "显示正确答案" }).click();
  // 答案行住在常驻的 role=status 里，读屏用户作答后能听到结果。
  const live = page.getByRole("status").filter({ hasText: /^正确答案：/ });
  await expect(live).toBeVisible();
  // 至少一个选项行被标成正确项（绿色边 + 行尾 ✓ 图标，图标是 aria-hidden 的 svg）。
  await expect(page.locator("main .border-win svg").first()).toBeVisible();
});

test("A1 模态打开时数字键不再改答案", async ({ page }) => {
  await page.goto("/exam");
  await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
  await page.getByRole("button", { name: "答题卡" }).first().click();
  await expect(page.getByRole("dialog", { name: "答题卡" })).toBeVisible();

  // 答题卡开着时按 1：背后的题不该被作答。
  await page.keyboard.press("1");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "答题卡" })).toBeHidden();
  await expect(page.getByText(/已作答\s*0\s*\/\s*40/).first()).toBeVisible();

  // 关掉之后快捷键恢复。
  await page.keyboard.press("1");
  await expect(page.getByText(/已作答\s*1\s*\/\s*40/).first()).toBeVisible();
});

test("B5 答题卡：当前题带 aria-current，图例作答中也常驻", async ({ page }) => {
  await page.goto("/exam");
  await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
  await page.getByRole("button", { name: "答题卡" }).first().click();
  const card = page.getByRole("dialog", { name: "答题卡" });
  await expect(card).toBeVisible();

  const cells = card.locator("div.grid button");
  await expect(cells.first()).toHaveAttribute("aria-current", "true");
  await expect(card.locator('button[aria-current="true"]')).toHaveCount(1);
  // 图例以前只有交卷后才出现，作答中那个小黄块的含义要靠猜。
  await expect(card.getByText("当前题目")).toBeVisible();
  await expect(card.getByText("未作答")).toBeVisible();
  // 筛选按钮是 aria-pressed 开关。
  await expect(card.getByRole("button", { name: "全部", exact: true })).toHaveAttribute("aria-pressed", "true");
});

test("B6 机考仿真：上一题与答题卡跳转如实置灰", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("exam:strict", "1"));
  await page.goto("/exam");
  // 机考仿真开考前先过一遍规则演练，确认后才开始。
  const rules = page.getByRole("dialog", { name: "考场规则演练" });
  await expect(rules).toBeVisible();
  await rules.getByRole("button", { name: "我已了解，开始机考" }).click();
  await expect(rules).toBeHidden();
  await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
  await option(page, "A").click();
  await page.getByRole("button", { name: "下一题", exact: true }).first().click();
  await expect(page.getByText(/第 2 \/ 40 题/).first()).toBeVisible();

  await expect(page.getByRole("button", { name: "上一题", exact: true }).first()).toBeDisabled();
  await page.getByRole("button", { name: "答题卡" }).first().click();
  const card = page.getByRole("dialog", { name: "答题卡" });
  await expect(card.getByRole("note")).toContainText("不能回看");
  await expect(card.locator("div.grid button").first()).toBeDisabled();
});

test("B8 Toast：悬停时不消失（暂停计时）", async ({ page }) => {
  await page.goto("/backup");
  // 导出会弹「已导出 N 项」的成功 Toast（2.4s 后自动消失）。
  const download = page.waitForEvent("download").catch(() => undefined);
  await page.getByRole("button", { name: /导出/ }).first().click();
  await download;
  const toast = page.locator('[data-slot="toast"]');
  await expect(toast).toBeVisible();
  await toast.hover();
  // 超过成功类 Toast 的默认停留（2.4s）+ 退场动画，悬停中的仍在。
  await page.waitForTimeout(3500);
  await expect(toast).toBeVisible();
  // 移开后再留约 2 秒，然后消失。
  await page.mouse.move(0, 0);
  await expect(toast).toBeHidden({ timeout: 8000 });
});

test("E1 首页：首屏只有一个最重的主操作，模拟考试入口不重复", async ({ page }) => {
  await page.goto("/");
  const hero = page.locator("main section[aria-labelledby=home-title]");
  await expect(hero.getByRole("link", { name: "出去闯关" })).toBeVisible();
  await expect(hero.getByRole("link", { name: "怪物图鉴" })).toBeVisible();
  // 「挑战 Boss」(/exam) 与下面快速开始里的「开始模拟考试」是同一去向，只留后者。
  await expect(hero.getByRole("link", { name: "挑战 Boss" })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "开始模拟考试" })).toBeVisible();
});

test("E2 首页模块卡：条目是各自的真链接", async ({ page }) => {
  await page.goto("/");
  const link = page.locator("main").getByRole("link", { name: "错题集", exact: true });
  await expect(link.first()).toHaveAttribute("href", "/mistakes");
  await link.first().click();
  await expect(page).toHaveURL(/\/mistakes$/);
});

test("E3 移动菜单：三大段可折叠，当前页所在的段默认展开", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/practice");
  await page.locator("[data-nav]").getByRole("button", { name: /菜单|导航/ }).first().click();
  const nav = page.getByRole("navigation", { name: "移动端导航" });
  await expect(nav).toBeVisible();
  const sections = nav.locator("details");
  await expect(sections).toHaveCount(3);
  // /practice 属于考试中心：只有这一段展开。
  await expect(sections.nth(0)).toHaveAttribute("open", "");
  await expect(sections.nth(1)).not.toHaveAttribute("open", "");
  // 点开知识库：里面的入口出现。
  await sections.nth(1).locator("summary").click();
  await expect(sections.nth(1)).toHaveAttribute("open", "");
  await expect(nav.locator('[aria-current="page"]')).toHaveCount(1);
});

test("E4 底部 Tab：激活项除了字色还有顶部主色条", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  const tabs = page.getByRole("navigation", { name: "底部导航" });
  const home = tabs.getByRole("link", { name: "首页" });
  await expect(home).toHaveAttribute("aria-current", "page");
  const border = await home.evaluate((el) => getComputedStyle(el).borderTopWidth);
  expect(border).toBe("4px");
});

// ---------------------------------------------------------------------------------------------
// F：首页下半部分的卡片统一（SectionCard）。默认（无存档、无后端）渲染的是这四张。

const HOME_CARDS = ["知识卡片", "实时追台建议", "每日一题", "实时传播条件"] as const;
/** 整卡可点的两张：标题是铺满整张卡的链接。 */
const LINKED_CARDS = { 实时追台建议: "/most-wanted", 实时传播条件: "/solar" } as const;

const homeCard = (page: Page, name: string) =>
  page
    .locator("main section.pxl-card")
    .filter({ has: page.getByRole("heading", { level: 2, name, exact: true }) });

test("F1 首页下半部分卡片：同一套外壳（2px 墨框、无投影、像素标题栏），卡片间距一致", async ({ page }) => {
  // 关动效：滚动浮现的位移会干扰量间距。
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/");

  const boxes: { top: number; bottom: number }[] = [];
  for (const name of HOME_CARDS) {
    const card = homeCard(page, name);
    await card.scrollIntoViewIfNeeded();
    await expect(card).toBeVisible();
    // 标题栏是 `pxl-section-title`，标题是 h2（以前有的卡片标题只是一个 div）。
    await expect(card.locator(":scope > .pxl-section-title h2")).toHaveText(name);
    const style = await card.evaluate((el) => {
      const s = getComputedStyle(el);
      return { shadow: s.boxShadow, border: s.borderTopWidth, radius: s.borderTopLeftRadius };
    });
    // L1 平卡：只有 2px 墨框，没有硬投影（投影是 L2 窗口的），零圆角。
    expect(style, name).toEqual({ shadow: "none", border: "2px", radius: "0px" });
    boxes.push(await card.evaluate((el) => {
      const r = el.getBoundingClientRect();
      return { top: r.top + window.scrollY, bottom: r.bottom + window.scrollY };
    }));
  }
  // 相邻卡片的间距恒为 24px（`space-y-6`）。以前卡片自带 `mb-[24px]`、追台卡返回空 `<div>`，
  // 间距时 24 时 48。
  for (let i = 1; i < boxes.length; i++) {
    expect(Math.round(boxes[i].top - boxes[i - 1].bottom), `${HOME_CARDS[i - 1]} → ${HOME_CARDS[i]}`).toBe(24);
  }
});

test("F2 首页卡片：悬停有反馈 = 整卡可点；不可点的卡片悬停不动", async ({ page }) => {
  await page.goto("/");
  const shadowOf = (name: string) => homeCard(page, name).evaluate((el) => getComputedStyle(el).boxShadow);

  for (const [name, href] of Object.entries(LINKED_CARDS)) {
    const card = homeCard(page, name);
    await card.scrollIntoViewIfNeeded();
    await expect(card.locator("a[data-card-link]")).toHaveAttribute("href", href);
    await card.hover({ position: { x: 24, y: 24 } });
    // `motion-lift`：硬投影出现，并且上抬 2px（一个 art-pixel；以前追台卡是 4px + 主色软阴影）。
    await expect.poll(() => shadowOf(name), name).not.toBe("none");
    // 位移走独立的 `translate` 属性（不是 `transform`）：与入场动画的 `transform` 互不覆盖，
    // 也和 `pxl-btn` 的上抬/下沉同一套实现。
    await expect
      .poll(() => card.evaluate((el) => getComputedStyle(el).translate), `${name} 上抬`)
      .toBe("0px -2px");
  }
  for (const name of ["知识卡片", "每日一题"]) {
    const card = homeCard(page, name);
    await card.scrollIntoViewIfNeeded();
    await expect(card.locator("a[data-card-link]")).toHaveCount(0);
    await card.hover({ position: { x: 24, y: 24 } });
    await page.waitForTimeout(400);
    expect(await shadowOf(name), name).toBe("none");
  }
});

for (const [name, href] of Object.entries(LINKED_CARDS)) {
  test(`F3 首页「${name}」：点卡片的空白处也能跳转（标题链接铺满整卡）`, async ({ page }) => {
    // 关动效：滚动浮现的位移会让 `boundingBox` 在动画中途取到偏移量，坐标就不准了。
    // 以前用 `waitForTimeout(600)` 等它落位，机器慢时仍会飘。
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/");
    const card = homeCard(page, name);
    await card.scrollIntoViewIfNeeded();
    const box = (await card.boundingBox())!;
    // 左下角的内边距：不是标题、不是任何控件。
    await page.mouse.click(box.x + 8, box.y + box.height - 8);
    await expect(page).toHaveURL(new RegExp(`${href}$`));
  });
}

test("F4 首页整卡可点的卡片：键盘焦点画在整张卡上（虚线），而不是标题那几个字", async ({ page }) => {
  await page.goto("/");
  const card = homeCard(page, "实时传播条件");
  const link = card.locator("a[data-card-link]");
  await link.focus();
  await expect(link).toBeFocused();
  const [cardOutline, linkOutline] = await Promise.all([
    card.evaluate((el) => getComputedStyle(el).outlineStyle),
    link.evaluate((el) => getComputedStyle(el).outlineStyle),
  ]);
  expect(cardOutline).toBe("dashed");
  expect(linkOutline).toBe("none");
});

test("F5 首页卡片的标题栏元信息是响应式的：异步数据到了会出现，切语言会跟着变", async ({ page }) => {
  await page.goto("/");
  // 追台卡的传播等级徽标依赖异步拉到的太阳数据：同页「实时传播条件」能显示数据，
  // 这里就必须也出现徽标（`{m.run()}` 只执行一次时它永远不会出现）。
  const wanted = homeCard(page, "实时追台建议");
  await expect(wanted.locator(":scope > .pxl-section-title .pxl-badge")).toContainText("传播");

  // 切到 English：每日一题的连续天数、追台卡的徽标都要跟着换语言。
  const combo = page.locator("[data-nav] [role=combobox]").first();
  await combo.click();
  await page.getByRole("option", { name: "English", exact: true }).click();
  const daily = page.locator("main section.pxl-card").filter({ has: page.getByRole("heading", { level: 2, name: "Daily Question", exact: true }) });
  await expect(daily.locator(":scope > .pxl-section-title")).toContainText(/streak|day/i);
  await expect(daily.locator(":scope > .pxl-section-title")).not.toContainText("打卡");
});

import type { Page } from "@playwright/test";
import { bankQuestions, expect, mistakeRecord, option, test } from "./fixtures";

// 界面评审整改的回归：用例标题里的编号对应 `docs/README.md` §2 的硬规则，改动时能对上。

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
  await expect(hero.getByRole("link", { name: "出发通联" })).toBeVisible();
  await expect(hero.getByRole("link", { name: "干扰图鉴" })).toBeVisible();
  // 「呼叫稀有台」(/exam) 与下面快速开始里的「开始模拟考试」是同一去向，只留后者。
  await expect(hero.getByRole("link", { name: "呼叫稀有台" })).toHaveCount(0);
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
  // 追台卡的传播等级徽标依赖 `/api/solar`。真实后端只在联调时存在，
  // 不 mock 的话这条用例在其他环境里必然超时（与界面无关的假失败）。
  await page.route("**/api/solar", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        source: "N0NBH / HamQSL",
        updated: "31 Oct 2026 1200 GMT",
        solar_flux: 118,
        a_index: 7,
        k_index: 2,
        sunspots: 62,
        xray: "C1.2",
        conditions: [
          { band: "80m-40m", time: "day", condition: "Good" },
          { band: "30m-20m", time: "day", condition: "Fair" },
          { band: "80m-40m", time: "night", condition: "Poor" },
          { band: "30m-20m", time: "night", condition: "Good" },
        ],
      }),
    }),
  );
  await page.goto("/");
  // 追台卡的传播等级徽标依赖异步拉到的太阳数据：同页「实时传播条件」能显示数据，
  // 这里就必须也出现徽标（`{m.run()}` 只执行一次时它永远不会出现）。
  const wanted = homeCard(page, "实时追台建议");
  await expect(wanted.locator(":scope > .pxl-section-title .pxl-badge")).toContainText("传播");

  // 切到 English：每日一题的连续天数、追台卡的徽标都要跟着换语言。
  // 语言在齿轮打开的设置对话框里（顶栏没有语言下拉了），选完 Esc 关掉。
  await page.locator("[data-nav]").getByRole("button", { name: "通用设置" }).first().click();
  const settings = page.getByRole("dialog");
  await settings.getByRole("radio", { name: "English", exact: true }).click();
  await page.keyboard.press("Escape");
  await expect(settings).toBeHidden();
  const daily = page.locator("main section.pxl-card").filter({ has: page.getByRole("heading", { level: 2, name: "Daily Question", exact: true }) });
  await expect(daily.locator(":scope > .pxl-section-title")).toContainText(/streak|day/i);
  await expect(daily.locator(":scope > .pxl-section-title")).not.toContainText("打卡");
});

// 双风格运行时隔离的回归在 `dual_isolation.spec.ts`（带「等挂载 + 正向对照 + 表面必须打开」三道护栏）。

test("G1 窄屏页头：只留标题，副标题与操作区折进「更多选项」", async ({ page }) => {
  // 页头是吸顶的，而副标题 + 操作区（题库 / 搜索 / 两枚筛选）在 390px 下折成三四行，
  // 钉住就是长期占掉三分之一视口。规则与实测数据见 `docs/ui/pixel.md` §4「窄屏折叠页头」。
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "pixel");
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/browse");
  const header = page.locator("[data-page-header]");
  const nav = page.locator("[data-nav]");
  const more = page.getByRole("button", { name: "更多选项" });
  const search = page.getByPlaceholder("搜索题干 / 答案 / 解析…");
  const headerHeight = () => header.evaluate((el) => Math.round(el.getBoundingClientRect().height));
  await expect(header).toBeVisible();
  await expect(nav).toBeVisible();

  // 页头仍然吸顶（不是靠取消吸顶来省地方）
  await expect(header).toHaveCSS("position", "sticky");
  // 收起时页头只有一行标题（实测 50px；副标题若漏在外面会到 76px 以上）
  await expect(more).toHaveAttribute("aria-expanded", "false");
  await expect.poll(headerHeight).toBeLessThan(64);
  // 折叠区里的控件在 DOM 里但不可见 / 不可聚焦
  await expect(search).toBeHidden();

  await more.click();
  await expect(more).toHaveAttribute("aria-expanded", "true");
  await expect(search).toBeVisible();
  // 副标题 + 三组控件都回来了（实测 212px）
  await expect.poll(headerHeight).toBeGreaterThan(150);

  // 焦点环（2px 虚线 + 3px 偏移 = 5px）画在控件外面，而折叠区靠 `overflow: hidden` 收起：
  // 控件贴着折叠区边缘，环就被裁掉。量折叠区正文与控件包络之间的间距，左 / 右 / 下都得留满 5px
  // （修复前左、下实测 0）。等展开过渡走完再量：`getBoundingClientRect` 量的是此刻的盒子。
  await expect
    .poll(
      () =>
        page.evaluate(() => {
          const body = document.querySelector("[data-page-header] [id^='page-head-actions'] > div")!;
          const box = body.getBoundingClientRect();
          const rects = [...body.querySelectorAll("button, input, select, a, [role='combobox']")]
            .map((el) => el.getBoundingClientRect())
            .filter((r) => r.width > 0 && r.height > 0);
          // 量不到控件就返回 -1：让「折叠区里没控件」直接红，而不是空转通过
          if (rects.length === 0) return -1;
          return Math.min(
            Math.min(...rects.map((r) => r.left)) - box.left,
            box.right - Math.max(...rects.map((r) => r.right)),
            box.bottom - Math.max(...rects.map((r) => r.bottom)),
          );
        }),
      { message: "折叠区正文与控件之间没给焦点环留够 5px" },
    )
    .toBeGreaterThanOrEqual(5);

  // 展开后滚下去页头依旧钉在导航下沿
  await page.evaluate(() => window.scrollTo(0, 500));
  await expect
    .poll(() => header.evaluate((el) => Math.round(el.getBoundingClientRect().top)))
    .toBe(66);
  expect(Math.round(await nav.evaluate((el) => el.getBoundingClientRect().top))).toBe(0);

  // 桌面端：没有「更多选项」，操作区直接在标题行右侧（`md:contents` 还原折叠前的版式）
  await page.setViewportSize({ width: 1280, height: 844 });
  await expect(more).toBeHidden();
  await expect(search).toBeVisible();
});

test("G2 窄屏页头：每日挑战的倒计时条常显，不跟着「更多选项」收起", async ({ page }) => {
  // `children` 里的实时状态用 `PageHeader` 的 `keep_rows` 留在折叠区外面。
  // 折进去就等于答题时看不见还剩多少时间 —— 见 `docs/ui/pixel.md` §4「窄屏折叠页头」。
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "pixel");
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/daily-challenge");
  await page.getByRole("button", { name: "开始挑战" }).click();

  const header = page.locator("[data-page-header]");
  const more = page.getByRole("button", { name: "更多选项" });
  // 折叠区仍是收起的（副标题与题库筛选在里面），倒计时条却在外面
  await expect(more).toHaveAttribute("aria-expanded", "false");
  await expect(header.getByText("剩余时间")).toBeVisible();
  await expect(header.getByRole("button", { name: "交卷" })).toBeVisible();

  // 滚下去也还看得见（页头吸顶）
  await page.evaluate(() => window.scrollTo(0, 600));
  await expect(header.getByText("剩余时间")).toBeVisible();
});

test("G3 窄屏页头：SDR 瀑布图的「开始」常显，不必先点「更多选项」", async ({ page }) => {
  // `actions` 里页面唯一的主操作用 `PageHeader` 的 `keep_actions` 留在折叠区外面：
  // 监听中想停下来，不该先去找一个收起的菜单 —— 见 `docs/ui/pixel.md` §4「窄屏折叠页头」。
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "pixel");
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/sdr-waterfall");

  const header = page.locator("[data-page-header]");
  const more = page.getByRole("button", { name: "更多选项" });
  const start = header.getByRole("button", { name: "开始", exact: true });
  // 折叠区仍收起（里面只剩副标题），主操作却在外面，且全页只有这一份
  await expect(more).toHaveAttribute("aria-expanded", "false");
  await expect(start).toBeVisible();
  await expect(start).toHaveCount(1);

  // 桌面端同样在标题行右侧，不因为常显而多出一份
  await page.setViewportSize({ width: 1280, height: 844 });
  await expect(more).toBeHidden();
  await expect(start).toBeVisible();
  await expect(start).toHaveCount(1);
});

test("H1 通联路线：窄屏下「下一站」的文字块不被压成一字宽", async ({ page }) => {
  // 这一行里同时有 `flex-shrink: 0` 的精灵与两个 `Size::Lg` 按钮。文字块曾写 `flex-1`
  // （`basis: 0%`）+ `min-w-0`，可以被压到 0 宽 —— flex 认为「放得下」，`flex-wrap`
  // 不触发，中文就逐字竖排成一列（390px 下实测 1 字宽、标题高 20 多行）。
  // 判据用「文字块宽度」与「标题行数」：只看是否横向溢出抓不到这种纵向塌缩。
  // 宽度取 600（不是 320/390）：窄到放不下时 `flex-wrap` 会把按钮组换行、文字块反而拿满宽度；
  // 真正塌缩的是「精灵 + 按钮刚好挤得下」的那一档（用户报的那张图是 600 左右）。
  await page.setViewportSize({ width: 600, height: 844 });
  await page.goto("/map");

  const title = page.locator("#next-quest-title");
  await expect(title).toBeVisible();
  const metrics = await title.evaluate((el) => {
    const rect = el.getBoundingClientRect();
    const lineHeight = Number.parseFloat(getComputedStyle(el).lineHeight) || 18;
    return { width: rect.width, lines: rect.height / lineHeight };
  });
  // 一行能放下至少 8 个中文字符，且标题不超过 2 行。
  expect(metrics.width).toBeGreaterThan(144);
  expect(metrics.lines).toBeLessThanOrEqual(2);
});

test("H2 窄屏宽表：表格按住最小宽度横滚，不把列压成一字宽", async ({ page }) => {
  // 三列以上的表要是不给 `min-w-[…]`，窄屏会让列均分剩下的宽度：
  // `/rig-reviews` 在 320px 下曾把「测法」列压到 55px、8 行/字（表格内部横滚才是预期行为）。
  await page.setViewportSize({ width: 320, height: 844 });
  await page.goto("/rig-reviews");

  const table = page.locator("table").first();
  await expect(table).toBeVisible();
  const width = await table.evaluate((el) => el.getBoundingClientRect().width);
  // 640px = `min-w-[40rem]`：三列各留 200px 上下的可读宽度。
  expect(width).toBeGreaterThanOrEqual(600);
});

test("H3 顶栏一排控件等高：站标 / 波段键 / 搜索 / 设置 / HUD 顶边与底边对齐", async ({ page }) => {
  // 像素风顶栏里的控件都带 2px 墨框，差一档高度一眼就看得出。此前是三种高度并存
  // （站标与右侧按钮 40px 顶到行边、波段键 36px、HUD 32px）。判据用几何而不是类名：
  // 同一排的顶边 / 底边必须一致；桌面档（≥1280）与窄屏档各测一次，两档高度本来就不同。
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "pixel");
  });
  for (const [width, expected] of [
    [1440, 36],
    [390, 40],
  ] as const) {
    await page.setViewportSize({ width, height: 800 });
    await page.goto("/");
    const nav = page.locator("[data-nav]");
    await expect(nav).toBeVisible();
    const boxes = await nav.evaluate((el) => {
      const pick = (sel: string) =>
        [...el.querySelectorAll<HTMLElement>(sel)].filter((n) => n.offsetWidth > 0);
      return [
        el.querySelector<HTMLElement>('a[href="/"]')!,
        ...pick(".pxl-band-key"),
        ...pick(".ml-auto [data-slot='button']"),
        pick('[data-slot="hud"]')[0]!.parentElement!,
      ]
        .filter(Boolean)
        .map((n) => {
          const r = n.getBoundingClientRect();
          return { top: Math.round(r.top), bottom: Math.round(r.bottom), h: Math.round(r.height) };
        });
    });
    expect(boxes.length).toBeGreaterThan(2);
    const tops = new Set(boxes.map((b) => b.top));
    const bottoms = new Set(boxes.map((b) => b.bottom));
    expect([...tops], `@${width} 顶边不齐：${JSON.stringify(boxes)}`).toHaveLength(1);
    expect([...bottoms], `@${width} 底边不齐：${JSON.stringify(boxes)}`).toHaveLength(1);
    expect(boxes[0]!.h, `@${width} 控件高度不是 ${expected}px：${JSON.stringify(boxes)}`).toBe(expected);
  }
});

test("H4 首页：三列模块卡片与「今日学习队列」之间有间距", async ({ page }) => {
  // 首页把 `PageContainer` 的 `space-y-6` 关成了 `space-y-0`（各块自己写 `mt-6` 分段）。
  // 模块卡片行与下方功能卡片区当初漏了 `mt-6`，两块上下墨框直接贴在一起（2026-11-14 修）。
  // 判据用容器几何而不是卡片：卡片带 `reveal` 入场位移（`translateY(8px)`），量它会量到动到一半的位置。
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "pixel");
    localStorage.setItem("theme", "light");
  });
  // 「今日学习队列」只在有复习项时渲染：预置一条到期错题，让它真的出现在版式里。
  const records = bankQuestions("A").slice(0, 3).map((q) => mistakeRecord(q));
  await page.addInitScript((book) => {
    if (window !== window.top) return;
    if (!localStorage.getItem("mistake-book")) localStorage.setItem("mistake-book", book);
  }, JSON.stringify({ records }));
  for (const width of [1280, 1000, 390] as const) {
    await page.setViewportSize({ width, height: 1000 });
    await page.goto("/");
    await expect(page.getByText("今日学习队列")).toBeVisible({ timeout: 30_000 });
    // 量容器几何；入场动效落定前读数可能偏，所以用 `poll` 等到满足（真的没间距会超时失败），不写固定等待。
    await expect
      .poll(
        () =>
          page.evaluate(() => {
            const stack = [...document.querySelectorAll<HTMLElement>("div")].find(
              (d) => /(^|\s)space-y-6(\s|$)/.test(d.className) && d.querySelector(".pxl-card"),
            );
            const row = stack?.previousElementSibling as HTMLElement | null;
            if (!stack || !row) return -1;
            return Math.round(stack.getBoundingClientRect().top - row.getBoundingClientRect().bottom);
          }),
        { message: `@${width} 模块卡片与下方卡片区之间没有间距` },
      )
      .toBeGreaterThanOrEqual(16);
  }
});

test("I1 表单语义：aria-describedby 不留空属性、不悬空，也不指向别的表单行的说明", async ({ page }) => {
  // `Field` 靠 context 把说明 / 错误节点的 id 下发给控件。context 若写在组件体里（而不是 `Provider`），
  // 后一个 `Field` 会覆盖前一个、排在 `Field` 之后的独立控件还会读到前一行的 id —— 都是读屏才听得出来的错。
  // 判据只看 DOM：每个 `field-hint-*` / `field-error-*` 必须属于控件所在的那一行。
  for (const url of ["/tools", "/backup"]) {
    await page.goto(url);
    await expect(page.locator("#route-content[data-sig]")).toBeAttached({ timeout: 45_000 });
    await expect(page.locator('[id^="field-hint"]').first()).toBeAttached({ timeout: 30_000 });
    const bad = await page.evaluate(() => {
      const out: string[] = [];
      const row = (el: Element) => el.closest(".space-y-1\\.5, .md-field");
      for (const el of document.querySelectorAll("[aria-describedby]")) {
        const v = el.getAttribute("aria-describedby") ?? "";
        const tag = `${el.tagName.toLowerCase()}#${el.id}`;
        if (v.trim() === "") out.push(`${tag} 空的 aria-describedby`);
        for (const id of v.split(/\s+/).filter(Boolean)) {
          const target = document.getElementById(id);
          if (!target) out.push(`${tag} 指向不存在的 #${id}`);
          else if (/^field-(hint|error)-/.test(id) && row(target) !== row(el)) {
            out.push(`${tag} 指向别的表单行的 #${id}`);
          }
        }
      }
      return out;
    });
    expect(bad, `${url} 的 aria-describedby 有问题`).toEqual([]);
  }
});

test("H5 首页知识频谱：每根柱的点击目标不小于 24px（320 / 360 / 390）", async ({ page }) => {
  // WCAG 2.5.8：10 根柱挤一行时，320px 下每列只剩约 20px；窄屏改成 5 列两行后每根都 >= 24px。
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "pixel");
  });
  for (const width of [320, 360, 390] as const) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    const bars = page.locator("a.pxl-spectrum-bar");
    await expect(bars).toHaveCount(10);
    const sizes = await bars.evaluateAll((els) =>
      els.map((el) => {
        const r = el.getBoundingClientRect();
        return { w: Math.round(r.width), h: Math.round(r.height) };
      }),
    );
    for (const s of sizes) {
      expect(s.w >= 24 && s.h >= 24, `@${width} 频谱柱 ${JSON.stringify(s)} 小于 24px`).toBe(true);
    }
  }
});

test("H6 成绩弹窗：设了呼号才有「显示我的呼号」开关，且默认不勾选", async ({ page }) => {
  // 导出的成绩图是要发给别人的，呼号能反查到个人信息：不能默认印上去（`docs/ui/decisions.md` §11.6）。
  const submit = async () => {
    await page.getByRole("button", { name: "交卷", exact: true }).first().click();
    await page.getByRole("dialog", { name: "确认交卷？" }).getByRole("button", { name: "确认交卷" }).click();
    return page.getByRole("dialog", { name: "成绩" });
  };
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "pixel");
    localStorage.setItem(
      "station-info",
      JSON.stringify({ callsign: "BG4XXX", operator: "", gridsquare: "", rig: "", antenna: "" }),
    );
  });
  await page.goto("/exam");
  await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
  const result = await submit();
  const toggle = result.getByRole("checkbox", { name: /显示我的呼号/ });
  await expect(toggle).toHaveAttribute("aria-checked", "false");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-checked", "true");
});

test("H7 成绩弹窗：没设呼号时没有「显示我的呼号」开关", async ({ page }) => {
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "pixel");
  });
  await page.goto("/exam");
  await expect(page.getByText(/第 1 \/ 40 题/).first()).toBeVisible();
  await page.getByRole("button", { name: "交卷", exact: true }).first().click();
  await page.getByRole("dialog", { name: "确认交卷？" }).getByRole("button", { name: "确认交卷" }).click();
  const result = page.getByRole("dialog", { name: "成绩" });
  await expect(result).toBeVisible();
  await expect(result.getByRole("checkbox", { name: /显示我的呼号/ })).toHaveCount(0);
});

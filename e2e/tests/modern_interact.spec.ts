import type { Page } from "@playwright/test";
import { expect, settleFrames, test } from "./fixtures";

// 现代风的**交互面**回归：结构对了不等于能用。
// 覆盖调谐台、下拉、确认弹层、移动端菜单四条主链路。

async function seed(page: Page) {
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "modern");
    localStorage.setItem("theme", "dark");
  });
}

test("调谐台：⌘K 打开 → 搜索 → ↑↓ 选择 → Enter 跳转 → Esc 关闭", async ({ page }) => {
  await seed(page);
  await page.goto("/");
  await expect(page.locator(".md-nav").first()).toBeVisible();

  await page.keyboard.press("Meta+k");
  await expect(page.locator(".md-palette")).toBeVisible();

  // 键盘事件只会落到**当前焦点**上：调谐台的输入框是在挂载后的 `requestAnimationFrame`
  // 里才拿到焦点的，面板刚可见时焦点可能还在 `body`（机器忙时必然发生过 —— 键入全丢，
  // 面板停在 40 条全量列表）。先等到焦点在输入框上，后面的键入才测得到过滤；
  // 这条等待同时就是「打开即聚焦」的产品断言。
  await expect(page.locator(".md-palette-input")).toBeFocused();
  await page.keyboard.type("工具");
  await expect(page.locator(".md-palette").first()).toContainText("工具");

  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/tools/);

  await page.keyboard.press("Escape");
  await expect(page.locator(".md-palette")).toHaveCount(0);
});

test("下拉：可展开、↑↓ 可移动、Enter 选中并收起", async ({ page }) => {
  await seed(page);
  await page.goto("/log");
  const combo = page.locator('main [role="combobox"]').nth(5);
  await combo.scrollIntoViewIfNeeded();
  const before = (await combo.textContent())?.trim() ?? "";

  await combo.click();
  await expect(page.locator('[role="listbox"]')).toBeVisible();

  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page.locator('[role="listbox"]')).toHaveCount(0);

  const after = (await combo.textContent())?.trim() ?? "";
  expect(after).not.toBe(before);
});

test("确认弹层：打开后 Esc 关闭，DOM 移除", async ({ page }) => {
  await seed(page);
  await page.goto("/bookmarks");
  await page.getByRole("textbox").first().fill("临时组");
  await page.getByRole("button", { name: "新建分组" }).first().click();
  await page.getByRole("button", { name: "删除该分组" }).first().click();

  const panel = page.locator(".md-modal-panel");
  await expect(panel).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
});

test("移动端菜单：dock 打开 → 点条目跳转 → 面板自动收起", async ({ page }) => {
  await seed(page);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await page.locator('.md-dock button:has-text("更多")').click();
  await expect(page.locator(".md-menu-panel")).toBeVisible();

  await page.locator('.md-menu-panel a[href="/browse"]').first().click();
  await expect(page).toHaveURL(/\/browse$/);
  await expect(page.locator(".md-menu-panel")).toHaveCount(0);
});

// 目录搜索框里的「全站搜索」胶囊必须收在 40px 字段的边框之内。
//
// 回归：`pointer: coarse` 把 `.md-chip` 抬到 44（触控命中区），但没排除**嵌在字段里**的那颗，
// 于是它从字段上下各顶出 2px。上面那条移动端用例量不出来 —— `setViewportSize` 只改宽度，
// 不给粗指针，`(pointer: coarse)` 不成立，胶囊仍是 32px。这里必须 `hasTouch` 才能走到那条规则。
test.describe("触屏（粗指针）", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  test("目录搜索框：全站搜索胶囊不越过字段边框", async ({ page }) => {
    await seed(page);
    await page.goto("/");
    await page.locator('.md-dock button:has-text("更多")').click();
    await expect(page.locator(".md-menu-panel")).toBeVisible();

    const r = await page.locator(".md-menu-search").evaluate((field) => {
      const btn = field.querySelector(".md-chip");
      if (!btn) throw new Error("目录搜索框里没有胶囊");
      const fs = getComputedStyle(field);
      const fr = field.getBoundingClientRect();
      const br = btn.getBoundingClientRect();
      // 量的是**边框内沿**：胶囊压住 1px 描边也算越界。
      return {
        coarse: matchMedia("(pointer: coarse)").matches,
        fieldH: fr.height,
        gapTop: br.top - (fr.top + parseFloat(fs.borderTopWidth)),
        gapBottom: fr.bottom - parseFloat(fs.borderBottomWidth) - br.bottom,
        gapRight: fr.right - parseFloat(fs.borderRightWidth) - br.right,
      };
    });

    // 先证明跑在粗指针下：否则测的是没被抬高的桌面版式，用例恒真、等于没测。
    expect(r.coarse, "用例没在粗指针环境里跑，测不到抬档规则").toBe(true);
    expect(r.fieldH, "字段没量到高度").toBeGreaterThan(0);
    expect(r.gapTop, "胶囊越过了字段上边框").toBeGreaterThanOrEqual(0);
    expect(r.gapBottom, "胶囊越过了字段下边框").toBeGreaterThanOrEqual(0);
    expect(r.gapRight, "胶囊越过了字段右边框").toBeGreaterThanOrEqual(0);
  });
});

// ---------------------------------------------------------------------------------------------
// 练习页在手机上的竖向密度。
//
// 回归对象（`crates/app/style/modern/kit.css` 末尾的「窄屏练习 / 考试：题卡竖向密度」）：
//   ① 题号 / 题型 / 题号徽标必须在**同一行**，朗读按钮另占卡头第二列、与它们**同一行** ——
//      朗读按钮曾挤在标题行里，44px 的触控高度加上比题号还高的徽标，320px 下被挤到第二行，
//      白占一整行 44px；现在它靠 `data-slot="card-action"` 走 `CARD_HEADER` 的 `1fr auto` 两列，
//      标题怎么折行它都钉在右上；
//   ② 选项字不能比题干还大 —— `text-base` 解析的是像素阶梯（18px），题干是 17px，
//      「选项比题干更醒目」既不符合层级，也让长选项更早折行。
//
// 判据都量几何 / 计算样式，不断言「某个 class 存在」：类名换了但排版没坏时不该红，
// 排版坏了但类名还在时必须红。两条都要在 390px 的现代风下量（`hasTouch` 才能走到
// `pointer: coarse` 的 44px 抬档，桌面视口量不到这个缺陷）。
// ---------------------------------------------------------------------------------------------
test.describe("现代风练习页（移动端密度）", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  test("题卡标题行：题号 / 徽标 / 朗读按钮同行，选项字不大于题干", async ({ page }) => {
    await seed(page);
    await page.addInitScript(() => {
      localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
    });
    await page.goto("/practice?bank=A");
    await expect(page.locator("main .md-opt").first()).toBeVisible();
    await settleFrames(page);

    const m = await page.evaluate(() => {
      const title = document.querySelector('[data-slot="card-title"]') as HTMLElement;
      if (!title) throw new Error("题卡标题行不存在");
      // 标题行的直接子项是「第 N / M 题」、题型徽标、题号徽标这三件；朗读按钮是卡头的另一列。
      const kids = Array.from(title.children) as HTMLElement[];
      const tops = kids.map((k) => Math.round(k.getBoundingClientRect().top));
      const speaker = document.querySelector('[data-slot="card-action"]') as HTMLElement | null;
      const stem = document.querySelector('[data-slot="card-content"] > div') as HTMLElement;
      const optText = document.querySelector('[data-slot="option-text"], .md-opt > label') as HTMLElement;
      const optRow = document.querySelector(".md-opt") as HTMLElement;
      const card = document.querySelector(".md-q") as HTMLElement;
      return {
        kidCount: kids.length,
        // 同一行的判据：所有子项顶边的极差小于最小子项高度的一半（换行会整行下移 ~24px）。
        topSpread: tops.length ? Math.max(...tops) - Math.min(...tops) : -1,
        rowHeights: kids.map((k) => Math.round(k.getBoundingClientRect().height)),
        // 朗读按钮与标题行首项的顶边差：按钮绝对定位在卡头右上并垂直居中，
      // 与标题行同处一行（差值是负的也可能，取绝对值）。
        speakerTopDelta: speaker ? Math.abs(Math.round(speaker.getBoundingClientRect().top - tops[0])) : null,
        speakerBox: speaker
          ? (() => {
              const r = speaker.getBoundingClientRect();
              return { w: Math.round(r.width), h: Math.round(r.height) };
            })()
          : null,
        // 按钮里那个图标的视觉尺寸（不是命中区）：它曾与题号不成比例地大。
        speakerIcon: speaker
          ? (() => {
              const svg = speaker.querySelector("svg");
              if (!svg) return null;
              const r = svg.getBoundingClientRect();
              return { w: Math.round(r.width), h: Math.round(r.height) };
            })()
          : null,
        stemFont: stem ? parseFloat(getComputedStyle(stem).fontSize) : null,
        optFont: optText ? parseFloat(getComputedStyle(optText).fontSize) : null,
        optRowHeight: optRow ? Math.round(optRow.getBoundingClientRect().height) : null,
        cardPad: card ? getComputedStyle(card).padding : null,
        hasOptionTextHook: !!document.querySelector('[data-slot="option-text"]'),
      };
    });

    expect(m.kidCount, "标题行应有 3 件内容（题号 + 两个徽标）").toBe(3);
    expect(
      m.topSpread,
      `标题行的 3 件内容必须在同一行（实际顶边极差 ${m.topSpread}px，换行说明被挤到了第二行）`,
    ).toBeLessThan(12);

    // 朗读按钮是唯一会因 `pointer: coarse` 抬到 44 的项：它必须在，且不小于 WCAG 2.5.8 的 24。
    expect(m.speakerBox, "题卡卡头里应有朗读按钮").not.toBeNull();
    expect(m.speakerBox!.h).toBeGreaterThanOrEqual(24);
    expect(m.speakerBox!.w).toBeGreaterThanOrEqual(24);
    // 视觉只有图标那么大：命中区是 44，图标不能跟着放大到与题号不成比例。
    expect(m.speakerIcon, "朗读按钮里应有图标").not.toBeNull();
    expect(m.speakerIcon!.h, "朗读图标不该与题号不成比例").toBeLessThanOrEqual(20);
    expect(m.speakerIcon!.w, "朗读图标不该与题号不成比例").toBeLessThanOrEqual(20);
    // 它在卡头里垂直居中：与标题行同一行，不能掉到下面自成一行。
    expect(
      m.speakerTopDelta,
      `朗读按钮应与标题行同一行（实际顶边差 ${m.speakerTopDelta}px）`,
    ).toBeLessThan(16);

    // 选项字不能比题干大：`text-base` 在像素阶梯里是 18，曾让选项比 17 的题干还醒目。
    expect(m.stemFont, "没量到题干字号").toBeGreaterThan(0);
    expect(m.optFont, "没量到选项字号").toBeGreaterThan(0);
    expect(m.optFont, "选项字不应大于题干（层级错位，且长选项更早折行）").toBeLessThanOrEqual(
      m.stemFont,
    );
    // 字号也不该掉出阶梯最小档（`--md-text-3xs`，中文被抬到 11）。
    expect(m.optFont, "选项字不应低于字号阶梯最小档").toBeGreaterThanOrEqual(12);
    // 单行选项仍要 ≥ 44 的触控目标（整行是 label，宽度撑满）。
    expect(m.optRowHeight, "选项行高度低于触控目标").toBeGreaterThanOrEqual(44);
  });

  // 答完题后窄屏同屏出现两个橙色「下一题」（底栏一个、浮钮一个），浮钮还盖住「朗读解析」。
  // 判据按「用户看得见、点得到的」数：隐藏态（`display:none` / `visibility:hidden`）不算。
  test("答完题后可见的「下一题」只有一个，且没有东西盖住朗读解析", async ({ page }) => {
    await seed(page);
    await page.addInitScript(() => {
      localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
    });
    await page.goto("/practice?bank=A");
    await expect(page.locator("main .md-opt").first()).toBeVisible();
    await settleFrames(page);

    // 第一题是多选：勾两个再「检查答案」才算作答；单选题点一下就算。两种都兼容。
    const checkBtn = page.getByRole("button", { name: "检查答案" });
    const opts = page.locator("main .md-opt");
    await opts.first().click();
    if (await checkBtn.count()) {
      await opts.nth(2).click();
      await checkBtn.first().click();
    }
    await expect(page.locator(".md-verdict, .md-opt-right, .md-opt-wrong").first()).toBeVisible();
    await settleFrames(page);

    const r = await page.evaluate(() => {
      const visible = (el: Element) => {
        const cs = getComputedStyle(el);
        const b = el.getBoundingClientRect();
        return cs.display !== "none" && cs.visibility !== "hidden" && +cs.opacity > 0 && b.width > 0;
      };
      const nexts = Array.from(document.querySelectorAll("button")).filter(
        (b) => /^下一题/.test(b.textContent?.trim() ?? "") && visible(b),
      );
      // 朗读解析按钮：中心点命中的必须是它自己（没被浮层盖住）。
      const explain = document.querySelector('button[aria-label="朗读解析"]') as HTMLElement | null;
      let covered: string | null = null;
      if (explain) {
        explain.scrollIntoView({ block: "center" });
        const b = explain.getBoundingClientRect();
        const top = document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2);
        covered = top && !explain.contains(top) ? (top.closest("[class]")?.className ?? "?").toString() : null;
      }
      return { nextCount: nexts.length, covered, hasExplain: !!explain };
    });

    expect(r.nextCount, "窄屏答完题后同屏只能有一个可见的「下一题」").toBe(1);
    expect(r.hasExplain, "没找到朗读解析按钮，用例失去意义").toBe(true);
    expect(r.covered, `朗读解析被 ${r.covered} 盖住`).toBeNull();
  });

  // 小屏上「顶栏 + 页头 + 固定底栏」不能把题卡挤出视口。
  // 判据是**题卡在顶栏与底栏之间的可视高度占视口的比例**，不是某个像素值，也不依赖随机抽到的题长短：
  // 整改前 320×568 的考试页只剩 24%（138px）、练习页 27%，整改后 43% / 45%。
  // 阈值 38% 落在两者之间：回退到旧版式会红，页头再长一行也会红。
  for (const [name, url] of [
    ["考试页", "/exam"],
    ["练习页", "/practice?bank=A"],
  ] as const) {
    test(`320×568 ${name}：题卡可视高度不低于视口的 38%`, async ({ page }) => {
      await seed(page);
      await page.addInitScript(() => {
        localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
      });
      await page.setViewportSize({ width: 320, height: 568 });
      await page.goto(url);
      await expect(page.locator("main .md-opt").first()).toBeVisible();
      await settleFrames(page);

      const m = await page.evaluate(() => {
        const q = document.querySelector(".md-q") as HTMLElement | null;
        const bars = Array.from(document.querySelectorAll(".md-bar, .md-bar-practice")).filter(
          (b) => getComputedStyle(b).display !== "none" && getComputedStyle(b).position === "fixed",
        );
        const barTop = bars.length ? Math.min(...bars.map((b) => b.getBoundingClientRect().top)) : innerHeight;
        return {
          vh: innerHeight,
          qTop: q ? q.getBoundingClientRect().top : -1,
          barTop,
          bars: bars.length,
        };
      });
      expect(m.bars, "没找到固定底栏，用例失去意义").toBeGreaterThan(0);
      expect(m.qTop, "没找到题卡").toBeGreaterThan(0);
      const visible = m.barTop - m.qTop;
      expect(
        visible / m.vh,
        `题卡在顶栏与底栏之间只剩 ${Math.round(visible)}px（视口 ${m.vh}px）：页头 / 底栏又长高了`,
      ).toBeGreaterThanOrEqual(0.38);
    });
  }

  /** 在练习页答完当前题（多选题要先勾第二项再点「检查答案」），让「下一题」出口露出来。 */
  async function answerCurrentQuestion(page: Page) {
    await expect(page.locator("main .md-opt").first()).toBeVisible();
    await page.locator("main .md-opt").first().click();
    const checkBtn = page.getByRole("button", { name: "检查答案" });
    if (await checkBtn.count()) {
      await page.locator("main .md-opt").nth(2).click();
      await checkBtn.first().click();
    }
  }

  // 1024 与恰好 900 各一条：`≥900` 是「底栏隐藏、浮钮保留」那一档，边界只属于这一边。
  // `.md-next-float` 的窄屏档若写成 `max-width: 900px`，900 会被两边同时算上，
  // 练习页就既没有底栏也没有浮钮 —— 答完题后没有任何「下一题」出口。
  for (const width of [1024, 900]) {
    test(`≥900（${width}px）：底栏不显示，浮动「下一题」仍是答完题后的出口`, async ({ page }) => {
      await seed(page);
      await page.setViewportSize({ width, height: 800 });
      await page.addInitScript(() => {
        localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
      });
      await page.goto("/practice?bank=A");
      await answerCurrentQuestion(page);
      await expect(page.locator(".md-next-float[data-visible='true']")).toBeVisible();
      await expect(page.locator(".md-bar-practice")).toBeHidden();
    });
  }
});

// ---------------------------------------------------------------------------------------------
// 阅读页的流体正文（`docs/ui/modern.md` §2.3「正文分两档」）。
//
// 回归对象（`crates/app/style/modern/{tokens,kit}.css`）：
//   ① 阅读正文 `--md-text-prose` 随视口**连续**放大：360 → 15、≥1200 → 17.5；
//   ② 行宽 `--md-measure` 用 `em`，所以「每行字数」不随视口变 ——
//      它要是写成 px，宽屏上字号变大、每行字数跟着涨，等于没约束。
//
// 判据只认几何与计算样式：换成别的单位 / 别的断点实现，只要这两个关系成立就不该红。
// ---------------------------------------------------------------------------------------------
test.describe("现代风阅读页（流体正文）", () => {
  test("正文字号随视口放大，每行字数恒定", async ({ page }) => {
    await seed(page);
    await page.goto("/nvis");
    await expect(page.locator(".md-article .md-panel").first()).toBeVisible();
    await settleFrames(page);

    const measure = async (width: number) => {
      await page.setViewportSize({ width, height: 900 });
      await settleFrames(page);
      return page.evaluate(() => {
        const host = document.querySelector(".md-article-page") as HTMLElement | null;
        if (!host) throw new Error("阅读页容器不存在");
        // 列表项是成段文本里最稳定的一类：段落来自知识库正文，条目数不固定。
        const seg = document.querySelector(".md-article .md-bullets li") as HTMLElement | null;
        const segCs = seg ? getComputedStyle(seg) : null;
        const rect = seg?.getBoundingClientRect();
        return {
          font: parseFloat(getComputedStyle(host).fontSize),
          // 每行能放几个汉字：宽度 / 字号（汉字方块的宽就是 1em）。
          charsPerLine: rect && segCs ? Math.round(rect.width / parseFloat(segCs.fontSize)) : null,
          overflowX: document.documentElement.scrollWidth > document.documentElement.clientWidth,
        };
      });
    };

    const phone = await measure(360);
    const desktop = await measure(1440);

    expect(phone.overflowX || desktop.overflowX, "阅读页出现横向滚动").toBe(false);
    // 手机落到下限，桌面接近上限；两端都要在「正文档」的范围内。
    expect(phone.font, "手机上的阅读正文低于 15").toBeGreaterThanOrEqual(15);
    expect(phone.font, "手机上的阅读正文不该超过 16").toBeLessThanOrEqual(16);
    expect(desktop.font, "桌面上的阅读正文没放大").toBeGreaterThan(phone.font);
    expect(desktop.font, "桌面上的阅读正文超过 17.5").toBeLessThanOrEqual(17.6);

    // 行宽上限 36em（约 36 个汉字）。手机上屏幕本来就窄，36em 落不到地，
    // 每行字数由视口决定；桌面上才能量到 measure 本身 —— 这条断言只在桌面上成立。
    expect(phone.charsPerLine, "手机上没量到成段文本").not.toBeNull();
    expect(desktop.charsPerLine, "桌面上没量到成段文本").not.toBeNull();
    expect(
      desktop.charsPerLine!,
      `桌面正文行宽跑出了 32~40 字（实际 ${desktop.charsPerLine}）`,
    ).toBeGreaterThanOrEqual(32);
    expect(
      desktop.charsPerLine!,
      `桌面正文行宽跑出了 32~40 字（实际 ${desktop.charsPerLine}）`,
    ).toBeLessThanOrEqual(40);
    // 手机上没有被 measure 压窄（压窄就成了「窄栏」，屏幕宽度还没用完）。
    expect(
      phone.charsPerLine!,
      `手机正文被行宽上限压窄了（实际 ${phone.charsPerLine} 字/行）`,
    ).toBeLessThanOrEqual(desktop.charsPerLine!);
  });
});
// ---------------------------------------------------------------------------------------------
// 触屏触控目标。
//
// 回归对象：`kit.css` 的 `@media (pointer: coarse)` 段。它按「族」抬到 44，
// 而历史上是逐个类列举 —— 新增控件（chip-sm / quiz-opt / link-row / switch / x-toggle）
// 不会自动被覆盖，28~38px 的目标在拇指下只能靠猜。
//
// 白名单（都不该按 44 要求）：
//   · `.md-chip-sm` 且是 `<a>`：卡片里的分类标签，信息价值优先，32 仍高于 WCAG 2.5.8 的 24；
//   · `.md-footnote`：视觉是行内小字，命中区由 `::after` 撑到 44，`getBoundingClientRect` 量不到；
//   · 段落 / 列表里的行内链接：WCAG 2.5.8 的行内例外。
// ---------------------------------------------------------------------------------------------
test.describe("触屏触控目标", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  test("代表页面的可点元素最小边不小于 44", async ({ page }) => {
    await seed(page);
    const routes = ["/", "/browse", "/glossary", "/practice", "/exam", "/cards"];
    const offenders: string[] = [];
    for (const route of routes) {
      await page.goto(route);
      await expect(page.locator("main")).toBeVisible();
      await settleFrames(page);
      if (route === "/browse") {
        // 窄屏的分类在抽屉里：开抽屉 → 选一个题型，子类行才渲染（默认视图只有题型）。
        await page.locator(".md-drawer-fab").click();
        // 抽屉是「下一个任务」才挂上的（推迟开是为了躲开 pointerup 之后那个 click），
        // 所以这里必须等面板出现再数按钮。
        await expect(page.locator(".md-sheet-panel")).toBeVisible();
        const type = page.locator(".md-sheet .md-drawer-list").first().locator("button").nth(1);
        if ((await type.count()) > 0) {
          await type.click();
          await settleFrames(page);
        } else {
          throw new Error("题库浏览的分类抽屉没渲染出题型，用例的覆盖面不成立");
        }
      }
      const bad = await page.evaluate(() => {
        const out: string[] = [];
        for (const el of document.querySelectorAll("button, a[href], [role=button], summary")) {
          const cs = getComputedStyle(el);
          const r = el.getBoundingClientRect();
          if (cs.display === "none" || cs.visibility === "hidden" || parseFloat(cs.opacity) < 0.1) continue;
          if (r.width <= 2 || r.height <= 2) continue;
          if (r.width >= 44 && r.height >= 44) continue;
          const cls = (el.className || "").toString();
          if (cls.includes("md-footnote")) continue;
          if (cls.includes("md-chip-sm") && el.tagName === "A") continue;
          if (el.tagName === "A" && el.closest("p, li, dd, .md-x-body, .md-article, .md-bullets")) continue;
          const name = (el.getAttribute("aria-label") || el.textContent || "").trim().slice(0, 20);
          out.push(
            `${cls.split(" ").find((c) => c.startsWith("md-")) || el.tagName.toLowerCase()} ` +
              `${Math.round(r.width)}x${Math.round(r.height)}「${name}」`,
          );
        }
        return out;
      });
      offenders.push(...bad.map((b) => `${route} ${b}`));
    }
    expect(offenders, "触屏上有小于 44×44 的可点元素").toEqual([]);
  });

  // 第二轮补的几族。上面那条用 `getBoundingClientRect` 量元素本身，
  // 量不到「靠伪元素撑命中区」的勾选框，也不扫勾选行 / 页签 / 页脚链接 / 滑杆 —— 单独点名。
  // 判据：整行 / 整条的**高度** ≥ 44；裸勾选框用 `elementFromPoint` 在框外 10px 处取点，
  // 命中的必须是它自己或包着它的 `label`（两种都能让一次点击切换勾选）。
  for (const style of ["modern", "pixel"] as const) {
    test(`第二轮补的触控目标（${style === "modern" ? "现代风" : "像素风"}）`, async ({ page }) => {
      await page.addInitScript((s) => {
        if (window !== window.top) return;
        localStorage.setItem("ui:uiStyle", s);
        localStorage.setItem("theme", "light");
      }, style);
      const sel = style === "modern"
        ? { check: "button.md-check", row: ".md-check-row", tab: ".md-tab", chip: ".md-chip:not(.md-chip-sm)" }
        : { check: "button.pxl-check", row: "label.cursor-pointer", tab: ".pxl-tab", chip: ".pxl-chip" };
      const bad: string[] = [];

      const measure = async (route: string, what: Record<string, string>) => {
        await page.goto(route);
        await expect(page.locator("main")).toBeVisible();
        await settleFrames(page);
        const r = await page.evaluate((w) => {
          const out: string[] = [];
          const tall = (selector: string, label: string) => {
            for (const el of document.querySelectorAll(selector)) {
              const b = el.getBoundingClientRect();
              const cs = getComputedStyle(el);
              if (cs.display === "none" || b.width <= 2 || b.height <= 2) continue;
              if (b.height < 43.5) out.push(`${label} 高 ${Math.round(b.height)}`);
            }
          };
          for (const [label, selector] of Object.entries(w)) tall(selector, label);
          // 裸勾选框：框外 14px 的点必须落在它自己或包着它的 label 里。
          return out;
        }, what);
        bad.push(...r.map((x) => `${route} ${x}`));
      };

      await measure("/log", { 勾选行: sel.row });
      await measure("/morse", { 页签: sel.tab, 滑杆: "input[type=range]" });
      await measure("/practice", { 筛选胶囊: sel.chip });

      // 勾选框命中区
      await page.goto("/log");
      await expect(page.locator("main")).toBeVisible();
      const hit = await page.evaluate((cs) => {
        const c = document.querySelector(cs) as HTMLElement | null;
        if (!c) return { found: false, ok: false };
        c.scrollIntoView({ block: "center" });
        const b = c.getBoundingClientRect();
        // 取框外 10px：命中区（16 + 2×14 = 44）的最外缘恰好是 14px，边界上的点不稳定，
        // 10px 是真实拇指会落的位置，且仍在 44×44 之内。
        const pts = [
          [b.left - 10, b.top + b.height / 2],
          [b.right + 10, b.top + b.height / 2],
          [b.left + b.width / 2, b.top - 10],
          [b.left + b.width / 2, b.bottom + 10],
        ];
        const ok = pts.every(([x, y]) => {
          const el = document.elementFromPoint(x, y);
          return !!el && (c.contains(el) || !!el.closest("label")?.contains(c));
        });
        return { found: true, ok };
      }, sel.check);
      expect(hit.found, "没找到勾选框，用例失去意义").toBe(true);
      expect(hit.ok, "勾选框周围 10px 的点没有落在它（或它的 label）里：命中区不到 44×44").toBe(true);

      if (style === "modern") {
        await page.goto("/stats");
        await expect(page.locator("footer .md-nav-link").first()).toBeVisible();
        const links = await page.evaluate(() =>
          Array.from(document.querySelectorAll("footer .md-nav-link")).map((a) =>
            Math.round(a.getBoundingClientRect().height),
          ),
        );
        expect(links.length, "页脚没有链接").toBeGreaterThan(0);
        for (const h of links) if (h < 44) bad.push(`/stats 页脚链接 高 ${h}`);
      }

      expect(bad, "触屏上有高度不足 44 的可点行 / 页签 / 滑杆 / 胶囊").toEqual([]);
    });
  }
});

// ---------------------------------------------------------------------------------------------
// 页内菜单的悬浮按钮：能拖、位置记得住、轻点仍然开抽屉。
// 拖动判定（阈值 6px、路径累计）在 `ham_web_core::fab`，这里验的是接线。
// ---------------------------------------------------------------------------------------------
test.describe("页内菜单的悬浮按钮", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  // 真机上浏览器在 `pointerup` 之后**稍晚**才补派 click，落点按「派发那一刻指针下方的元素」
  // 重新算（Playwright 的 tap 把它们挤在同一个任务里，复现不出）。所以这里手动按真机时序
  // 建模：先发 pointerdown / pointerup，隔 `delay` 毫秒后对 `elementFromPoint` 发 click。
  // 在 `pointerup` 里就开抽屉的实现，晚到的 click 会打进抽屉行：浏览页是行间空白、
  // 工具页是某个计算器行 —— 后者会直接滚走并收起抽屉（「点悬浮按钮就触发了某个菜单」）。
  for (const route of ["/tools", "/browse"]) {
    for (const delay of [0, 60, 300]) {
      test(`轻点按钮只开抽屉，不触发抽屉里的菜单（${route}，click 晚到 ${delay}ms）`, async ({
        page,
      }) => {
        await seed(page);
        await page.goto(route);
        await expect(page.locator(".md-drawer-fab")).toBeVisible();
        const scrollBefore = await page.evaluate(() => window.scrollY);

        const landed = await page.evaluate(async (wait) => {
          const fab = document.querySelector<HTMLElement>(".md-drawer-fab")!;
          const r = fab.getBoundingClientRect();
          const x = r.left + r.width / 2;
          const y = r.top + r.height / 2;
          const pointer = {
            bubbles: true,
            cancelable: true,
            clientX: x,
            clientY: y,
            pointerId: 7,
            pointerType: "touch",
            isPrimary: true,
          };
          fab.dispatchEvent(new PointerEvent("pointerdown", pointer));
          fab.dispatchEvent(new PointerEvent("pointerup", pointer));
          await new Promise((resolve) => setTimeout(resolve, wait));
          const target = document.elementFromPoint(x, y)!;
          target.dispatchEvent(
            new MouseEvent("click", { bubbles: true, cancelable: true, detail: 1, clientX: x, clientY: y }),
          );
          return target.closest(".md-sheet") ? "in-sheet" : "outside";
        }, delay);

        expect(landed, "补派的 click 落进了抽屉（会触发一个菜单项）").toBe("outside");
        await expect(page.locator(".md-sheet-panel"), "轻点按钮没打开抽屉，或被菜单项顺手收起了").toBeVisible();
        // 没有任何菜单项被触发：页面没有被滚走、地址没有带上锚点，抽屉也没有开始收起。
        // 等有限动画播完 + 两帧，而不是赌一个毫秒数：菜单项一旦被触发，收起的退场动画
        // 会在这段时间里走完并卸载抽屉，下面再断言一次「还开着」才有意义。
        await settleFrames(page);
        await expect(page.locator(".md-sheet-panel"), "抽屉被菜单项顺手收起了").toBeVisible();
        expect(await page.evaluate(() => window.scrollY), "页面被菜单项滚走了").toBe(scrollBefore);
        expect(await page.evaluate(() => location.hash), "地址带上了菜单项的锚点").toBe("");
      });
    }
  }

  test("拖动后晚到的 click 不开抽屉，键盘激活仍然能开", async ({ page }) => {
    await seed(page);
    await page.goto("/tools");
    const fab = page.locator(".md-drawer-fab");
    await expect(fab).toBeVisible();

    await page.evaluate(async () => {
      const el = document.querySelector<HTMLElement>(".md-drawer-fab")!;
      const r = el.getBoundingClientRect();
      const x = r.left + r.width / 2;
      const y = r.top + r.height / 2;
      const at = (cx: number, cy: number) => ({
        bubbles: true,
        cancelable: true,
        clientX: cx,
        clientY: cy,
        pointerId: 7,
        pointerType: "touch",
        isPrimary: true,
      });
      el.dispatchEvent(new PointerEvent("pointerdown", at(x, y)));
      el.dispatchEvent(new PointerEvent("pointermove", at(x - 60, y - 140)));
      el.dispatchEvent(new PointerEvent("pointerup", at(x - 60, y - 140)));
      // 拖完之后很久才补派的 click（位置已经变了，落点仍是按钮自己）。
      await new Promise((resolve) => setTimeout(resolve, 300));
      const r2 = el.getBoundingClientRect();
      const target = document.elementFromPoint(r2.left + r2.width / 2, r2.top + r2.height / 2)!;
      target.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, detail: 1 }));
    });
    // 负向断言：等「该发生的都发生过了」（有限动画 + 两帧）再确认抽屉没开。
    await settleFrames(page);
    await expect(page.locator(".md-sheet-panel"), "拖动后补派的 click 把抽屉带开了").toBeHidden();

    // 键盘 Enter（`detail == 0`）不受「刚拖完」影响，照样能开。
    await fab.focus();
    await page.keyboard.press("Enter");
    await expect(page.locator(".md-sheet-panel"), "键盘激活没能打开抽屉").toBeVisible();
  });

  test("拖到别处后位置还在，轻点仍能打开抽屉", async ({ page }) => {
    await seed(page);
    await page.goto("/browse");
    const fab = page.locator(".md-drawer-fab");
    await expect(fab).toBeVisible();
    const before = (await fab.boundingBox())!;

    // 按住往上左拖，位移远超 6px 的轻点阈值。
    await page.mouse.move(before.x + before.width / 2, before.y + before.height / 2);
    await page.mouse.down();
    await page.mouse.move(before.x + before.width / 2 - 60, before.y + before.height / 2 - 140, {
      steps: 10,
    });
    await page.mouse.up();

    const after = (await fab.boundingBox())!;
    expect(Math.round(before.x - after.x), "按钮没往左移动").toBeGreaterThan(40);
    expect(Math.round(before.y - after.y), "按钮没往上移动").toBeGreaterThan(110);
    // 拖动不是轻点：不该顺手把抽屉打开。
    await expect(page.locator(".md-sheet-panel"), "拖动后不该打开抽屉").toBeHidden();
    // 位置写进了 `localStorage`，且与像素风共用同一个 key。
    const saved = await page.evaluate(() => localStorage.getItem("fab-position"));
    expect(saved, "拖动后没有持久化位置").toMatch(/^-?\d+(\.\d+)?,-?\d+(\.\d+)?$/);

    // 换页 / 刷新都还在原地（同一个 key，两个皮肤共用）。
    await page.reload();
    const reloaded = (await page.locator(".md-drawer-fab").boundingBox())!;
    expect(Math.abs(reloaded.x - after.x), "刷新后横坐标丢了").toBeLessThanOrEqual(2);
    expect(Math.abs(reloaded.y - after.y), "刷新后纵坐标丢了").toBeLessThanOrEqual(2);

    // 轻点仍然开抽屉。
    await page.locator(".md-drawer-fab").click();
    await expect(page.locator(".md-sheet-panel")).toBeVisible();
  });
});

// ---------------------------------------------------------------------------------------------
// 展开子类要有入场动画：逐帧采 `opacity`，不能只看「元素出现了」。
// 只动 `opacity` / `translate`（`docs/ui/motion.md` §2.1），时长读 token。
// ---------------------------------------------------------------------------------------------
test.describe("抽屉里展开子类", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  test("子类入场是动画而不是瞬间出现", async ({ page }) => {
    await seed(page);
    await page.goto("/browse");
    await page.locator(".md-drawer-fab").click();
    await expect(page.locator(".md-sheet-panel")).toBeVisible();

    // 点击与采样放进同一次求值：跨一次 `await` 就错过 180ms 的动画了。
    const sampled = await page.evaluate(async () => {
      const rows = document.querySelectorAll<HTMLElement>(
        ".md-sheet .md-drawer-list > li > .md-drawer-item",
      );
      rows[1]!.click();
      // 等 Leptos 把子类节点挂上（下一帧）。
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const sub = document.querySelector<HTMLElement>(".md-drawer-sublist");
      if (!sub) return null;
      const style = getComputedStyle(sub);
      const opacities: number[] = [];
      for (let i = 0; i < 14; i += 1) {
        opacities.push(Number(getComputedStyle(sub).opacity));
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
      return {
        opacities,
        name: style.animationName,
        duration: style.animationDuration,
        property: style.transitionProperty,
      };
    });

    expect(sampled, "选了题型后没有渲染子类").not.toBeNull();
    expect(sampled!.name, "子类入场不是动画").toBe("md-drawer-sub-in");
    // 时长走 token（`--md-dur` = 180ms），不是手写魔法值。
    expect(sampled!.duration, "入场时长没走 `--md-dur`").toBe("0.18s");
    // 逐帧采样：起始帧是半透明的，末帧（180ms 后）完全不透明。
    expect(Math.min(...sampled!.opacities), "起始帧就不是半透明，说明没有动画").toBeLessThan(0.9);
    expect(Math.max(...sampled!.opacities), "末帧没有回到完全不透明").toBeGreaterThan(0.95);
  });

  test("减少动态效果下同一条入场被压到 1ms", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await seed(page);
    await page.goto("/browse");
    await page.locator(".md-drawer-fab").click();
    await expect(page.locator(".md-sheet-panel")).toBeVisible();

    const duration = await page.evaluate(async () => {
      const rows = document.querySelectorAll<HTMLElement>(
        ".md-sheet .md-drawer-list > li > .md-drawer-item",
      );
      rows[1]!.click();
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const sub = document.querySelector<HTMLElement>(".md-drawer-sublist");
      return sub ? getComputedStyle(sub).animationDuration : null;
    });
    // `.md-sheet *` 在 tokens.css 的 reduce 名单里（像素风 motion.css 的全局兜底会把它
    // 再压到 0.01ms）；漏登记时这里会是 0.18s。
    const ms = Number((duration ?? "").replace("s", "")) * 1000;
    expect(ms, `减少动态效果没有压掉子类入场（${duration}）`).toBeLessThanOrEqual(1);
  });
});

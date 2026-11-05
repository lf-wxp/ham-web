import type { Page } from "@playwright/test";
import { expect, settleFrames, test, waitSettled } from "./fixtures";

// 布局一致性回归：结构页头、现代风页头不吸顶、尺度收敛、首屏入口。
// 判据写在各用例里；现行的页头 / 尺度规则见 `docs/ui/modern.md`。

async function seed(page: Page) {
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "modern");
    localStorage.setItem("theme", "dark");
  });
}

test("迁移页都有结构页头（标题 + 副标题 + data-page-head）", async ({ page }) => {
  await seed(page);
  for (const url of ["/browse", "/glossary", "/log", "/tools", "/reference", "/bookmarks"]) {
    await page.goto(url);
    const head = page.locator("header[data-page-head]").first();
    await expect(head, `${url} 缺少结构页头`).toBeVisible();
    await expect(head.locator("h1"), `${url} 页头缺 h1`).not.toBeEmpty();
    await expect(head.locator("p.md-lede"), `${url} 页头缺副标题`).toHaveCount(1);
  }
});

test("桥接页与迁移页用同一个页头组件，且不吸顶", async ({ page }) => {
  await seed(page);
  await page.goto("/radio-law");
  // `PageHeader` 在现代风下直接派发到 `ModernPageHead`：桥接页也长这样。
  const header = page.locator("header[data-page-head]").first();
  await expect(header).toBeVisible();
  await expect(header).toHaveCSS("position", "static");
});

test("同层级页面的首块内容起点一致（差值 ≤ 8px）", async ({ page }) => {
  await seed(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1440, height: 1000 });
  const top = async (url: string) => {
    await page.goto(url);
    await page.waitForTimeout(400);
    return page.evaluate(() => {
      const main = document.querySelector("#main-content") as HTMLElement;
      const first = main.querySelector(
        ".md-page > *:not(header), .max-w-5xl.space-y-6 > *",
      ) as HTMLElement | null;
      return first ? Math.round(first.getBoundingClientRect().top + window.scrollY) : -1;
    });
  };
  const migrated = await top("/browse");
  const bridged = await top("/radio-law");
  // 取不到首块时两页都返回 -1，差值为 0 会「通过」：先证明两页都真的量到了。
  expect(migrated, "/browse 没找到首块内容").toBeGreaterThan(0);
  expect(bridged, "/radio-law 没找到首块内容").toBeGreaterThan(0);
  expect(Math.abs(migrated - bridged), `${migrated} vs ${bridged}`).toBeLessThanOrEqual(8);
});

test("字号与卡片层次落在收敛后的尺度内", async ({ page }) => {
  await seed(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1440, height: 1000 });

  await page.goto("/");
  let size = await page.locator("h1").first().evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
  expect(size, "display-1 上限 72px").toBeLessThanOrEqual(72);

  await page.goto("/browse");
  size = await page.locator("h1").first().evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
  expect(size, "页标题上限 52px").toBeLessThanOrEqual(52);

  // 卡片必须有一层投影：完全平面时浅色下只剩 1px 描边，认不出「一张卡」。
  // `/log` 已迁到现代组件（卡片是 `.md-panel`），判据不变，选择器按皮肤各认一个。
  await page.goto("/log");
  await page.waitForTimeout(600);
  const shadow = await page
    .locator(".pxl-card, .md-panel")
    .first()
    .evaluate((el) => getComputedStyle(el).boxShadow);
  expect(shadow).not.toBe("none");

  await page.setViewportSize({ width: 320, height: 844 });
  await page.goto("/");
  const h1 = page.locator("h1").first();
  size = await h1.evaluate((el) => parseFloat(getComputedStyle(el).fontSize));
  expect(size, "320 宽下标题字号上限 44px").toBeLessThanOrEqual(44);
  // 标题写的是「不超过 3 行」，就量行数：高度 ÷ 行高。
  const lines = await h1.evaluate((el) => {
    const cs = getComputedStyle(el);
    const lineHeight = parseFloat(cs.lineHeight) || parseFloat(cs.fontSize) * 1.2;
    return Math.round(el.getBoundingClientRect().height / lineHeight);
  });
  expect(lines, "320 宽下标题不超过 3 行").toBeLessThanOrEqual(3);
});

test("吸顶条与页面同色，且不比下面的卡片宽", async ({ page }) => {
  await seed(page);
  await page.emulateMedia({ reducedMotion: "reduce" });

  // 量一根吸顶条：位置、底色与页面底色的对比、左右边界与卡片是否对齐。
  const read = (barSel: string, cardSel: string) =>
    page.evaluate(
      ([sel, card]) => {
        const bar = document.querySelector(sel) as HTMLElement | null;
        if (!bar) return null;
        const card0 = document.querySelector(card) as HTMLElement | null;
        const cs = getComputedStyle(bar);
        const rect = bar.getBoundingClientRect();
        const r = card0?.getBoundingClientRect();
        return {
          position: cs.position,
          top: Math.round(rect.top),
          left: Math.round(rect.left),
          right: Math.round(rect.right),
          cardLeft: r ? Math.round(r.left) : -1,
          cardRight: r ? Math.round(r.right) : -1,
          background: cs.backgroundColor,
          page: getComputedStyle(document.body).backgroundColor,
        };
      },
      [barSel, cardSel],
    );

  const check = async (barSel: string, cardSel: string, label: string) => {
    const bar = await read(barSel, cardSel);
    expect(bar, `${label}没有渲染`).not.toBeNull();
    expect(bar!.position, `${label}应吸顶`).toBe("sticky");
    // 吸在顶栏下方而不是随页面滚走，才说明量到的是吸顶态。
    expect(bar!.top, `${label}没有吸在顶栏下方`).toBeLessThanOrEqual(80);
    // 半透明 + 模糊会让底下的卡片透上来，整条比页面亮一档：底色必须与页面完全一致。
    expect(bar!.background, `${label}底色应与页面同色`).toBe(bar!.page);
    // 左右边界不能探出下面的卡片：铺到版心边缘会让下边框比卡片宽出一截。
    expect(bar!.cardLeft, `${label}下面没有可比的卡片`).toBeGreaterThan(0);
    expect(Math.abs(bar!.left - bar!.cardLeft), `${label}左边比卡片宽`).toBeLessThanOrEqual(1);
    expect(Math.abs(bar!.right - bar!.cardRight), `${label}右边比卡片宽`).toBeLessThanOrEqual(1);
  };

  // 桌面：题库浏览的搜索条吸顶（`interaction.css` 的 `data-sig="browse"` 分支）。
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto("/browse");
  // 列表渲染出来页面才够高：滚得动，才量得到「吸住」之后的位置。
  await expect(page.locator(".md-q-row").first()).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 900));
  await page.waitForFunction(() => window.scrollY >= 800);
  await settleFrames(page);
  await check(".md-glossary-bar", ".md-q-row", "题库浏览的搜索条");

  // 窄屏没有页内吸顶条：分类 / 目录收进抽屉（见下一条用例），搜索条随页面滚走。
  await page.setViewportSize({ width: 640, height: 900 });
  await page.goto("/glossary");
  await waitSettled(page);
  await page.evaluate(() => window.scrollTo(0, 900));
  await page.waitForFunction(() => window.scrollY >= 800);
  await settleFrames(page);
  await expect(page.locator(".md-glossary-bar"), "窄屏的搜索条不该吸顶").not.toHaveCSS(
    "position",
    "sticky",
  );
});

test("页内菜单在三页是同一种交互：<768 悬浮按钮 + 抽屉，>=768 交给桌面", async ({ page }) => {
  await seed(page);
  await page.emulateMedia({ reducedMotion: "reduce" });

  // <768：三页都有同一颗悬浮按钮，点开是同一个抽屉（`.md-sheet` + `.md-drawer-item` 行）。
  await page.setViewportSize({ width: 390, height: 844 });
  for (const route of ["/browse", "/glossary", "/tools"]) {
    await page.goto(route);
    const fab = page.locator(".md-drawer-fab");
    await expect(fab, `${route} 没有页内菜单按钮`).toBeVisible();
    // 按钮抬在底部 dock 之上（dock 约 84px），否则会被 dock 盖住一半。
    const box = (await fab.boundingBox())!;
    expect(box.y + box.height, `${route} 的按钮被 dock 盖住了`).toBeLessThanOrEqual(844 - 84);
    // 触屏可点目标：圆形按钮自身就是 56。
    expect(Math.round(box.width), `${route} 的按钮宽度`).toBe(56);

    await fab.click();
    const panel = page.locator(".md-sheet-panel");
    await expect(panel, `${route} 点按钮没有打开抽屉`).toBeVisible();
    await expect(panel.locator(".md-drawer-item").first(), `${route} 抽屉里没有菜单行`).toBeVisible();
    // 行高 ≥44（触屏下限）。
    const row = (await panel.locator(".md-drawer-item").first().boundingBox())!;
    expect(row.height, `${route} 的菜单行低于 44`).toBeGreaterThanOrEqual(44);

    if (route === "/browse") {
      // 子类必须缩进挂在**被选中那一类**下面，而不是另起一段（那是「和主类脱节」的旧样子）。
      const types = panel.locator(".md-drawer-list > li > .md-drawer-item");
      await types.nth(1).click();
      const sub = panel.locator(".md-drawer-list > li").nth(1).locator(".md-drawer-sublist");
      await expect(sub, "选题型后子类没有挂在它下面").toBeVisible();
      const subBox = (await sub.locator(".md-drawer-item").first().boundingBox())!;
      const typeBox = (await types.nth(1).boundingBox())!;
      expect(subBox.y, "子类应在主类下方").toBeGreaterThan(typeBox.y + typeBox.height - 2);
      // 子类行也要能选中并收起抽屉。
      await sub.locator(".md-drawer-item").first().click();
      await expect(panel, `${route} 选中子类后抽屉没有收起`).toBeHidden();
      continue;
    }

    // 点一行要能关掉抽屉（否则页面停在遮罩下面）。
    await panel.locator(".md-drawer-item").first().click();
    await expect(panel, `${route} 选中后抽屉没有收起`).toBeHidden();
  }

  // 窄屏不留桌面版胶囊行：同一屏两份菜单会让人不知道点哪个才是「当前分类」。
  for (const route of ["/browse", "/glossary"]) {
    await page.goto(route);
    await expect(page.locator(".md-menu-chips"), `${route} 窄屏还留着胶囊行`).toBeHidden();
  }

  // >=768：按钮退场，桌面侧栏 / 胶囊行接管。
  await page.setViewportSize({ width: 800, height: 900 });
  for (const route of ["/browse", "/glossary"]) {
    await page.goto(route);
    await expect(page.locator(".md-drawer-fab"), `${route} 在 >=768 应隐藏按钮`).toBeHidden();
    await expect(page.locator(".md-menu-chips"), `${route} 在 >=768 应显示胶囊行`).toBeVisible();
  }
  await page.goto("/tools");
  await expect(page.locator(".md-drawer-fab"), "工具页在 >=768 应隐藏按钮").toBeHidden();
  await expect(page.locator(".md-tools-aside"), "工具页的桌面索引卡应接管").toBeVisible();
});

// `kit.css` 两套皮肤都会加载：现代风给站点页脚（`body > footer`）的 84px 让位若漏了
// `html[data-ui-style="modern"]` 门禁，层序更靠后，会把像素风在 `pixel/surfaces.css` 里的 4rem 盖掉。
test("站点页脚的底部让位按皮肤各算各的：像素风 4rem，现代风 84px", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const footer = page.locator("body > footer");
  const readPadding = () =>
    footer.evaluate((el) => ({
      pad: parseFloat(getComputedStyle(el).paddingBottom),
      rem: parseFloat(getComputedStyle(document.documentElement).fontSize),
    }));

  // 像素风是默认皮肤：不种风格键。
  await page.goto("/");
  await expect(footer, "像素风下没找到站点页脚，用例失去意义").toHaveCount(1);
  const pixel = await readPadding();
  expect(pixel.pad, "像素风页脚的底部让位被现代风规则盖掉了").toBeCloseTo(4 * pixel.rem, 1);

  await page.evaluate(() => localStorage.setItem("ui:uiStyle", "modern"));
  await page.reload();
  await expect(footer, "现代风下没找到站点页脚，用例失去意义").toHaveCount(1);
  expect((await readPadding()).pad, "现代风页脚要给底部 dock 让出 84px").toBeCloseTo(84, 1);
});

test("390 首页：插画退场，首屏能看到主行动与第一个内容入口", async ({ page }) => {
  await seed(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");

  await expect(page.locator(".md-hero-art")).toBeHidden();
  await expect(page.getByRole("link", { name: /开始练习/ }).first()).toBeVisible();

  // 看的是**标题**的位置而不是 section 的盒子顶边：section 的上沿被章节线占着，
  // 真正能被认出的入口是标题。780 = 844 视口减去底部 dock（约 64px）。
  const top = await page
    .locator("#main-content .md-page > section")
    .first()
    .locator("h2")
    .evaluate((el) => el.getBoundingClientRect().top + window.scrollY);
  expect(top, "第一个内容入口的标题应露在 dock 之上").toBeLessThanOrEqual(780);
});

test("390 首页的模块入口是纵向列表（不再横向藏起 2/3）", async ({ page }) => {
  await seed(page);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  const rail = page.locator(".md-modules");
  await rail.scrollIntoViewIfNeeded();
  const overflow = await rail.evaluate((el) => el.scrollWidth - el.clientWidth);
  expect(overflow).toBeLessThanOrEqual(1);
  // 标题写的是「纵向列表」：不只是不溢出，还要真的是竖着排（各模块卡的 top 依次递增）。
  const tops = await rail.locator(".md-mod").evaluateAll((els) =>
    els.map((el) => Math.round(el.getBoundingClientRect().top)),
  );
  expect(tops.length, "应有至少两张模块卡").toBeGreaterThan(1);
  expect(tops, "模块卡应纵向依次排开").toEqual([...tops].sort((a, b) => a - b));
  expect(new Set(tops).size, "纵向列表里不应有两张卡在同一行").toBe(tops.length);
});

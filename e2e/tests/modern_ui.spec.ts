import type { Page } from "@playwright/test";
import { expect, settleFrames, settleReveal, test } from "./fixtures";

// 双设计系统的回归：现代风 / 像素风互不串味、切换不刷新、无横向溢出。
// 判据见 `docs/ui/modern.md` §7（门禁与自检）与 `docs/ui/decisions.md` §8（两套皮肤互不串的验证方法）。

/** 预置界面风格与明暗（在页面脚本执行前写入 localStorage）。 */
async function seed(page: Page, style: "modern" | "pixel") {
  await page.addInitScript((s) => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", s);
    localStorage.setItem("theme", s === "modern" ? "dark" : "light");
  }, style);
}

/** `main` 里是否出现现代风的类名（`md-` 前缀）。 */
function hasModernClass(page: Page) {
  return page.evaluate(() =>
    Array.from(document.querySelectorAll("main *")).some((el) =>
      el.className?.toString().includes("md-"),
    ),
  );
}

test("现代风：首页 / 练习 / 题库浏览都是现代组件（无像素卡外壳）", async ({ page }) => {
  await seed(page, "modern");
  for (const url of ["/", "/practice", "/browse"]) {
    await page.goto(url);
    await expect(page.locator(".md-root").first()).toBeVisible();
    // 迁移过的页面不应再出现像素卡容器（桥接的是未迁移页面，这三页是专属实现）。
    await expect(page.locator("main .pxl-card")).toHaveCount(0);
  }
});

test("像素风：main 里没有任何现代类名（零污染）", async ({ page }) => {
  await seed(page, "pixel");
  for (const url of ["/", "/practice", "/browse"]) {
    await page.goto(url);
    expect(await hasModernClass(page)).toBe(false);
  }
});

test("两种宽度下都没有横向溢出", async ({ page }) => {
  await seed(page, "modern");
  for (const viewport of [
    { width: 1280, height: 900 },
    { width: 390, height: 844 },
  ]) {
    await page.setViewportSize(viewport);
    for (const url of ["/", "/practice", "/browse", "/glossary"]) {
      await page.goto(url);
      await page.waitForTimeout(400);
      const overflow = await page.evaluate(() => document.body.scrollWidth - window.innerWidth);
      expect(overflow, `${url} @${viewport.width}`).toBeLessThanOrEqual(0);
    }
  }
});

test("现代风：入场动画不会把内容永久藏住（工具 / 题库 / 术语）", async ({ page }) => {
  await seed(page, "modern");
  // 浮现的初始态是 `opacity: 0`，一旦观察器漏掉某个元素，内容就永久不可见 ——
  // 这是所有入场动画里唯一「不能靠肉眼一眼看出」的失败模式（工具页整页空白踩过一次）。
  for (const url of ["/tools", "/browse", "/glossary"]) {
    await page.goto(url);
    // 登记分三趟（rAF / 220ms / 640ms）：慢机上主线程被题库索引占住时，最后一趟会晚到，
    // 固定等 1200ms 会读到「一个都还没登记」。判据不变（必须有元素被登记），只是给它时间。
    await expect
      .poll(() => page.locator("[data-reveal]").count(), { timeout: 20_000 })
      .toBeGreaterThan(0);
    await settleReveal(page);
    const { total, stuck } = await page.evaluate(() => {
      const vh = window.innerHeight;
      const els = [...document.querySelectorAll("[data-reveal]")];
      return {
        total: els.length,
        stuck: els.filter((el) => {
          const r = el.getBoundingClientRect();
          const inView = r.top < vh * 0.9 && r.bottom > 0 && r.height > 0;
          return inView && getComputedStyle(el).opacity === "0";
        }).length,
      };
    });
    // 正常动效下页面一定有被登记的浮现元素；为 0 说明观察器根本没跑，下面的断言就成了空话。
    expect(total, `${url} 没有任何 data-reveal 元素`).toBeGreaterThan(0);
    expect(stuck, `${url} 有元素停在透明态`).toBe(0);
  }
});

test("顶栏搜索与 `/` 打开站内搜索（内容 + 页面），不是调谐台", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");
  await page.getByRole("button", { name: /^搜索/ }).first().click();
  const dialog = page.locator('[role="dialog"]').first();
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAttribute("aria-label", "全站搜索");
  // 调谐台（路由跳转）不该被顺带打开：两者是不同入口。
  await expect(page.locator(".md-palette")).toHaveCount(0);

  await dialog.locator("input").first().fill("术语");
  await expect(dialog.locator(".md-group-title").first()).toBeVisible();
  await expect(dialog.getByText("术语表", { exact: true }).first()).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page.locator('[role="dialog"]')).toHaveCount(0);

  // `/` 与按钮同一个弹层（像素风一直是这个约定，现代风不能再分叉）。
  await page.keyboard.press("/");
  await expect(page.locator('[role="dialog"]').first()).toBeVisible();
});

test("⌘K 打开调谐台，输入后能跳到目标页", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");
  await page.keyboard.press("Control+k");
  const input = page.locator(".md-palette-input");
  await expect(input).toBeVisible();
  await input.fill("术语");
  await page.waitForTimeout(300);
  await expect(page.locator(".md-palette-item").first()).toBeVisible();
  await page.locator(".md-palette-item").first().click();
  await expect(page.locator(".md-palette")).toHaveCount(0);
  expect(new URL(page.url()).pathname).not.toBe("/");
});

test("Portal 出来的弹层在现代风下也是现代外观（遮罩 / 按钮 / 字体）", async ({ page }) => {
  await seed(page, "modern");
  // 日志页的「同步 QSL」用的是像素风 `ui::Dialog`：它经 Portal 挂在 body 上，
  // 桥接规则如果带上 `.md-root` 限定就会整条落空 —— 表现为「现代页面里弹出像素窗体」。
  await page.goto("/log");
  await page.getByRole("button", { name: "同步 QSL" }).first().click();
  const dialog = page.locator('[role="dialog"]').first();
  await expect(dialog).toBeVisible();

  const look = await page.evaluate(() => {
    const dlg = document.querySelector('[role="dialog"]')!;
    // 遮罩类名按风格分叉（现代 `.md-dialog-veil` / 像素 `.pxl-overlay`），
    // 断言只关心「现代风用的是哪一套」，不再写死像素类名。
    const overlay = document.querySelector(".md-dialog-veil, .pxl-overlay")!;
    const btn = dlg.querySelector("button")!;
    const dcs = getComputedStyle(dlg);
    const ocs = getComputedStyle(overlay);
    const bcs = getComputedStyle(btn);
    return {
      overlayImage: ocs.backgroundImage,
      panelRadius: parseFloat(dcs.borderRadius),
      panelFont: dcs.fontFamily,
      btnRadius: parseFloat(bcs.borderRadius),
      btnShadow: bcs.boxShadow,
    };
  });

  // 遮罩不能还是 4px 抖动网点；窗体与按钮要圆角、不能有 2px 硬投影。
  expect(look.overlayImage).toBe("none");
  expect(look.panelRadius).toBeGreaterThanOrEqual(10);
  expect(look.panelFont).toContain("Geist");
  expect(look.btnRadius).toBeGreaterThanOrEqual(10);
  expect(look.btnShadow).not.toContain("2px 2px");
});

test("现代风抽屉可按 Esc 关闭（设置 / 目录面板）", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");
  await page.getByRole("button", { name: "外观与动效" }).first().click();
  await expect(page.locator(".md-sheet")).toHaveCount(1);
  await page.keyboard.press("Escape");
  await expect(page.locator(".md-sheet")).toHaveCount(0);

  await page.getByRole("button", { name: "更多" }).first().click();
  await expect(page.locator(".md-sheet")).toHaveCount(1);
  await page.keyboard.press("Escape");
  await expect(page.locator(".md-sheet")).toHaveCount(0);
});

// 目录面板里的「全站搜索」与设置入口是两个不同的信号。它们曾经都经
// `expect_context::<RwSignal<bool>>()` 取（同类型 → 拿到同一个），于是点「全站搜索」
// 打开的是设置面板。现在设置开关改成 `ModernMenu` 的 prop，这里守住两条入口的区别。
test("现代风目录面板：全站搜索开搜索弹层、设置开抽屉", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");
  await page.getByRole("button", { name: "更多" }).first().click();

  await page.getByRole("button", { name: "全站搜索" }).click();
  await expect(page.getByRole("dialog", { name: "全站搜索" })).toBeVisible();
  await expect(page.locator(".md-sheet")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "全站搜索" })).toHaveCount(0);

  // 目录面板底部的设置入口：中文界面下它的可见文字与顶栏齿轮按钮同为「外观与动效」。
  await page.getByRole("button", { name: "更多" }).first().click();
  await page.locator(".md-sheet").getByRole("button", { name: "外观与动效" }).click();
  await expect(page.locator(".md-sheet").getByRole("dialog", { name: "外观与动效" })).toBeVisible();
});

test("切换风格不刷新即生效（像素风 → 出现像素卡，页面未重载）", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");
  await expect(page.locator(".md-root").first()).toBeVisible();

  // 打一个只在本轮文档生命周期里存在的标记：重载后就没了。
  await page.evaluate(() => {
    (window as unknown as { __styleMark?: number }).__styleMark = 42;
  });

  await page.getByRole("button", { name: "外观与动效" }).first().click();
  await page.getByRole("button", { name: "像素风" }).click();

  await expect
    .poll(() => page.evaluate(() => document.documentElement.dataset.uiStyle))
    .toBe("pixel");
  await expect.poll(() => page.locator("main .pxl-card").count()).toBeGreaterThan(0);
  expect(
    await page.evaluate(() => (window as unknown as { __styleMark?: number }).__styleMark),
  ).toBe(42);
});

// 回归：像素风的自绘滚动条在切到现代风时被卸载，它挂在 `window` 上的 scroll / resize 监听
// 曾没有随之摘除 —— 卸载后的第一次滚动会读已释放的 `NodeRef`，触发 panic 兜底页。
test("像素风 → 现代风后滚动页面不报错（自绘滚动条卸载干净）", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });

  await seed(page, "pixel");
  await page.goto("/glossary");
  await expect(page.locator("main h1").first()).toBeVisible();

  await page.getByRole("button", { name: "外观与动效" }).first().click();
  await page.getByRole("radio", { name: "现代风" }).click();
  await expect
    .poll(() => page.evaluate(() => document.documentElement.dataset.uiStyle))
    .toBe("modern");
  await expect(page.locator(".pxl-scrollbar")).toHaveCount(0);

  await page.keyboard.press("Escape");
  for (const y of [400, 1200, 0]) {
    await page.evaluate((top) => window.scrollTo(0, top), y);
    await page.waitForTimeout(80);
  }
  await page.setViewportSize({ width: 900, height: 700 });
  await page.waitForTimeout(120);

  expect(errors, errors.join("\n")).toEqual([]);
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
});

// 逐页签名（`data-sig`）在像素风下也会写到 `#route-content`，所以 interaction.css 里
// 按签名分支的规则必须自带 `html[data-ui-style="modern"]` 门禁。少了门禁像素页会凭空
// 长出蓝色扫描带与现代蓝描边 —— 这一类「不是类名、而是计算样式」的串味，
// `main` 里查 `md-` 类名是看不见的（C7.7）。
test("像素风：逐页签名样式不外泄（lab 卡片 / map 标记）", async ({ page }) => {
  await seed(page, "pixel");
  await page.goto("/waveform-lab");
  const card = page.locator("#wave").first();
  await expect(card).toBeVisible();
  const labLook = await card.evaluate((el) => {
    const cs = getComputedStyle(el);
    return { background: cs.backgroundImage, animation: cs.animationName };
  });
  expect(labLook.background, "像素风卡片不该有现代风的扫描带渐变").not.toContain("linear-gradient");

  // 地图标记用合成节点判：不依赖 DXCC 数据什么时候取回来，只问「这条规则会不会命中像素 DOM」。
  const mapLook = await page.evaluate(() => {
    const host = document.getElementById("route-content");
    if (!host) return null;
    const previous = host.getAttribute("data-sig");
    host.setAttribute("data-sig", "map");
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("role", "img");
    const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    circle.setAttribute("r", "2");
    const title = document.createElementNS("http://www.w3.org/2000/svg", "title");
    title.textContent = "marker";
    circle.appendChild(title);
    svg.appendChild(circle);
    host.appendChild(svg);
    const cs = getComputedStyle(circle);
    const look = { cursor: cs.cursor, transition: cs.transitionProperty };
    svg.remove();
    if (previous === null) host.removeAttribute("data-sig");
    else host.setAttribute("data-sig", previous);
    return look;
  });
  expect(mapLook, "#route-content 应存在").not.toBeNull();
  expect(mapLook!.transition).not.toContain("stroke-width");
  expect(mapLook!.cursor).not.toBe("pointer");
});

// `aria-modal="true"` 声明了「背景是 inert」，就必须真的把 Tab 关在面板里（C7.8）。
test("现代风弹层：焦点进面板、Tab 不跑出去、关闭后归还", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");
  const trigger = page.getByRole("button", { name: "外观与动效" }).first();
  await trigger.click();
  const panel = page.locator("[data-modal-root]").first();
  await expect(panel).toBeVisible();

  const focusInside = () =>
    page.evaluate(() => Boolean(document.activeElement?.closest("[data-modal-root]")));
  await expect.poll(focusInside).toBe(true);

  // 连按 Tab：焦点始终留在面板内（面板内可交互元素少于 12 个，必然走到过循环边界）。
  for (let i = 0; i < 12; i += 1) {
    await page.keyboard.press("Tab");
    expect(await focusInside(), `第 ${i + 1} 次 Tab 跑出了面板`).toBe(true);
  }
  // 反向同理。
  for (let i = 0; i < 3; i += 1) {
    await page.keyboard.press("Shift+Tab");
    expect(await focusInside(), `第 ${i + 1} 次 Shift+Tab 跑出了面板`).toBe(true);
  }

  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await expect
    .poll(() =>
      page.evaluate(() => document.activeElement?.getAttribute("aria-label") ?? ""),
    )
    .toBe("外观与动效");
});

test("通联日志：所有下拉的尺寸与箭头一致（两个组件同类控件不分裂）", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/log");
  await expect(page.locator("main .md-root, .md-root").first()).toBeVisible();
  const boxes = await page.locator('main [role="combobox"]').evaluateAll((els) =>
    els.map((el) => {
      const r = el.getBoundingClientRect();
      const svg = el.querySelector("svg")?.getBoundingClientRect();
      return {
        h: Math.round(r.height),
        chev: svg ? `${Math.round(svg.width)}x${Math.round(svg.height)}` : null,
      };
    }),
  );
  // 控件高度取 `--md-control-h`（40px），并且**必须等于输入框高度** —— 这才是
  // `Select` 与 `SimpleSelect` 不分裂的真正判据，写死数字会在 token 调整时误报。
  const inputH = await page
    .locator("main .md-input")
    .first()
    .evaluate((el) => Math.round(el.getBoundingClientRect().height));
  const token = await page.evaluate(() =>
    getComputedStyle(document.documentElement).getPropertyValue("--md-control-h").trim(),
  );
  expect(token).toBe("40px");
  expect(boxes.length).toBeGreaterThan(2);
  for (const box of boxes) {
    expect(box.h).toBe(inputH);
    expect(box.chev).toBe("16x16");
  }
});

test("下拉选项：选中项没有像素对勾（选中态只靠蓝底 + 加粗）", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/log");
  const combo = page.locator('main [role="combobox"]').nth(5);
  await combo.click();
  const option = page.locator('[role="option"][aria-selected="true"]').first();
  await expect(option).toBeVisible();
  // 选项行在两套皮肤下都是同一个结构（`SelectItem`），不再渲染单独的对勾元素：
  // 以前数据式下拉多画一枚绝对定位的像素对勾，现代风只能用一条层外 CSS 把它藏起来，像素风里
  // 则和 `.pxl-item` 自带的三角光标叠成一团。这里断言它**根本不存在**，而不是「存在但被隐藏」。
  await expect(option.locator("> span.absolute")).toHaveCount(0);
  // 选中态只靠蓝色左轨：行内除了文字容器没有别的元素。
  await expect(option.locator("> *")).toHaveCount(1);
});

test("确认框：现代风是 md-modal-panel，Esc 可关闭", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/bookmarks");
  await page.getByRole("textbox").first().fill("临时组");
  await page.getByRole("button", { name: "新建分组" }).first().click();
  await page.getByRole("button", { name: "删除该分组" }).first().click();

  const panel = page.locator(".md-modal-panel");
  await expect(panel).toBeVisible();
  await expect(panel.locator(".md-btn-danger")).toHaveCount(1);
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
});

test("主导航菜单跳转是 SPA：不重载、无启动加载、菜单自动收起", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");
  await expect(page.locator(".md-nav").first()).toBeVisible();
  // 在页面上留一个标记：若是整页跳转（`<a href>` 未被路由拦截），标记会消失。
  await page.evaluate(() => {
    (window as unknown as { __spa?: string }).__spa = "alive";
  });

  await page.locator('button.md-nav-link:has-text("更多")').click();
  await expect(page.locator(".md-menu-panel")).toBeVisible();
  await page.locator('.md-menu-panel a[href="/browse"]').first().click();

  await expect(page).toHaveURL(/\/browse$/);
  expect(await page.evaluate(() => (window as unknown as { __spa?: string }).__spa)).toBe(
    "alive",
  );
  // 面板要在跳转后收起，不能浮在新页面上面。
  await expect(page.locator(".md-sheet-panel")).toHaveCount(0);
});

test("共享组件：统计卡与页签在现代风下是专属实现（不是桥接）", async ({ page }) => {
  await seed(page, "modern");
  // 统计卡：/bands 顶部四格
  await page.goto("/bands");
  await expect(page.locator(".md-stat").first()).toBeVisible();
  await expect(page.locator(".md-stat .md-stat-value").first()).toBeVisible();
  expect(await page.locator("main .pxl-window").count()).toBe(0);

  // 页签：/morse 的训练视图切换
  await page.goto("/morse");
  await expect(page.locator(".md-tablist").first()).toBeVisible();
  await expect(page.locator('.md-tab[data-state="active"]').first()).toBeVisible();
});

test("现代风首页：今日开台清单三行 + 7 日瀑布七格", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");

  const rows = page.locator(".md-onair-row");
  await expect(rows).toHaveCount(3);
  await expect(rows.first()).toBeVisible();

  // 每行都是一个真链接（不是整块 div + 点击委托），键盘能逐个走到。
  for (const row of await rows.all()) {
    expect(await row.getAttribute("href")).toMatch(/^\//);
  }

  // 瀑布 7 格，整图有可访问名（读屏不依赖逐格 `title`）。
  await expect(page.locator(".md-wf-cell")).toHaveCount(7);
  await expect(page.locator('.md-wf[role="img"]')).toHaveAttribute("aria-label", /.+/);
});

test("现代风首页：实时传播用假接口渲染读数与波段条件", async ({ page }) => {
  await seed(page, "modern");
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
        // 白天 / 夜间各两条：本地小时决定取哪一组，断言不依赖跑测试的时区。
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

  const live = page.locator(".md-live");
  await expect(live).toBeVisible();
  // 三个读数按 SFI / A / K 排列。
  await expect(live.locator(".md-live-metric-value").nth(0)).toHaveText("118");
  await expect(live.locator(".md-live-metric-value").nth(1)).toHaveText("7");
  await expect(live.locator(".md-live-metric-value").nth(2)).toHaveText("2");
  // K=2 / SFI=118 按 core 的规则是「良好」。
  await expect(live.locator(".md-live-level-text")).toHaveText("良好");
  // 只显示当前时段那一组（2 条），不是把 4 条全铺出来。
  await expect(live.locator(".md-live-band")).toHaveCount(2);
  await expect(live.locator(".md-live-source")).toContainText("HamQSL");
});

test("现代风首页：/api/solar 失败时降级为可重试提示", async ({ page }) => {
  await seed(page, "modern");
  await page.route("**/api/solar", (route) => route.abort());
  await page.goto("/");

  const failed = page.locator(".md-live-failed");
  await expect(failed).toBeVisible();
  await expect(failed.getByRole("button", { name: "重试" })).toBeVisible();
  await expect(failed.getByRole("link", { name: /查看详情/ })).toBeVisible();
  // 失败时不再渲染半截读数面板，避免「0 / 0 / 0」被当成真实读数。
  await expect(page.locator(".md-live")).toHaveCount(0);
});

test("现代风首页：瀑布按最近 7 日作答量分档上色", async ({ page }) => {
  await seed(page, "modern");
  await page.addInitScript(() => {
    if (window !== window.top) return;
    const pad = (n: number) => String(n).padStart(2, "0");
    const fmt = (d: Date) =>
      `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    const days: Record<string, { answered: number; new: number; duration_ms: number }> = {};
    // 倒数第 3 天做得最多：它应当是满档，其余更浅。
    for (const [back, answered] of [[4, 2], [3, 18], [1, 6]] as const) {
      const d = new Date();
      d.setDate(d.getDate() - back);
      days[fmt(d)] = { answered, new: answered, duration_ms: answered * 9000 };
    }
    localStorage.setItem("study-daily", JSON.stringify({ days }));
  });
  await page.goto("/");

  const cells = page.locator(".md-wf-cell");
  await expect(cells).toHaveCount(7);
  // 最大值那天是 5 档（满档），至少还有一档更浅的中间色。
  await expect(page.locator('.md-wf-cell[data-level="5"]')).toHaveCount(1);
  const levels = await cells.evaluateAll((els) =>
    els.map((el) => Number(el.getAttribute("data-level"))),
  );
  expect(levels.filter((lv) => lv > 0 && lv < 5).length).toBeGreaterThanOrEqual(2);
});

// 现代版答题卡与像素版共用取数（core 的 `answer_card`）。像素版的 e2e 靠 `div.grid button`，
// 那条路径在现代风下摸不到，这里单独守一遍：筛选、跳转、格子名字里带状态。
test("现代风答题卡：筛选、跳转与格子的无障碍名", async ({ page }) => {
  await seed(page, "modern");
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:shortcutsHelpSeen:exam", "1");
  });
  await page.goto("/exam");
  await expect(page.locator(".md-root").first()).toBeVisible();

  // 标记第 1 题、作答一题。现代风的单选 / 多选框都视觉隐藏（`pointer-events: none`），可点的是
  // 整行（`fixtures.option` 点的是控件本身，这里用不了）。抽到单选还是多选是随机的：点行的中心，
  // 对两种题型都落在可点区域内（多选题的行就是 label；单选题的行里 label 铺满了整行，见 R19）。
  // 不能点行内的 `strong`：单选题里它被 label 的覆盖层盖住，Playwright 的命中检测会一直重试到超时。
  await page.getByRole("button", { name: "标记", exact: true }).first().click();
  await page.locator("main .md-opt").first().click();
  await expect(page.getByText(/已作答\s*1\s*\/\s*40/).first()).toBeVisible();

  await page.getByRole("button", { name: "答题卡" }).first().click();
  const card = page.getByRole("dialog", { name: "答题卡" });
  await expect(card).toBeVisible();
  const cells = card.locator(".md-cell");
  await expect(cells).toHaveCount(40);

  // 当前题 + 标记：名字里要带状态（以前只有颜色 / 角标，读屏听不到）。
  await expect(cells.first()).toHaveAttribute("aria-current", "true");
  await expect(cells.first()).toHaveAccessibleName(/第 1 题.*已标记/);
  // 可见的题号就是 "1"；角标 `!` 是装饰（`aria-hidden`），状态已经写进上面的名字里。
  await expect(cells.first().locator(".md-cell-no")).toHaveText("1");
  await expect(cells.first().locator(".md-cell-flag")).toHaveAttribute("aria-hidden", "true");

  // 筛选：未答 39 题；标记 1 题；而且筛选不改格子对应的题号。
  await card.getByRole("button", { name: "未答", exact: true }).click();
  await expect(cells).toHaveCount(39);
  await card.getByRole("button", { name: "标记", exact: true }).click();
  await expect(cells).toHaveCount(1);
  await expect(cells.first().locator(".md-cell-no")).toHaveText("1");

  // 「下一个未答」从当前题往后找，落在第 2 题。
  await card.getByRole("button", { name: "全部" }).click();
  await card.getByRole("button", { name: "下一个未答" }).click();
  await expect(card).toBeHidden();
  await expect(page.getByText(/第 2 \/ 40 题/).first()).toBeVisible();

  // 点第 10 格直接跳到第 10 题。
  await page.getByRole("button", { name: "答题卡" }).first().click();
  await card.getByRole("button", { name: "全部" }).click();
  await cells.nth(9).click();
  await expect(card).toBeHidden();
  await expect(page.getByText(/第 10 \/ 40 题/).first()).toBeVisible();
});

// 版式不能依赖 `:has()`（Firefox 120 及以下不支持）：练习 / 考试页的形态由 Rust 写在 <body> 上。
// 判据是标记的生命周期：进入时有、练习 → 考试这类跳转里不被旧页面的清理抹掉、离开后没有。
test("页面形态标记：进入、切换、离开都对（body[data-page-mode]）", async ({ page }) => {
  await seed(page, "modern");
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:shortcutsHelpSeen:exam", "1");
    localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
  });
  const mode = () => page.locator("body").getAttribute("data-page-mode");

  await page.goto("/practice");
  await expect(page.locator(".md-practice")).toBeVisible();
  await expect.poll(mode).toBe("practice");

  // 站内跳到考试页（调谐台，不整页重载）：新页面先写、旧页面后清，必须仍是 exam。
  await page.keyboard.press("Control+k");
  await page.locator(".md-palette-input").fill("模拟考试");
  const item = page.locator(".md-palette-item").filter({ hasText: "/exam" }).first();
  await expect(item).toBeVisible();
  await item.click();
  await expect(page.locator(".md-exam")).toBeVisible();
  await expect.poll(mode).toBe("exam");

  // 回首页：两种形态的标记都应已清掉。
  await page.locator('a[href="/"]').first().click();
  await expect(page.locator(".md-practice, .md-exam")).toHaveCount(0);
  await expect.poll(mode).toBeNull();
});

// 输入框的图标内边距以前靠 `.md-input-wrap:has(.md-adorn-left)`；老浏览器没有它，图标就压在文字上。
test("带前缀图标的输入框让出左内边距（属性选择器，不依赖 :has）", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");
  // 站内搜索弹层里的输入框用了 `ui::Input` 的前缀槽（页面里的搜索框在现代风下是 `ModernInput`，没有图标槽）。
  await page.keyboard.press("/");
  const wrap = page.locator(".md-input-wrap[data-adorn-left]").first();
  await expect(wrap).toBeVisible();
  await expect(wrap.locator(".md-adorn-left")).toBeVisible();
  await expect(wrap).not.toHaveAttribute("data-adorn-right", /.*/);
  const pad = await wrap.locator(".md-input").evaluate((el) => parseFloat(getComputedStyle(el).paddingInlineStart));
  expect(pad, "有前缀图标时输入框左内边距应为 30px").toBe(30);
});

// 单选题的整行都要能点：以前只有文字宽度的 `<label>` 可点，行右侧空白与内边距点了没反应，
// 而多选题的行本身就是 `<label>`、点哪都算 —— 两种题型手感不一致。
// 判据是「落点命中 label 且真的选中」，而不只看样式：伪元素被别的规则盖住时样式在、点击却丢。
test("现代风选项行：单选题整行可点，交卷后的只读态不可点", async ({ page }) => {
  await seed(page, "modern");
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
    localStorage.setItem("ui:shortcutsHelpSeen:exam", "1");
  });

  // 练习页：翻到一道单选题（题型是随机的，翻到才有得测，翻不到就红，不悄悄跳过）。
  await page.goto("/practice");
  await expect(page.locator("main .md-opt").first()).toBeVisible();
  const radios = page.locator('main [role="radio"]');
  for (let i = 0; i < 60 && (await radios.count()) === 0; i++) {
    await page.getByRole("button", { name: /下一题/ }).first().click();
    await settleFrames(page);
  }
  await expect(radios.first(), "60 题内应出现单选题").toBeAttached();

  const rows = page.locator("main .md-opt");
  const checked = () =>
    radios.evaluateAll((els) => els.map((e) => e.getAttribute("aria-checked") === "true"));
  // 行内三个落点：中部空白、右侧空白、左内边距。每次点完看选中的是不是这一行。
  for (const [name, at] of [
    ["右侧空白", (r: { x: number; width: number }) => r.x + r.width * 0.9],
    ["中部空白", (r: { x: number; width: number }) => r.x + r.width * 0.6],
    ["左内边距", (r: { x: number; width: number }) => r.x + 6],
  ] as const) {
    const row = 1;
    // 先选别的行，再点目标落点：避免「本来就选着」造成假通过。
    await rows.nth(0).locator("label").click();
    await expect.poll(async () => (await checked())[row]).toBe(false);
    // 坐标必须在「先选别的行」**之后**再量：点选会让页面重排（答案解析等内容插入），
    // 之前量的坐标会过期，点击落到别处 —— 那是用例的竞态，不是功能丢点击。
    await settleFrames(page);
    const box = await rows.nth(row).boundingBox();
    if (!box) throw new Error("选项行不可见");
    const x = at(box);
    const y = box.y + box.height / 2;
    const hit = await page.evaluate(([px, py]) => document.elementFromPoint(px, py)?.tagName, [x, y]);
    expect(hit, `${name}应命中 label（伪元素铺满了行）`).toBe("LABEL");
    await page.mouse.click(x, y);
    await expect.poll(async () => (await checked())[row], `点${name}应选中这一行`).toBe(true);
  }

  // 考试页：交卷后进入只读回顾，点行内空白不能改答案，光标不是手型。
  await page.goto("/exam");
  await expect(page.locator("main .md-opt").first()).toBeVisible();
  await page.getByRole("button", { name: "交卷", exact: true }).first().click();
  await page
    .getByRole("dialog", { name: "确认交卷？" })
    .getByRole("button", { name: "确认交卷" })
    .click();
  await page.keyboard.press("Escape");
  const first = page.locator("main .md-opt").first();
  await expect(first).toBeVisible();
  const readonlyInfo = await first.evaluate((row) => {
    const label = row.querySelector("label");
    return {
      // 多选题的行本身就是 label（`row.tagName === "LABEL"`），没有内层 label 可量。
      cursor: label && row.tagName !== "LABEL" ? getComputedStyle(label, "::before").cursor : null,
      disabled: (row.querySelector('[role="radio"],[role="checkbox"]') as HTMLButtonElement | null)?.disabled,
    };
  });
  expect(readonlyInfo.disabled, "交卷后选项应为只读").toBe(true);
  if (readonlyInfo.cursor !== null) {
    expect(readonlyInfo.cursor, "只读态的整行覆盖层不应显示手型").toBe("default");
  }
  const marks = page.locator('main [role="radio"], main [role="checkbox"]');
  const before = await marks.evaluateAll((els) => els.map((e) => e.getAttribute("aria-checked")));
  const box = await first.boundingBox();
  if (!box) throw new Error("选项行不可见");
  await page.mouse.click(box.x + box.width * 0.9, box.y + box.height / 2);
  await settleFrames(page);
  expect(await marks.evaluateAll((els) => els.map((e) => e.getAttribute("aria-checked")))).toEqual(before);
});

// ---------------------------------------------------------------------------------------------
// `.md-link`（行内文字链接）与 `.md-link-row`（首页入口列表的一行）曾共用一个类名：全站 36 个行内链接被一并
// 撑成 `flex` + 36px 高 + `margin-inline: -8px` 的「列表行」（`/radio-law` 一页 22 个，`/developers` 段内链接的
// 盒子比整段还宽 16px）。i18n_layout 的现代风档把它报成「文本裁剪」才被发现 —— 而那一档此前从没人跑过。
// ---------------------------------------------------------------------------------------------

/** 行内链接的几何：高度对比所在行高、外边距、是否比父容器更宽。 */
function inlineLinkGeometry(page: Page) {
  return page.evaluate(() =>
    Array.from(document.querySelectorAll("main a.md-link"))
      .filter((a) => !a.closest(".md-link-list"))
      .map((a) => {
        const c = getComputedStyle(a);
        const r = a.getBoundingClientRect();
        const pr = (a.parentElement as Element).getBoundingClientRect();
        const lineH = parseFloat(getComputedStyle(a.parentElement as Element).lineHeight) || parseFloat(c.fontSize) * 1.5;
        return {
          text: (a.textContent ?? "").trim().slice(0, 16),
          h: r.height,
          lineH,
          marginL: c.marginLeft,
          marginR: c.marginRight,
          overflowsParent: r.right - pr.right,
        };
      }),
  );
}

for (const route of ["/developers", "/mistake-topics", "/radio-law"]) {
  test(`现代风：${route} 的行内链接是文字，不是 36px 的列表行`, async ({ page }) => {
    await seed(page, "modern");
    await page.goto(route);
    await expect(page.locator("main a.md-link").first()).toBeVisible();
    const links = await inlineLinkGeometry(page);
    expect(links.length, "页面上应有行内链接（否则这条用例什么也没测）").toBeGreaterThan(0);
    for (const l of links) {
      expect(l.h, `「${l.text}」高 ${l.h}px，比所在行高 ${l.lineH}px 的 1.3 倍还高 —— 被撑成了列表行`).toBeLessThanOrEqual(l.lineH * 1.3);
      expect(l.marginL, `「${l.text}」不该有负外边距`).toBe("0px");
      expect(l.marginR).toBe("0px");
      expect(l.overflowsParent, `「${l.text}」比父容器还宽`).toBeLessThanOrEqual(1);
    }
  });
}

test("现代风首页：入口列表的一行是 md-link-row，出血与悬停反馈都还在", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");
  const rows = page.locator(".md-link-list > a");
  await expect(rows.first()).toBeVisible();
  // 列表里不再有行内链接的类名（两个组件不共用类名）。
  await expect(page.locator(".md-link-list a.md-link")).toHaveCount(0);
  expect(await rows.count()).toBeGreaterThanOrEqual(3);
  const first = rows.first();
  const css = () =>
    first.evaluate((el) => {
      const c = getComputedStyle(el);
      return { cls: el.className, minH: c.minHeight, marginL: c.marginLeft, padL: c.paddingLeft, color: c.color, bg: c.backgroundColor };
    });
  const rest = await css();
  expect(rest.cls).toContain("md-link-row");
  // 有意的出血：`padding-inline: 8px` + `margin-inline: -8px`，悬停底色比文字宽一圈，文字仍与卡片内容对齐。
  expect(rest).toMatchObject({ minH: "36px", marginL: "-8px", padL: "8px" });
  await first.hover();
  // 底色与文字色是同一时刻起步的 120ms 过渡，但各自的第一帧不同步：两个都要轮询，
  // 不能等其中一个变了就立刻去读另一个（那样会偶发读到过渡起点的旧值）。
  await expect.poll(async () => (await css()).bg, { message: "悬停应出现底色" }).not.toBe(rest.bg);
  await expect.poll(async () => (await css()).color, { message: "悬停应换成前景色" }).not.toBe(rest.color);
});

// 西语长标签 + 320px：`.md-btn` 是固定高度、强制单行的药丸，标签一长就比所在行还宽，把整页撑出横向滚动。
// `mobile_overflow` 的现代风档（`E2E_UI_STYLE=modern`）才扫得到，而那一档此前没人跑过。
for (const route of ["/backup", "/nec"]) {
  test(`现代风 西语 320px：${route} 的长标签按钮不把页面撑出横向滚动`, async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 900 });
    await page.addInitScript(() => {
      if (window !== window.top) return;
      localStorage.setItem("locale", "es");
      localStorage.setItem("ui:uiStyle", "modern");
      localStorage.setItem("theme", "dark");
    });
    await page.goto(route);
    const wrapped = page.locator("main button.md-btn-wrap").first();
    await expect(wrapped, "长标签按钮应带 md-btn-wrap（kit::btn_wrap_class）").toBeVisible();
    await settleFrames(page);
    const over = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(over, "文档横向溢出").toBeLessThanOrEqual(0);
    // 标签没有溢出按钮自己的盒子（折行而不是溢出）。
    const spill = await page.locator("main button.md-btn-wrap").evaluateAll((els) =>
      els.map((e) => e.scrollWidth - e.clientWidth).filter((d) => d > 1),
    );
    expect(spill, "标签溢出了按钮的盒子").toEqual([]);
  });
}

test("现代风：短标签的按钮加了 md-btn-wrap 也不变（高度仍是控件高）", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/backup");
  const btns = page.locator("main button.md-btn-wrap");
  await expect(btns.first()).toBeVisible();
  const heights = await btns.evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().height)));
  // 中文标签都放得下一行：每个按钮都应是 `--md-control-h`（40px），不因为允许换行而变高或变矮。
  // 不写死数量 —— 备份页每加一个操作就会多一个 `md-btn-wrap`，写死 3 会跟着页面内容变红。
  expect(heights.length, "没量到带 md-btn-wrap 的按钮").toBeGreaterThan(0);
  expect(
    heights.every((h) => h === 40),
    `有按钮不是控件高（实际 ${heights.join(" / ")}）`,
  ).toBe(true);
});

// 分类浏览的题目行：标签行与题干分成两个块级行。
//
// 回归：标签与题干曾同处一个 flex 行，题干首行被推到胶囊右侧、换行后回到最左边，
// 左边缘呈锯齿（首行缩进、其余顶格）。用例量的是**题干每一行文本的左边缘**与标签行起点，
// 三者必须落在同一条竖线上 —— 只看卡片左内距发现不了，因为卡片与标签一直是齐的。
//
// 同时守住「卡片带分类标签」这件事：分类名来自 core 数据（`P` 码 → 二级 / 一级分类），
// 曾经只有像素风渲染它们，现代风只显示题号。
for (const width of [390, 1280] as const) {
  test(`现代风题目行：${width}px 下标签独占一行、题干左边缘对齐且带分类标签`, async ({ page }) => {
    await seed(page, "modern");
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/browse");
    const row = page.locator(".md-q-row").first();
    await expect(row).toBeVisible();
    await settleFrames(page);

    const m = await row.evaluate((el) => {
      const head = el.querySelector(".md-q-row-head");
      const tags = el.querySelector(".md-q-row-tags");
      const text = el.querySelector(".md-q-row-text");
      if (!head || !tags || !text) return { missing: true };
      // 逐行量题干：Range 的每一块 client rect 就是一行。
      const range = document.createRange();
      range.selectNodeContents(text);
      const lines = [...range.getClientRects()].filter((r) => r.width > 1 && r.height > 1);
      return {
        missing: false,
        tagCount: el.querySelectorAll(".md-q-row-tag").length,
        // 分类标签的可见文字：一级 + 二级分类名，两枚都要有内容。
        tagTexts: [...el.querySelectorAll(".md-q-row-tag")].map((t) => (t.textContent ?? "").trim()),
        // 标签名不许被省略号截断（分类名是「这是什么题」的答案，截了就读不出来）
        truncatedTags: [...el.querySelectorAll(".md-q-row-tag")].filter(
          (t) => t.scrollWidth > t.clientWidth + 1,
        ).length,
        tagLefts: [...el.querySelectorAll(".md-q-row-tags > *")].map((t) =>
          Math.round(t.getBoundingClientRect().left),
        ),
        headLeft: Math.round(head.getBoundingClientRect().left),
        headBottom: head.getBoundingClientRect().bottom,
        textTop: text.getBoundingClientRect().top,
        textLeft: Math.round(text.getBoundingClientRect().left),
        lineLefts: lines.map((r) => Math.round(r.left)),
        // 卡片左右内沿（padding box 边界，含 1px 描边）：胶囊不得越过，题干必须从内沿起排
        innerLeft:
          el.getBoundingClientRect().left +
          parseFloat(getComputedStyle(el).borderLeftWidth) +
          parseFloat(getComputedStyle(el).paddingLeft),
        innerRight:
          el.getBoundingClientRect().right -
          parseFloat(getComputedStyle(el).borderRightWidth) -
          parseFloat(getComputedStyle(el).paddingRight),
        tagRights: [...el.querySelectorAll(".md-q-row-tags > *")].map((t) =>
          t.getBoundingClientRect().right,
        ),
        docOverflow: document.documentElement.scrollWidth - window.innerWidth,
      };
    });

    expect(m.missing, "题目行里找不到标签行 / 题干").toBe(false);
    if (m.missing) return;
    // 先证明真的渲染出了分类标签：否则下面「左边缘对齐」的断言在标签为 0 时也可通过。
    expect(m.tagCount, "题目行没有分类标签").toBeGreaterThanOrEqual(2);
    expect(m.tagTexts.length, "分类标签没有可见文字").toBeGreaterThanOrEqual(2);
    expect(m.tagTexts.every((t) => t.length > 0), `分类标签文字为空：${JSON.stringify(m.tagTexts)}`).toBe(true);
    expect(m.truncatedTags, "分类标签被截断了（英文 / 西语长分类名在窄屏放不下时应折行）").toBe(0);

    // 题干每一行的左边缘 == 题干盒子的左边缘 == 卡片内沿 —— 不再有「首行缩进」。
    expect(new Set(m.lineLefts).size, `题干各行左边缘不一致：${JSON.stringify(m.lineLefts)}`).toBe(1);
    expect(m.lineLefts[0], "题干没有从卡片内沿起排").toBe(m.textLeft);
    expect(m.textLeft, "题干没有落在卡片内沿").toBe(m.innerLeft);
    // 标签行自己也从同一条竖线起排。
    expect(m.headLeft, "标签行与题干不在同一条左边缘").toBe(m.textLeft);
    expect(m.tagLefts[0], "第一枚标签与题干不在同一条左边缘").toBe(m.textLeft);
    // 两个块级行：标签行的底边在题干顶部之上（同行时它们会重叠）。
    expect(m.headBottom, "题干与标签仍挤在同一行").toBeLessThanOrEqual(m.textTop + 1);
    // 胶囊收在卡片内：中文 / 长分类名都不许顶出去。
    expect(Math.max(...m.tagRights), "标签越过了卡片右内沿").toBeLessThanOrEqual(m.innerRight + 1);
    expect(m.docOverflow, "文档横向溢出").toBeLessThanOrEqual(0);
  });
}

// ---------------------------------------------------------------------------------------------
// 手机宽度下的版式与可读性回归。
// 都在 390px、粗指针下量**渲染结果**：类名换了但版式没坏时不该红，版式坏了类名还在时必须红。
// ---------------------------------------------------------------------------------------------
test.describe("手机宽度版式回归", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  // 关卡名是 `/map` 的主标识，单行 ellipsis 把「Radio regulations and …」截掉。
  for (const [locale, label] of [["en", "英文"], ["es", "西语"]] as const) {
    test(`闯关地图：${label}关卡名不被截断`, async ({ page }) => {
      await seed(page, "modern");
      await page.addInitScript((l) => localStorage.setItem("locale", l), locale);
      await page.goto("/map");
      await expect(page.locator(".md-stage-name").first()).toBeVisible();
      await settleFrames(page);
      const r = await page.evaluate(() => {
        const names = Array.from(document.querySelectorAll<HTMLElement>(".md-stage-name"));
        return {
          total: names.length,
          // 两个方向都要量：旧版式是单行 `text-overflow: ellipsis`（水平溢出，`scrollHeight` 不变），
          // 新版式是多行 `line-clamp`（垂直溢出）。只量一个方向，退回另一种写法时用例照样绿。
          cut: names
            .filter((e) => e.scrollWidth > e.clientWidth + 1 || e.scrollHeight > e.clientHeight + 1)
            .map((e) => e.textContent),
        };
      });
      expect(r.total, "没找到关卡").toBeGreaterThan(0);
      expect(r.cut, `这些关卡名被截断了：${r.cut.join(" | ")}`).toEqual([]);
    });
  }

  // 输入框 16px、答案却是 12px，读数层级是倒的。
  test("小工具：计算结果字号不小于输入框的 90%（≤640）", async ({ page }) => {
    await seed(page, "modern");
    await page.goto("/tools");
    await expect(page.locator(".md-tools-grid .bg-inset").first()).toBeVisible();
    await settleFrames(page);
    const r = await page.evaluate(() => {
      const result = document.querySelector(".md-tools-grid .bg-inset") as HTMLElement;
      const input = document.querySelector(".md-tools-grid input.md-input, .md-tools-grid input") as HTMLElement;
      return {
        result: parseFloat(getComputedStyle(result).fontSize),
        input: parseFloat(getComputedStyle(input).fontSize),
      };
    });
    expect(r.input, "没量到输入框字号").toBeGreaterThan(0);
    expect(r.result, `结果 ${r.result}px 比输入框 ${r.input}px 小太多：答案的层级比字段还低`).toBeGreaterThanOrEqual(
      r.input * 0.9,
    );
  });

  // 「更多字段」展开钮——现代风下是整行按钮：行高 1.4 以上、不带下划线、有 chevron、高度 ≥ 44。
  test("通联日志：「更多字段」是带 chevron 的整行按钮，不再是下划线粗体字", async ({ page }) => {
    await seed(page, "modern");
    await page.goto("/log");
    const btn = page.getByRole("button", { name: /更多字段/ });
    await expect(btn).toBeVisible();
    await btn.scrollIntoViewIfNeeded();
    const m = await btn.evaluate((el) => {
      const cs = getComputedStyle(el);
      const b = el.getBoundingClientRect();
      return {
        h: b.height,
        underline: cs.textDecorationLine,
        lh: parseFloat(cs.lineHeight) / parseFloat(cs.fontSize),
        hasIcon: !!el.querySelector("svg"),
        expanded: el.getAttribute("aria-expanded"),
      };
    });
    expect(m.h, "按钮高度低于触控目标").toBeGreaterThanOrEqual(44);
    expect(m.underline, "现代风下不该再用下划线当按钮").not.toContain("underline");
    expect(m.lh, "折行后行高太小，第二行会顶着第一行").toBeGreaterThanOrEqual(1.2);
    expect(m.hasIcon, "缺少展开 chevron").toBe(true);
    expect(m.expanded).toBe("false");
    await btn.click();
    await expect(btn).toHaveAttribute("aria-expanded", "true");
  });

  // 浏览页的「题库 + 筛选」胶囊在窄屏是一条横滚行；桌面仍与搜索框同处一行（display: contents）。
  test("题库浏览：窄屏筛选胶囊是一条横滚行，不折成两行；桌面版式不变", async ({ page }) => {
    await seed(page, "modern");
    await page.goto("/browse");
    const bar = page.locator(".md-chip-bar");
    await expect(bar).toBeVisible();
    const phone = await bar.evaluate((el) => {
      const cs = getComputedStyle(el);
      const tops = Array.from(el.children).map((c) => Math.round(c.getBoundingClientRect().top));
      return {
        overflowX: cs.overflowX,
        oneRow: new Set(tops).size === 1,
        scrolls: el.scrollWidth > el.clientWidth,
        docOverflow: document.documentElement.scrollWidth - innerWidth,
      };
    });
    expect(phone.overflowX).toBe("auto");
    expect(phone.oneRow, "胶囊折成了多行").toBe(true);
    expect(phone.scrolls, "胶囊放得下就不必横滚，用例失去意义").toBe(true);
    expect(phone.docOverflow, "横滚条不能把整页撑出横向滚动").toBeLessThanOrEqual(1);

    await page.setViewportSize({ width: 1280, height: 900 });
    await settleFrames(page);
    const desktop = await page.evaluate(() => {
      const barEl = document.querySelector(".md-chip-bar") as HTMLElement;
      const search = document.querySelector(".md-glossary-search") as HTMLElement;
      const chip = barEl.querySelector(".md-chip") as HTMLElement;
      return {
        display: getComputedStyle(barEl).display,
        sameRow: Math.abs(search.getBoundingClientRect().top - chip.getBoundingClientRect().top) < 30,
      };
    });
    expect(desktop.display, "桌面应是 display: contents（与没有这层包裹时一致）").toBe("contents");
    expect(desktop.sameRow, "桌面的胶囊应与搜索框同处一行").toBe(true);
  });

  // dock 是常驻导航，不是脚注。
  test("底部 dock 的标签字号不小于 12px（中文）", async ({ page }) => {
    await seed(page, "modern");
    await page.goto("/");
    await expect(page.locator(".md-dock-item").first()).toBeVisible();
    const sizes = await page
      .locator(".md-dock-item span")
      .evaluateAll((els) => els.map((e) => parseFloat(getComputedStyle(e).fontSize)));
    expect(sizes.length, "dock 里没有文字标签").toBeGreaterThan(0);
    for (const s of sizes) expect(s, `dock 标签 ${s}px 过小`).toBeGreaterThanOrEqual(12);
  });

  // F5-7：窄屏筛选行里的分组竖线换行后会变成孤立的竖线，窄屏直接不显示。
  test("练习页：窄屏没有孤立的分组竖线", async ({ page }) => {
    await seed(page, "modern");
    await page.goto("/practice?bank=A");
    await expect(page.locator(".md-chip-row").first()).toBeVisible();
    const display = await page
      .locator(".md-chip-row-divider")
      .first()
      .evaluate((e) => getComputedStyle(e).display);
    expect(display).toBe("none");
  });
});

// ---------------------------------------------------------------------------------------------
// 考试设置里的键位表：触屏上没有键盘可按，整段不占位；鼠标设备照常显示。
// 隐藏规则（`kit.css` 的 `.md-shortcut-list`）必须挂在现代风自己的弹层 DOM 上：
// 曾经把类名写进了像素风弹窗，CSS 却只作用于现代风 —— 两头都成立、却永远对不上。
// ---------------------------------------------------------------------------------------------
test.describe("现代风考试设置的键位表", () => {
  async function openExamSettings(page: Page) {
    await seed(page, "modern");
    await page.goto("/exam");
    await expect(page.locator("main .md-opt").first()).toBeVisible();
    await page.locator("main").getByRole("button", { name: "设置", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "设置" });
    await expect(dialog).toBeVisible();
    return dialog;
  }

  test("鼠标设备：键位表可见", async ({ page }) => {
    const dialog = await openExamSettings(page);
    await expect(dialog.locator(".md-shortcut-list"), "弹层里没有键位表，用例失去意义").toHaveCount(1);
    await expect(dialog.locator(".md-shortcut-list")).toBeVisible();
  });

  test.describe("触屏", () => {
    test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

    test("键位表整段不占位", async ({ page }) => {
      const dialog = await openExamSettings(page);
      // 先证明元素在 DOM 里：不在的话 `toBeHidden` 会空洞通过。
      await expect(dialog.locator(".md-shortcut-list"), "弹层里没有键位表，用例失去意义").toHaveCount(1);
      await expect(dialog.locator(".md-shortcut-list")).toBeHidden();
    });
  });
});

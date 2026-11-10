import type { Page } from "@playwright/test";
import { expect, settleFrames, settleReveal, test } from "./fixtures";

// 交互层回归：逐页签名、换页方向、按压反馈、可拖拽轨道、降级。
// 判据与 `docs/ui/interaction.md` §7 一一对应。

/** 预置界面风格（页面脚本执行前写入，避免首屏闪一次像素风）。 */
async function seed(page: Page, style: "modern" | "pixel") {
  await page.addInitScript((s) => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", s);
    localStorage.setItem("theme", s === "modern" ? "dark" : "light");
  }, style);
}

const SIGNATURES: [string, string][] = [
  ["/", "landing"],
  ["/practice", "practice"],
  ["/browse", "browse"],
  ["/glossary", "index"],
  ["/tools", "tools"],
  ["/ft8-decode", "lab"],
  ["/log", "log"],
  ["/radio-law", "read"],
  ["/stats", "board"],
  ["/grid-map", "map"],
  ["/exam", "exam"],
];

// 12 个签名各取一条代表路由（全表的唯一性与覆盖率由 core 的单测守着，见 `interaction.rs`）。
test("每个交互签名的代表路由都落位正确的签名", async ({ page }) => {
  await seed(page, "modern");
  for (const [url, sig] of SIGNATURES) {
    await page.goto(url);
    await expect(page.locator("#route-content"), `${url} 应为 ${sig}`).toHaveAttribute(
      "data-sig",
      sig,
    );
  }
});

test("首屏是 initial，站内前进是 forward，浏览器后退是 back", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/");
  await expect(page.locator("#route-content")).toHaveAttribute("data-nav-dir", "initial");

  // 走调谐台做站内导航（唯一的 View Transition 入口），避免整页重载。
  await page.keyboard.press("Control+k");
  const input = page.locator(".md-palette-input");
  await expect(input).toBeVisible();
  await input.fill("学习进度");
  // 等过滤结果真的收敛到目标项，而不是睡一个固定时长再点「第一项」。
  const target = page.locator(".md-palette-item").filter({ hasText: "/progress" }).first();
  await expect(target).toBeVisible();
  await target.click();
  await expect(page.locator("#route-content")).toHaveAttribute("data-nav-dir", "forward");
  expect(new URL(page.url()).pathname).toBe("/progress");

  await page.goBack();
  await expect(page.locator("#route-content")).toHaveAttribute("data-nav-dir", "back");
});

test("按压有涟漪（控件）且 600ms 后自清理", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/glossary");
  const chip = page.locator(".md-chip").first();
  // 入场浮现会把内容下移再上浮：没落位就按下，坐标在按下的那一刻已经
  // 过期，`pointerdown` 落到旁边元素上 —— 慢机上必现「按了没涟漪」。
  await settleReveal(page);
  // 落点交给 `hover()`：它会等元素稳定、滚动到可点位置，并校验这一点真的命中它。
  await chip.hover();
  await page.mouse.down();
  await expect(chip.locator(".md-ping")).toHaveCount(1);
  await page.mouse.up();
  // 涟漪节点必须自删：留着会随点击次数累积。
  await expect(chip.locator(".md-ping")).toHaveCount(0, { timeout: 2_000 });
});

test("平板宽度下模块卡是可拖拽轨道：拖动与方向键都能滚动", async ({ page }) => {
  await seed(page, "modern");
  // 640–899 才是甲板；<640 回退纵向列表（发现性优先：横向滑动会把 2/3 的入口藏在屏幕外），
  // 那条由 `layout_consistency.spec.ts` 守着。
  await page.setViewportSize({ width: 768, height: 1024 });
  await page.goto("/");

  // 首页的模块甲板同样可能在入场浮现里（下移再上浮），没落位时量的坐标按下去已经偏了。
  await settleReveal(page);
  const rail = page.locator(".md-modules.md-track");
  await expect(rail).toBeVisible();
  const overflow = await rail.evaluate((el) => el.scrollWidth - el.clientWidth);
  expect(overflow, "轨道在窄屏应该溢出，否则拖动没有意义").toBeGreaterThan(0);

  // 落点交给 `hover()`：自己算坐标时，慢机上页面还在长高（实时传播面板从骨架变成读数），
  // 按下会落到轨道外面，整个拖动静默失效。
  const size = await rail.evaluate((el) => ({ w: el.clientWidth, h: el.clientHeight }));
  await rail.hover({ position: { x: size.w - 24, y: size.h / 2 } });
  const box = await rail.boundingBox();
  if (!box) throw new Error("轨道不可见");
  const y = box.y + box.height / 2;
  await page.mouse.down();
  await page.mouse.move(box.x + 24, y, { steps: 12 });
  await page.mouse.up();

  const afterDrag = await rail.evaluate((el) => el.scrollLeft);
  expect(afterDrag, "拖动后应向右滚动").toBeGreaterThan(0);
  // 拖拽结束时的那次 click 必须被吞掉，否则「想看第二张」会变成「点进第一张」。
  expect(new URL(page.url()).pathname).toBe("/");

  await rail.focus();
  await page.keyboard.press("Home");
  await expect.poll(() => rail.evaluate((el) => el.scrollLeft)).toBe(0);
  await page.keyboard.press("ArrowRight");
  await expect.poll(() => rail.evaluate((el) => el.scrollLeft)).toBeGreaterThan(0);
});

// 回归：`pointerdown` 就捕获指针会把 click 目标改写成轨道本身，轨道里的链接点不开。
test("平板宽度下鼠标单击轨道内的模块链接能正常跳转", async ({ page }) => {
  await seed(page, "modern");
  await page.setViewportSize({ width: 768, height: 1024 });
  await page.goto("/");
  const rail = page.locator(".md-modules.md-track");
  await rail.scrollIntoViewIfNeeded();
  await expect(rail).toBeVisible();
  expect(await rail.evaluate((el) => el.scrollWidth - el.clientWidth)).toBeGreaterThan(0);

  const link = rail.locator("a.md-mod-title").first();
  const href = await link.getAttribute("href");
  expect(href).toBeTruthy();
  await link.click();
  await expect.poll(() => new URL(page.url()).pathname).toBe(href);
});

test("窄屏首页不产生整页横滚（轨道是滚动容器，不算溢出）", async ({ page }) => {
  await seed(page, "modern");
  for (const width of [320, 390]) {
    await page.setViewportSize({ width, height: 844 });
    await page.goto("/");
    await expect(page.locator(".md-root").first()).toBeVisible();
    await settleFrames(page);
    const overflow = await page.evaluate(() => document.body.scrollWidth - window.innerWidth);
    expect(overflow, `@${width}`).toBeLessThanOrEqual(0);
  }
});

test("减少动态效果时不注入涟漪，内容也不会被藏住", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await seed(page, "modern");
  await page.goto("/glossary");
  const chip = page.locator(".md-chip").first();
  await chip.scrollIntoViewIfNeeded();
  const box = await chip.boundingBox();
  if (!box) throw new Error("chip 不可见");
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  // 否定断言要先给「该发生的」一个确定的落点：等按压事件处理完（两帧）再断言没有涟漪，
  // 否则 `toHaveCount(0)` 在 `mouse.down` 之后立刻成立，什么也没验证。
  await settleFrames(page);
  await expect(page.locator(".md-ping")).toHaveCount(0);
  await page.mouse.up();

  await settleFrames(page);
  const state = await page.evaluate(() => {
    const content = [...document.querySelectorAll("main .md-page > *")];
    return {
      motionReady: document.documentElement.classList.contains("js-motion"),
      tagged: document.querySelectorAll("[data-reveal]").length,
      content: content.length,
      hidden: content.filter((el) => getComputedStyle(el).opacity === "0").length,
    };
  });
  // 设计如此（`motion.rs`「降级策略」）：动效被关时既不启用 `.js-motion`，也不给任何元素打
  // `data-reveal` —— 内容「没有动效」，而不是「卡在透明态」。所以这里不能要求有 data-reveal 元素。
  expect(state.motionReady, "减少动态效果时不应启用 JS 浮现").toBe(false);
  expect(state.tagged, "减少动态效果时不应给元素打 data-reveal").toBe(0);
  // 非空守卫：确实有页面内容可查，「没有透明态」才有意义。
  expect(state.content, "页面应有内容块").toBeGreaterThan(0);
  expect(state.hidden, "动效关闭时不应有内容块停在透明态").toBe(0);
});

// 回归：按压反馈的类名曾与首页旧的 `.md-ring`（132×132 的橙色圆环）撞名 —— 按下选项行的 560ms 内
// 整行缩成一个圆，`mouseup` 落到别处、`click` 被丢掉，表现为「点前几个选项偶尔没反应」。
//
// 判据不依赖具体类名，也不依赖时序：按下之前先在页面里装一个逐帧采样器，记录整个按压窗口内
// 选项行的宽高，之后断言它**从头到尾没变过**。（早先版本断言「出现 md-sonar 类」，那个类只活 560ms，
// 机器一忙轮询就漏掉；而且换个类名撞名它就看不见了。）
test("按压整块表面（选项行）全程不改变它的尺寸，点选项文字总能选中", async ({ page }) => {
  await seed(page, "modern");
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:shortcutsHelpSeen:exam", "1");
  });
  await page.goto("/exam");
  const row = page.locator("main .md-opt").first();
  await expect(row).toBeVisible();
  const before = await row.boundingBox();
  // 抽到单选还是多选是随机的：点「A.」这段文字，两种题型下都在可点区域内。
  const target = await row.locator("strong").first().boundingBox();
  if (!before || !target) throw new Error("选项行不可见");

  // 采样 900ms（> 反馈动画的 560ms），期间鼠标一直按着。
  await page.evaluate(() => {
    const el = document.querySelector("main .md-opt") as HTMLElement;
    const samples: [number, number][] = [];
    const w = window as unknown as { __sizes: [number, number][]; __sampled: boolean };
    w.__sizes = samples;
    w.__sampled = false;
    const t0 = performance.now();
    const tick = () => {
      const r = el.getBoundingClientRect();
      samples.push([r.width, r.height]);
      if (performance.now() - t0 < 900) requestAnimationFrame(tick);
      else w.__sampled = true;
    };
    requestAnimationFrame(tick);
  });
  await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2);
  await page.mouse.down();
  await page.waitForFunction(() => (window as unknown as { __sampled: boolean }).__sampled);
  await page.mouse.up();

  const sizes = await page.evaluate(() => (window as unknown as { __sizes: [number, number][] }).__sizes);
  expect(sizes.length, "采样器应跑了足够多的帧").toBeGreaterThan(10);
  const drift = sizes.filter(([w, h]) => Math.abs(w - before.width) > 1 || Math.abs(h - before.height) > 1);
  expect(drift, `按压期间选项行尺寸变过（原 ${before.width}×${before.height}）`).toEqual([]);

  await expect(row.locator('[role="radio"], [role="checkbox"]')).toHaveAttribute("aria-checked", "true");
});

test("像素风不装这套交互（无涟漪、无 md 类名）", async ({ page }) => {
  await seed(page, "pixel");
  await page.goto("/glossary");
  // `:visible`：页头里窄屏才出现的「更多选项」在桌面宽度下是 `display: none`，
  // 取到它会让 `scrollIntoViewIfNeeded` 一直等到超时。
  const button = page.locator("main button:visible").first();
  await button.scrollIntoViewIfNeeded();
  const box = await button.boundingBox();
  if (!box) throw new Error("按钮不可见");
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await settleFrames(page);
  await expect(page.locator(".md-ping")).toHaveCount(0);
  await page.mouse.up();
  // 标题写了「无 md 类名」，就要真的断言类名。
  const md = await page.evaluate(() =>
    [...document.querySelectorAll("[class]")].flatMap((el) =>
      [...el.classList].filter((c) => c.startsWith("md-")),
    ),
  );
  expect(md).toEqual([]);
});

// 签名规则必须命中页面「今天真正的 DOM」：lab 族大多还是桥接页的 `section.bg-card`，
// 只写 `.md-panel` 的选择器会一个节点都匹配不到，而且不会报任何错。
test("lab 签名：桥接页的卡片悬停会播扫描带", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/waveform-lab");
  const card = page.locator("#wave");
  await card.scrollIntoViewIfNeeded();
  await expect(card).toBeVisible();
  await card.hover();
  await expect
    .poll(() => card.evaluate((el) => getComputedStyle(el).animationName))
    .toBe("md-lab-scan");
});

test("lab 签名：减少动态效果时扫描带不播", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await seed(page, "modern");
  await page.goto("/waveform-lab");
  const card = page.locator("#wave");
  await card.scrollIntoViewIfNeeded();
  await card.hover();
  await page.waitForTimeout(200);
  expect(await card.evaluate((el) => getComputedStyle(el).animationName)).toBe("none");
});

test("map 签名：可点标记悬停提亮，无 <title> 的底图不提亮", async ({ page }) => {
  await seed(page, "modern");
  await page.goto("/dxcc-map");
  const markers = page.locator('#route-content svg[role="img"] polygon:has(> title)');
  await expect(markers.first()).toBeAttached({ timeout: 15_000 });

  // 悬停点必须真的落在这个标记上：多边形的包围盒中心常被相邻图形盖住，
  // 所以在视口内按网格取点，用 `elementFromPoint` 验证命中的就是它自己。
  await markers.first().scrollIntoViewIfNeeded();
  const target = await markers.evaluateAll((els) => {
    for (const [i, el] of els.entries()) {
      const r = el.getBoundingClientRect();
      for (let gx = 1; gx < 8; gx++) {
        for (let gy = 1; gy < 8; gy++) {
          const x = r.left + (r.width * gx) / 8;
          const y = r.top + (r.height * gy) / 8;
          if (x < 0 || y < 0 || x > innerWidth || y > innerHeight) continue;
          if (document.elementFromPoint(x, y) === el) return { i, x, y };
        }
      }
    }
    return null;
  });
  expect(target, "视口内应该有一个能被鼠标真正点中的标记").not.toBeNull();
  const marker = markers.nth(target!.i);
  await page.mouse.move(target!.x, target!.y);
  const probe = () =>
    marker.evaluate((el) => {
      const s = getComputedStyle(el);
      return { hovered: el.matches(":hover"), width: s.strokeWidth, cursor: s.cursor };
    });
  await expect.poll(probe).toMatchObject({ hovered: true, width: "1.6px", cursor: "pointer" });

  // 没有 <title> 的底图（陆地 / 经纬网）悬停时不应该被提亮。
  const base = page.locator('#route-content svg[role="img"] polygon:not(:has(> title))').first();
  // 不用 `if (count > 0)` 悄悄跳过：底图不存在，这半条断言就什么也没测，应当让用例红。
  await expect(base, "页面上应有无 <title> 的底图多边形").toBeAttached();
  expect(await base.evaluate((el) => getComputedStyle(el).strokeWidth)).not.toBe("1.6px");
});

// 旧规则给 `.md-root :is(svg, canvas)` 统一设了 `cursor: grab`：`/satellites` 页内没有地图，
// 它下拉框的箭头 svg 也会变成抓手；`/grid-map` 里同理。抓手只属于 `MapView` 自己的内联样式。
for (const url of ["/satellites", "/grid-map"]) {
  test(`map 签名：${url} 的非地图 svg 不带抓手光标`, async ({ page }) => {
    await seed(page, "modern");
    await page.goto(url);
    await expect(page.locator("#route-content")).toHaveAttribute("data-sig", /map/);
    await page.waitForTimeout(1500);
    const result = await page.evaluate(() => {
      const icons = [...document.querySelectorAll('#route-content svg:not([role="img"])')];
      return {
        count: icons.length,
        grab: icons.filter((el) => getComputedStyle(el).cursor === "grab").length,
      };
    });
    expect(result.count, "页内应该有下拉箭头这类 svg，否则这条用例什么也没测").toBeGreaterThan(0);
    expect(result.grab).toBe(0);
  });
}

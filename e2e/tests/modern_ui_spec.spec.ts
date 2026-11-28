import { expect, test, type Page } from "./fixtures";

/** 本文件显式切现代风：`E2E_UI_STYLE` 只对全站审计类 spec 生效。 */
async function useModern(page: Page) {
  await page.addInitScript(() => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", "modern");
    localStorage.setItem("theme", "dark");
  });
}

/** 顶层板块标签（内嵌的组件子标签不在这里，避免两套 tab 串名）。 */
function topTab(page: Page, name: string) {
  return page.getByRole("tablist", { name: "设计规范板块" }).getByRole("tab", { name, exact: true });
}

/** 某个展示列表的直接子项：`<For>` 不产生 wrapper 节点，所以数直接子节点即可。 */
function items(page: Page, panel: string, list: string, child = "*") {
  return page.locator(`[data-spec-panel="${panel}"] [data-spec-list="${list}"] > ${child}`);
}

test("现代风 /ui-spec：七个板块的展示项目与像素风逐项对齐", async ({ page }) => {
  await useModern(page);
  await page.goto("/ui-spec");

  await expect(page.getByRole("heading", { level: 1, name: "Modern UI 设计规范" })).toBeVisible();
  await expect(page.getByText("筹备中")).toHaveCount(0);
  await expect(page.locator("#fatal")).toHaveCount(0);

  // intro 是说明文字，不该带内容块的左上角标，圆角也应与页面其它卡片一致。
  const intro = await page.evaluate(() => {
    const el = document.querySelector('[data-spec-intro]')!;
    return {
      marker: getComputedStyle(el, "::before").backgroundImage,
      radius: getComputedStyle(el).borderRadius,
    };
  });
  expect(intro.marker, "说明卡片不应带内容块角标").toBe("none");
  expect(intro.radius, "说明卡片圆角应与其它卡片一致").toBe("10px");
  await expect(page.getByRole("tablist", { name: "设计规范板块" }).getByRole("tab")).toHaveCount(7);

  // 总览：6 个筛选维度之外，8 张原则卡 + 10 行规格速查，点卡片会高亮对应速查行。
  await expect(items(page, "overview", "principles")).toHaveCount(8);
  await expect(items(page, "overview", "specs")).toHaveCount(10);
  await items(page, "overview", "principles").first().click();
  const highlighted = await items(page, "overview", "specs").evaluateAll(
    (nodes) => nodes.filter((n) => n.className.includes("bg-accent")).length,
  );
  expect(highlighted).toBe(1);

  // 通用组件：组件索引 6 族 + 6 个子标签；两个像素风专属子标签用现代等价物对齐。
  await topTab(page, "通用组件").click();
  const components = page.locator('[data-spec-panel="components"]');
  await expect(components.getByRole("tab")).toHaveCount(6);
  await expect(items(page, "components", "component-families")).toHaveCount(6);
  // 清除按钮 hover 不该出现任何背景块（输入框可能被自动填充染成浅黄）。
  await components.getByRole("tab", { name: "基础控件" }).click();
  await page.locator("#design-search").fill("franciscowu");
  const clear = page.locator('[class*="md-input-clear"]').first();
  await expect(clear).toBeVisible();
  const readClear = () =>
    clear.evaluate((el) => ({
      bg: getComputedStyle(el).backgroundColor,
      color: getComputedStyle(el).color,
    }));
  const clearNormal = await readClear();
  await clear.scrollIntoViewIfNeeded();
  const clearBox = await clear.boundingBox();
  if (!clearBox) throw new Error("清除按钮没有布局盒，无法测悬停");
  await page.mouse.move(clearBox.x + clearBox.width / 2, clearBox.y + clearBox.height / 2);
  const clearHover = await readClear();
  expect(clearHover.bg, "清除按钮 hover 时不应出现背景块").toBe("rgba(0, 0, 0, 0)");
  expect(clearHover.color, "hover 仍应有图标颜色反馈").not.toBe(clearNormal.color);

  await components.getByRole("tab", { name: "电台面板" }).click();
  await expect(items(page, "components", "radio-items")).toHaveCount(16);
  await expect(items(page, "components", "radio-levels")).toHaveCount(5);

  // 滑杆与读数必须同量纲：把第一档拖到 50，读数就是 50%，进度条也不得超出轨道。
  const radioLevel = items(page, "components", "radio-levels").first();
  await radioLevel.locator('input[type="range"]').evaluate((el: HTMLInputElement) => {
    el.value = "50";
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await expect(radioLevel.getByText("50%", { exact: true })).toBeVisible();
  const fill = await radioLevel.evaluate((el) => {
    const bar = el.querySelector(".md-bar-fill")!.getBoundingClientRect();
    const track = el.querySelector(".md-bar-track")!.getBoundingClientRect();
    return { bar: bar.width, track: track.width };
  });
  expect(fill.bar, "进度条不应超出轨道").toBeLessThanOrEqual(fill.track + 1);

  await components.getByRole("tab", { name: "弹层与读数" }).click();
  await expect(items(page, "components", "leds")).toHaveCount(4);

  // 四档灯要真的分色，ON/OFF 要真的换态：只有 4 个灰点等于这条交互没实现。
  const ledColors = (selector: string) =>
    page.locator(selector).evaluateAll((els) => els.map((el) => getComputedStyle(el).backgroundColor));
  const ledsOn = await ledColors('[data-spec-panel="components"] [data-spec-list="leds"] .md-led');
  expect(new Set(ledsOn).size, "四档灯应有 4 种不同色相").toBe(4);
  await components.getByText("OFF", { exact: true }).click();
  const ledsOff = await ledColors('[data-spec-panel="components"] [data-spec-list="leds"] .md-led');
  expect(ledsOff, "关灯后色相应改变").not.toEqual(ledsOn);
  expect(new Set(ledsOff).size, "关灯后应回到同一种熄灭色").toBe(1);

  // 图标：58 个形状 + 10 个徽记 + 3 档倍率 + 搜索框；搜索再清空后必须回到全量。
  await topTab(page, "图标一览").click();
  await expect(items(page, "icons", "emblems")).toHaveCount(10);
  await expect(items(page, "icons", "icons")).toHaveCount(58);
  await expect(page.getByPlaceholder("搜索图标名称")).toBeVisible();
  const iconSearch = page.getByPlaceholder("搜索图标名称");
  await iconSearch.fill("ar");
  await expect.poll(() => items(page, "icons", "icons").count()).toBeLessThan(58);
  await iconSearch.fill("");
  await expect(items(page, "icons", "icons")).toHaveCount(58);

  // 色板：6 组 / 45 个 token，明暗切换与点击复制都在。
  await topTab(page, "色彩体系").click();
  await expect(
    page.locator('[data-spec-panel="colors"] [data-spec-list="token-group"]'),
  ).toHaveCount(6);
  await expect(items(page, "colors", "tokens")).toHaveCount(45);
  await expect(page.getByRole("button", { name: "复制变量名" })).toBeVisible();
  // 色板只能放颜色变量：字号 / 字体 / 圆角分别归排版与总览，不能混进来撑爆卡片。
  const nonColorTokens = await page.evaluate(() => {
    const rows = [
      ...document.querySelectorAll('[data-spec-panel="colors"] [data-spec-list="tokens"] > *'),
    ];
    return rows
      .map((el) => el.querySelectorAll("span")[1]?.textContent?.trim() || "")
      .filter((value) => !CSS.supports("color", value));
  });
  expect(nonColorTokens, "色板只应展示颜色变量").toEqual([]);
  await expect(page.getByText("屏幕层", { exact: true })).toHaveCount(0);
  await expect(page.getByText("指示灯", { exact: true })).toHaveCount(0);

  // 排版：7 阶字号 + 4 档字族 + 5 个排版类，预览输入联动四处。
  await topTab(page, "排版规范").click();
  await expect(items(page, "typography", "type-steps", "div")).toHaveCount(7);
  await expect(items(page, "typography", "type-families", "div")).toHaveCount(4);
  await expect(items(page, "typography", "type-class-items", "div")).toHaveCount(5);
  await expect(page.getByRole("textbox", { name: "预览文本" })).toBeVisible();
  // 现代风不做像素风的「易读字体」偏好切换，改为切换自己三档字体 token。
  await expect(page.getByText("易读字体")).toHaveCount(0);
  await expect(
    page.locator('[data-spec-panel="typography"] [data-spec-list="font-families"]'),
  ).toBeVisible();
  await expect(page.getByText("display", { exact: true })).toBeVisible();

  // 动效：4 个时长 token + 6 个可重播演示。
  await topTab(page, "动效一览").click();
  await expect(items(page, "motion", "motion-tokens")).toHaveCount(4);
  await expect(items(page, "motion", "motion-demos")).toHaveCount(6);
  await expect(page.getByRole("button", { name: "重播此演示" }).first()).toBeVisible();
  // 6 个演示都要有可见方块，且不能溢出卡片（装饰类的伪元素曾把方块顶出卡片）。
  const demoBoxes = await page.locator('[data-spec-list="motion-demos"] > *').evaluateAll((cards) =>
    cards.map((card) => {
      const box = card.querySelector('[data-motion-box]');
      const stage = card.querySelector('[data-motion-stage]');
      const c = card.getBoundingClientRect();
      const s = stage?.getBoundingClientRect();
      const b = box?.getBoundingClientRect();
      const cs = stage ? getComputedStyle(stage) : null;
      const within = (outer?: DOMRect) =>
        !!b &&
        !!outer &&
        b.left >= outer.left - 1 &&
        b.right <= outer.right + 1 &&
        b.top >= outer.top - 1 &&
        b.bottom <= outer.bottom + 1;
      return {
        visible: !!b && b.width > 0 && b.height > 0,
        notClipped: !!cs && cs.overflowX !== "hidden" && cs.overflowY !== "hidden",
        insideStage: within(s),
        insideCard: within(c),
      };
    }),
  );
  expect(demoBoxes).toHaveLength(6);
  expect(
    demoBoxes.every((d) => d.visible && d.notClipped && d.insideStage && d.insideCard),
    "动效演示方块应完整可见、既不被裁剪也不溢出卡片",
  ).toBe(true);

  // 每个演示的动画名必须与名称 / 中文说明对应，不能只是「随便一个 transform」。
  const animationNames = await page.locator('[data-spec-list="motion-demos"] > *').evaluateAll(
    (cards) =>
      cards.map((card) => {
        const el = card.querySelector('[data-animate]');
        return el ? getComputedStyle(el).animationName : "none";
      }),
  );
  expect(animationNames).toEqual([
    "md-signal-ring",
    "md-sonar",
    "md-current",
    "md-ring-out",
    "md-fade-in",
    "md-skel-shimmer",
  ]);

  // 指针高光要真的跟随指针：移动鼠标后舞台应写入 --mx。
  const spotlightStage = page
    .locator('[data-spec-list="motion-demos"] > *')
    .nth(4)
    .locator('[data-motion-stage]');
  await spotlightStage.scrollIntoViewIfNeeded();
  const stageBox = await spotlightStage.boundingBox();
  if (stageBox) {
    await page.mouse.move(stageBox.x + stageBox.width * 0.25, stageBox.y + stageBox.height * 0.5);
  }
  const mx = await spotlightStage.evaluate((el) => el.style.getPropertyValue("--mx"));
  expect(mx, "指针高光应跟随指针位置").not.toBe("");

  // 重播必须重启真实动画：取一直循环的波形演示，点击后时间轴应回到起点。
  const waveCard = page.locator('[data-spec-list="motion-demos"] > *').nth(2);
  const wavePath = waveCard.locator('[data-animate]').first();
  await expect
    .poll(() => wavePath.evaluate((el) => el.getAnimations()[0]?.currentTime ?? 0))
    .toBeGreaterThan(80);
  const beforeReplay = await wavePath.evaluate((el) => el.getAnimations()[0]?.currentTime ?? 0);
  await waveCard.getByRole("button", { name: "重播此演示" }).click();
  const afterReplay = await wavePath.evaluate((el) => el.getAnimations()[0]?.currentTime ?? 0);
  expect(afterReplay, "重播应重启动画时间轴").toBeLessThan(beforeReplay);

  // 状态：8 档切换 + 8 行 ARIA 对照，整组控件联动。
  await topTab(page, "状态一览").click();
  const states = page.locator('[data-spec-panel="states"]');
  await expect(states.locator(".divide-y > div")).toHaveCount(8);
  await expect(states.getByText("aria-invalid='true'")).toBeVisible();
  await expect(states.getByText("只读")).toBeVisible();

  // 现代风分支不能漏出像素风类名，也不能漏出像素字体（`<code>` 曾回落到 Fusion Pixel Mono）。
  await expect(page.locator('[class*="pxl-"]')).toHaveCount(0);
  const pixelFonts = await page.evaluate(
    () =>
      [...document.querySelectorAll("*")].filter((el) =>
        /pixel|pxl/i.test(getComputedStyle(el).fontFamily || ""),
      ).length,
  );
  expect(pixelFonts, "现代风规范页不应残留像素字体").toBe(0);

  // 展示格是卡片不是药丸按钮：`.md-btn` 默认 `--md-r-pill`（999px），必须被卡片半径覆盖。
  const cardRadii = await page.evaluate(() => {
    const cards = [...document.querySelectorAll('[data-spec-panel] button.md-tile')];
    return {
      total: cards.length,
      pills: cards.filter((el) => getComputedStyle(el).borderRadius === "999px").length,
    };
  });
  expect(cardRadii.total).toBeGreaterThan(0);
  expect(cardRadii.pills, "展示卡片不应退化成药丸圆角").toBe(0);
});

test("现代风首页：题库版本选择会带进练习与考试链接", async ({ page }) => {
  await useModern(page);
  await page.goto("/");

  const combo = page.getByRole("combobox", { name: "选择题库版本", exact: true });
  await expect(combo).toBeVisible();

  // 选择器异步载入题库版本后，首屏 CTA 与快速入口的链接都带上 version / bank。
  await expect
    .poll(() => page.locator('a[href^="/practice?version="]').count())
    .toBeGreaterThan(0);
  await expect
    .poll(() => page.locator('a[href^="/exam?version="]').count())
    .toBeGreaterThan(0);
  await expect(page.locator("#fatal")).toHaveCount(0);
});

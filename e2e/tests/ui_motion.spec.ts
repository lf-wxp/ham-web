import type { Page } from "@playwright/test";
import { expect, settleFrames, test } from "./fixtures";

// `ui/` 组件的动效与状态契约：退场、禁用、错误态、悬停过渡、勾选反馈。
//
// 每条都在两套皮肤下各跑一遍。这些问题的共同点是「静态审计和 axe 都看不见」：
// 退场是 200ms 的一瞬、禁用态要对比同款可用控件才看得出缺了什么、错误态只在校验失败时出现。
// 所以判据都是**在浏览器里量真实行为**，而不是读样式表。

type Style = "pixel" | "modern";
const STYLES: Style[] = ["pixel", "modern"];

async function boot(page: Page, style: Style, url: string, ready: string) {
  await page.addInitScript((s) => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", s);
    localStorage.setItem("theme", s === "modern" ? "dark" : "light");
    localStorage.setItem("ui:shortcutsHelpSeen:exam", "1");
  }, style);
  await page.goto(url);
  await expect(page.locator("#route-content[data-sig]")).toBeAttached({ timeout: 45_000 });
  await expect(page.locator(ready).first()).toBeVisible({ timeout: 30_000 });
  await settleFrames(page);
}

interface Frame {
  t: number;
  mounted: boolean;
  state: string | null;
  opacity: number;
  /** 这一帧上面板正在跑的动画：名字 + 总时长（含延迟）。 */
  anims: { name: string; end: number }[];
  /** 这一帧上遮罩的 `pointer-events`（没有遮罩为 null）。 */
  overlayPe: string | null;
}

/**
 * 在页面里逐帧采样一个弹层的关闭过程。
 *
 * 采样器在关闭动作**之前**就装好（rAF 逐帧），关闭动作由调用方触发；返回所有帧。
 * 不能靠 Playwright 轮询：退场只有 200ms，轮询的间隔与 IPC 延迟比它还长。
 */
async function traceClose(
  page: Page,
  sel: { panel: string; overlay?: string },
  close: () => Promise<void>,
  windowMs = 700,
): Promise<Frame[]> {
  await page.evaluate(
    ([panelSel, overlaySel, ms]) => {
      const w = window as unknown as { __frames: Promise<Frame[]> };
      w.__frames = new Promise((resolve) => {
        const rows: Frame[] = [];
        const t0 = performance.now();
        const tick = () => {
          const p = document.querySelector(panelSel) as HTMLElement | null;
          const o = overlaySel ? (document.querySelector(overlaySel) as HTMLElement | null) : null;
          rows.push({
            t: performance.now() - t0,
            mounted: !!p,
            state: p?.getAttribute("data-state") ?? null,
            opacity: p ? Number(getComputedStyle(p).opacity) : 0,
            anims: p
              ? p.getAnimations().map((a) => ({
                  name: (a as CSSAnimation).animationName ?? (a as CSSTransition).transitionProperty,
                  end: Number(a.effect?.getComputedTiming().endTime ?? 0),
                }))
              : [],
            overlayPe: o ? getComputedStyle(o).pointerEvents : null,
          });
          if (performance.now() - t0 < ms) requestAnimationFrame(tick);
          else resolve(rows);
        };
        requestAnimationFrame(tick);
      });
    },
    [sel.panel, sel.overlay ?? "", windowMs] as const,
  );
  await close();
  return page.evaluate(() => (window as unknown as { __frames: Promise<Frame[]> }).__frames);
}

/** 关闭动作之后的帧：从第一个 `data-state=closed` 起算。 */
function afterClose(frames: Frame[]) {
  const i = frames.findIndex((f) => f.mounted && f.state === "closed");
  expect(i, "关闭后应出现 data-state=closed 的帧").toBeGreaterThanOrEqual(0);
  const rows = frames.slice(i);
  const gone = rows.find((f) => !f.mounted);
  return { rows, t0: rows[0].t, goneAt: gone ? gone.t - rows[0].t : null };
}

for (const style of STYLES) {
  test.describe(`${style} · 弹层退场`, () => {
    // 回归：像素风的退场曾写成 `transition: opacity`，而入场动画在同一次样式变更里被移除，
    // 浏览器不会从动画值起过渡 —— 面板在关闭后第一帧就是透明的（逐帧实测，退场动画列表是空的）。
    // 现代风的 `ui::Dialog` 则根本没有 `[data-state="closed"]` 规则。
    const CASES = [
      {
        name: "ui::Dialog（全站搜索）",
        url: "/",
        ready: "main",
        open: (p: Page) => p.keyboard.press("/"),
        close: (p: Page) => p.keyboard.press("Escape"),
        panel: '[data-slot="dialog-content"]',
        overlay: '[data-slot="dialog-overlay"]',
      },
      {
        name: "下拉面板（SimpleSelect）",
        url: "/log",
        ready: '[role="combobox"]',
        open: (p: Page) => p.getByRole("combobox", { name: "CAT 协议", exact: true }).click(),
        close: (p: Page) => p.keyboard.press("Escape"),
        // 外层面板才带 `data-state` 与动画；listbox 是它里面可滚动的那一层。
        panel: '[role="listbox"]',
        overlay: ".fixed.inset-0.z-40",
      },
    ];

    for (const c of CASES) {
      test(`${c.name}：退场是真实的动画，不拦截点击，卸载不早于动画结束`, async ({ page }) => {
        await boot(page, style, c.url, c.ready);
        await c.open(page);
        // 下拉的 `data-state` 在 listbox 的父层：统一取面板节点本身再向上找带 data-state 的那层。
        const panelSel = c.panel === '[role="listbox"]' ? '[role="listbox"]' : c.panel;
        await expect(page.locator(panelSel).first()).toBeVisible();
        await settleFrames(page);

        // 下拉的外观 / 动画挂在 listbox 的外层 div 上：采样那一层。
        const sampled = c.panel === '[role="listbox"]' ? '[data-state] > [role="listbox"]' : c.panel;
        const frames = await page.evaluate(() => 0).then(() =>
          traceClose(
            page,
            {
              // 采 `data-state` 所在的那层：下拉是 listbox 的父节点
              panel: sampled === '[data-state] > [role="listbox"]' ? "[data-state]:has(> [role=\"listbox\"])" : sampled,
              overlay: c.overlay,
            },
            () => c.close(page),
          ),
        );
        const { rows, goneAt } = afterClose(frames);

        // 1) 退场期间有一个真实在跑的动画（名字里带 out）。
        const exiting = rows.find((f) => f.mounted && f.anims.some((a) => /out/i.test(a.name)));
        expect(exiting, "退场期间应有名为 *-out 的动画在跑（不是过渡、不是空的）").toBeTruthy();

        // 2) 退场期间遮罩不拦截点击（用户松手后紧接着点别处，那一下不能被吞）。
        for (const f of rows) {
          if (f.mounted && f.overlayPe !== null) {
            expect(f.overlayPe, `t=${Math.round(f.t)}ms：关闭态的遮罩应是 pointer-events: none`).toBe("none");
          }
        }

        // 3) 卸载时刻 ≥ 动画结束（不截断），且不比它晚太多（不留长长的空壳）。
        const animEnd = Math.max(...rows.flatMap((f) => f.anims.map((a) => a.end)));
        expect(goneAt, "应在 700ms 内卸载").not.toBeNull();
        expect(goneAt!, `卸载（${Math.round(goneAt!)}ms）不应早于退场动画结束（${Math.round(animEnd)}ms）`).toBeGreaterThanOrEqual(animEnd - 20);
        // 上限按**实测帧间隔**放宽，不能写死 120ms：`goneAt` 来自逐帧采样，「已经卸载」最早也要
        // 等到下一帧才看得见，本身就带一帧的量化误差。CI 上并行跑的另一个 worker 把 CPU 占满时
        // 单帧能到 140–180ms（180ms 的退场被量成 360ms 左右），固定余量会把「机器慢」判成
        // 「留了长长的空壳」。健康机器上一帧 16–30ms，下限 120ms 仍与原来一样紧，真正的空壳回归
        // 照样红。两帧：卸载计时器起算与采样各吃一次。
        const frameGap = Math.max(...rows.slice(1).map((f, i) => f.t - rows[i].t));
        const slack = Math.min(360, Math.max(120, frameGap * 2));
        expect(
          goneAt!,
          `卸载（${Math.round(goneAt!)}ms）比动画结束（${Math.round(animEnd)}ms）晚了太久（单帧 ${Math.round(frameGap)}ms，上限 ${Math.round(animEnd + slack)}ms）`,
        ).toBeLessThanOrEqual(animEnd + slack);
      });
    }

    test("悬停气泡：移开后有淡出，之后才隐藏；淡出途中悬停到下一个不会被旧计时器藏掉", async ({ page }) => {
      await boot(page, style, "/morse", "main [data-tip]");
      const tips = page.locator("main [data-tip]");
      const popup = page.locator('[data-slot="tooltip-popup"]');
      const hover = async (i: number) => {
        const box = await tips.nth(i).boundingBox();
        if (!box) throw new Error("触发器不可见");
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      };
      await tips.first().scrollIntoViewIfNeeded();
      await hover(0);
      await expect(popup).toBeVisible();
      await expect(popup).toHaveAttribute("data-state", "open");

      // 移开：先是 closed + 退场动画，仍然可见（在播），随后才 hidden。
      await page.mouse.move(2, 2);
      await expect(popup).toHaveAttribute("data-state", "closed");
      const during = await popup.evaluate((el) => ({
        hidden: (el as HTMLElement).hidden,
        anim: el.getAnimations().map((a) => (a as CSSAnimation).animationName),
      }));
      expect(during.hidden, "淡出期间气泡还没隐藏").toBe(false);
      expect(during.anim.some((n) => /out/.test(n)), `淡出期间应有 *-out 动画，实际 ${during.anim}`).toBe(true);
      await expect(popup).toBeHidden();

      // 代际：移开 → 立刻悬停到下一个触发器，旧的延迟隐藏不能把新气泡藏掉。
      await hover(0);
      await expect(popup).toBeVisible();
      await page.mouse.move(2, 2);
      await expect(popup).toHaveAttribute("data-state", "closed");
      await hover(1);
      await expect(popup).toHaveAttribute("data-state", "open");
      // 等过旧计时器的到点时间（Fast 档 + 一帧，留足余量）：气泡必须还在。
      await page.waitForTimeout(500);
      await expect(popup, "旧计时器不该把重新出现的气泡藏掉").toBeVisible();
    });

    // 回归：退场中的面板还留在 DOM 里播动画，但对读屏 / 键盘 / 定位器它应当已经不存在了。
    // 实测（CDP 无障碍树）：选完一项后约 250ms 内，已关闭的列表框仍是 1 个 listbox + N 个 option，
    // 紧接着打开的第二个下拉会同时露出两份「全部」—— 读屏用户会听到两个列表，`pickOption` 连续调用会撞上
    // strict mode violation。`inert`（浏览器无障碍树）与 `aria-hidden`（Playwright 与老辅助技术）要**一起**写。
    test("下拉：选完立刻打开另一个，退场中的面板不在无障碍树里（inert + aria-hidden）", async ({ page }) => {
      await boot(page, style, "/diy-projects", '[role="combobox"]');
      await page.getByRole("combobox", { name: "难度" }).click();
      await page.getByRole("option", { name: "进阶", exact: true }).click();
      // 与 `pickOption` 完全一样：连续调用，中间不等退场结束。
      await page.getByRole("combobox", { name: "所需仪表" }).click();

      // 退场中的那个面板：属性要对。
      const closing = page.locator('[data-state="closed"]:has(> [role="listbox"])');
      await expect(closing).toHaveCount(1);
      await expect(closing).toHaveAttribute("inert", "");
      await expect(closing).toHaveAttribute("aria-hidden", "true");
      // 打开着的那个：这两个属性必须**不存在**（写成 aria-hidden="false" 也不行，整棵树会被意外隐藏）。
      const opened = page.locator('[data-state="open"]:has(> [role="listbox"])');
      await expect(opened).toHaveCount(1);
      await expect(opened).not.toHaveAttribute("inert", /.*/);
      await expect(opened).not.toHaveAttribute("aria-hidden", /.*/);

      // 浏览器真实的无障碍树里只有一个列表框、一份「全部」；定位器也只命中一个。
      const cdp = await page.context().newCDPSession(page);
      await cdp.send("Accessibility.enable");
      const { nodes } = await cdp.send("Accessibility.getFullAXTree");
      const live = (role: string) => nodes.filter((n) => n.role?.value === role && !n.ignored);
      expect(live("listbox"), "无障碍树里同时只应有一个列表框").toHaveLength(1);
      expect(live("option").filter((n) => n.name?.value === "全部"), "「全部」只应有一份").toHaveLength(1);
      await expect(page.getByRole("option", { name: "全部", exact: true })).toHaveCount(1);
    });

    // 关 → 开 → 关：第一次关闭排的卸载计时器稍后到点，那时 `open` 恰好又是 false。
    // 只比较 `open` 的实现会在这一刻把**第二次**退场截断在半路；代际号（打开与关闭都自增）才认得出它已过期。
    // 单纯「关 → 开」区分不出两种实现（计时器到点时 `open` 是 true，谁都不会卸载）。
    test("下拉：关 → 开 → 关 的快速序列里，旧计时器不会截断第二次退场（代际号）", async ({ page }) => {
      await boot(page, style, "/log", '[role="combobox"]');
      const trigger = page.getByRole("combobox", { name: "CAT 协议", exact: true });
      await trigger.click();
      await expect(page.getByRole("listbox")).toBeVisible();
      await settleFrames(page);

      await page.keyboard.press("Escape"); // 第一次关闭（排下计时器 A）
      await page.waitForTimeout(40);
      await trigger.focus();
      await page.keyboard.press("ArrowDown"); // 重新打开
      await expect(page.getByRole("listbox")).toBeVisible();
      await page.waitForTimeout(40);

      // 第二次关闭：从这里开始逐帧采样。计时器 A 会在这之后约 100ms（现代）/ 160ms（像素）到点。
      const frames = await traceClose(
        page,
        { panel: '[data-state]:has(> [role="listbox"])' },
        () => page.keyboard.press("Escape"),
      );
      const { rows, goneAt } = afterClose(frames);
      const animEnd = Math.max(...rows.flatMap((f) => f.anims.map((a) => a.end)));
      expect(animEnd, "第二次退场应有动画在跑").toBeGreaterThan(0);
      expect(goneAt, "应卸载").not.toBeNull();
      expect(
        goneAt!,
        `第二次退场被旧计时器截断了：${Math.round(goneAt!)}ms 卸载，动画要播 ${Math.round(animEnd)}ms`,
      ).toBeGreaterThanOrEqual(animEnd - 20);
    });
  });
}

// ---------------------------------------------------------------------------------------------
// 禁用态：和「同款但可用」的控件对比，视觉上必须能看出区别。
// ---------------------------------------------------------------------------------------------
for (const style of STYLES) {
  test.describe(`${style} · 禁用态`, () => {
    /**
     * 同一个控件，禁用前后各截一张图，字节不同就算「看得出」。
     * 对比同一个元素而不是两个元素：避免把「两个按钮本来就长得不一样」算成通过。
     */
    async function visiblyDisabled(page: Page, selector: string) {
      const loc = page.locator(selector).first();
      await loc.scrollIntoViewIfNeeded();
      await page.mouse.move(2, 2);
      const box = await loc.boundingBox();
      if (!box) throw new Error(`${selector} 不可见`);
      const clip = {
        x: Math.max(0, box.x - 6),
        y: Math.max(0, box.y - 6),
        width: Math.min(box.width + 12, 360),
        height: box.height + 12,
      };
      await loc.evaluate((el) => ((el as HTMLButtonElement).disabled = false));
      await settleFrames(page);
      const enabled = await page.screenshot({ clip });
      await loc.evaluate((el) => ((el as HTMLButtonElement).disabled = true));
      await settleFrames(page);
      const disabled = await page.screenshot({ clip });
      return !enabled.equals(disabled);
    }

    // 回归：现代风的 `.md-btn` / `.md-input` / 下拉触发器曾完全没有禁用态 —— 备份页里不可用的
    // 「写入远端」与旁边可用的按钮截图逐字节相同，主按钮还会在悬停时扫光 / 上抬。
    test("按钮 / 输入框 / 多行输入 / 下拉触发器：禁用后视觉上与可用态不同", async ({ page }) => {
      await boot(page, style, "/log", 'main input[placeholder="BG4XXX"]');
      expect(await visiblyDisabled(page, 'main button:has-text("添加记录")'), "按钮").toBe(true);
      expect(await visiblyDisabled(page, 'main input[placeholder="BG4XXX"]'), "输入框").toBe(true);
      expect(await visiblyDisabled(page, 'main [role="combobox"]'), "下拉触发器").toBe(true);
    });

    test("多行输入：禁用后视觉上与可用态不同", async ({ page }) => {
      await boot(page, style, "/tools", "main textarea");
      expect(await visiblyDisabled(page, "main textarea"), "多行输入").toBe(true);
    });

    // 每个变体的悬停规则各写各的，漏一个就是漏一个 —— 所以要遍历页面上**全部**禁用按钮，
    // 而不是取第一个（备份页的第一个是描边款，它一直排除了 `:disabled`；漏的是第三个，主按钮）。
    test("禁用的按钮悬停不会有反馈（不扫光、不上抬、不换色）：每个变体都测", async ({ page }) => {
      await boot(page, style, "/backup", "main button[disabled]");
      const buttons = page.locator("main button[disabled]");
      const n = await buttons.count();
      expect(n, "备份页应有至少 3 个禁用按钮（描边 / 描边 / 主按钮）").toBeGreaterThanOrEqual(3);
      const variants = new Set<string>();
      for (let i = 0; i < n; i++) {
        const btn = buttons.nth(i);
        await btn.scrollIntoViewIfNeeded();
        await page.mouse.move(2, 2);
        await page.waitForTimeout(250);
        const label = ((await btn.textContent()) ?? "").trim();
        variants.add(await btn.evaluate((el) => [...el.classList].find((c) => /^(md|pxl)-btn-/.test(c)) ?? "?"));
        const snap = () =>
          btn.evaluate((el) => {
            const c = getComputedStyle(el);
            return {
              transform: c.transform,
              bg: c.backgroundColor,
              shadow: c.boxShadow,
              // 主按钮的扫光是 `::after` 的 translate 过渡：getAnimations 带 subtree 才看得到。
              running: el
                .getAnimations({ subtree: true })
                .filter((a) => a.playState === "running")
                .map((a) => (a as CSSTransition).transitionProperty ?? (a as CSSAnimation).animationName),
            };
          });
        const idle = await snap();
        const box = (await btn.boundingBox())!;
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await page.waitForTimeout(250);
        const hovered = await snap();
        expect(hovered, `「${label}」悬停前后样式不应变`).toEqual(idle);
        expect(hovered.running, `「${label}」悬停不应启动任何过渡 / 动画`).toEqual([]);
      }
      // 至少要覆盖到主按钮这一类：否则这条用例又会变成只测了「本来就对」的那几个。
      expect([...variants].some((v) => /accent|primary/.test(v)), `应覆盖到主按钮变体，实际 ${[...variants]}`).toBe(true);
    });
  });
}

// ---------------------------------------------------------------------------------------------
// 错误态：`aria-invalid` 是唯一的事实来源，所有表单控件都要把它画出来。
// ---------------------------------------------------------------------------------------------
for (const style of STYLES) {
  test.describe(`${style} · 错误态`, () => {
    const border = (page: Page, selector: string) =>
      page.locator(selector).first().evaluate((el) => getComputedStyle(el).borderTopColor);

    const CASES = [
      { name: "复选框", url: "/log", sel: 'main [role="checkbox"]', attr: "self" },
      { name: "开关", url: "/grid-map", sel: 'main [role="switch"]', attr: "self" },
      // 单选的属性挂在 radiogroup 上（读屏对整组说「无效」），样式往下传给每个单选框。
      { name: "单选组", url: "/tools", sel: 'main [role="radiogroup"] [role="radio"]', attr: "group" },
    ] as const;

    for (const c of CASES) {
      test(`${c.name}：aria-invalid 让描边变色`, async ({ page }) => {
        await boot(page, style, c.url, c.sel);
        const before = await border(page, c.sel);
        await page.locator(c.sel).first().evaluate((el, attr) => {
          const host = attr === "group" ? el.closest('[role="radiogroup"]')! : el;
          host.setAttribute("aria-invalid", "true");
        }, c.attr);
        await settleFrames(page);
        const after = await border(page, c.sel);
        expect(after, `${c.name}加上 aria-invalid 后描边应变（原 ${before}）`).not.toBe(before);
        // 与输入框同一个红：不是随便变个色。取页面上真实的输入框做基准。
        const probe = await page.evaluate(() => {
          const el = document.createElement("input");
          el.className = document.documentElement.getAttribute("data-ui-style") === "modern" ? "md-input" : "pxl-field";
          el.setAttribute("aria-invalid", "true");
          document.querySelector("main")!.appendChild(el);
          const c = getComputedStyle(el).borderTopColor;
          el.remove();
          return c;
        });
        expect(after, `${c.name}的错误描边应与输入框的错误描边同色`).toBe(probe);
      });
    }
  });
}

// ---------------------------------------------------------------------------------------------
// 悬停 / 勾选反馈：同类控件要同一档过渡，勾选要有动画（且是自己皮肤的那一种）。
// ---------------------------------------------------------------------------------------------
for (const style of STYLES) {
  test.describe(`${style} · 反馈`, () => {
    test("输入框 / 多行输入 / 下拉触发器：悬停时描边是过渡而不是瞬变", async ({ page }) => {
      await boot(page, style, "/log", 'main input[placeholder="BG4XXX"]');
      for (const sel of ['main input[placeholder="BG4XXX"]', 'main [role="combobox"]']) {
        const loc = page.locator(sel).first();
        await loc.scrollIntoViewIfNeeded();
        await page.mouse.move(2, 2);
        await page.waitForTimeout(300);
        const box = (await loc.boundingBox())!;
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        const running = await loc.evaluate((el) =>
          el
            .getAnimations()
            .filter((a) => a.playState === "running")
            .map((a) => (a as CSSTransition).transitionProperty ?? ""),
        );
        expect(
          running.some((p) => /border/.test(p)),
          `${sel}：悬停应启动 border-color 过渡，实际 [${running}]`,
        ).toBe(true);
      }
    });

    test("复选框 / 单选框：勾选有弹跳动画，且用自己皮肤的那一种", async ({ page }) => {
      // 现代风：平滑的 `md-check-pop`；像素风：逐格的 `check-pop`。回归：现代风的单选框曾借用像素风的
      // `motion-check`（`steps(2)` 逐格跳），复选框则一点动画也没有。
      const expected = style === "modern" ? "md-check-pop" : "check-pop";

      await boot(page, style, "/log", 'main [role="checkbox"]');
      const box = page.locator('main [role="checkbox"]').first();
      await box.scrollIntoViewIfNeeded();
      // 要测「变为选中」那一下：已选中的先点掉。
      if ((await box.getAttribute("aria-checked")) === "true") {
        await box.click();
        await expect(box).toHaveAttribute("aria-checked", "false");
        await page.waitForTimeout(400);
      }
      await box.click();
      await expect(box).toHaveAttribute("aria-checked", "true");
      const boxAnims = await box.evaluate((el) =>
        el
          .getAnimations({ subtree: true })
          .map((a) => (a as CSSAnimation).animationName)
          .filter(Boolean),
      );
      expect(boxAnims, "勾选复选框应播放对勾弹跳动画").toContain(expected);

      await boot(page, style, "/tools", 'main [role="radio"]');
      const radios = page.locator('main [role="radiogroup"] [role="radio"]');
      // 按下标定位：用 `[aria-checked=false]` 过滤的定位器在点击后会重新解析、不再匹配。
      const index = await radios.evaluateAll((els) => els.findIndex((e) => e.getAttribute("aria-checked") === "false"));
      expect(index, "页面上应有一个未选中的单选框").toBeGreaterThanOrEqual(0);
      const target = radios.nth(index);
      await target.scrollIntoViewIfNeeded();
      await target.click();
      await expect(target).toHaveAttribute("aria-checked", "true");
      const radioAnims = await target.evaluate((el) =>
        el
          .getAnimations({ subtree: true })
          .map((a) => (a as CSSAnimation).animationName)
          .filter(Boolean),
      );
      expect(radioAnims, "选中单选框应播放圆点弹跳动画").toContain(expected);
      if (style === "modern") {
        expect(radioAnims, "现代风不该再出现像素风的逐格动画").not.toContain("check-pop");
      }
    });

    test("分页按钮：按下有反馈（涟漪），禁用的没有", async ({ page }) => {
      test.skip(style !== "modern", "涟漪是现代风的按压反馈；像素风按压是 2px 下沉（CSS 过渡）");
      await page.addInitScript(() => {
        if (window !== window.top) return;
        const entries = Array.from({ length: 130 }, (_, i) => ({
          id: i + 1, date: "2026-01-01", time: "00:00", freq: "14.074", band: "20m", mode: "FT8",
          callsign: "JA" + (1000 + i), gridsquare: "", my_sota_ref: "", my_pota_ref: "",
          contact_sota_ref: "", contact_pota_ref: "", lotw_rcvd: false, remark: "",
        }));
        if (!sessionStorage.getItem("seeded")) {
          sessionStorage.setItem("seeded", "1");
          localStorage.setItem("logbook", JSON.stringify({ entries }));
        }
      });
      await boot(page, style, "/log", ".md-page-btn");
      const enabled = page.locator(".md-page-btn:not(:disabled)").first();
      await enabled.scrollIntoViewIfNeeded();
      const eb = (await enabled.boundingBox())!;
      await page.mouse.move(eb.x + eb.width / 2, eb.y + eb.height / 2);
      await page.mouse.down();
      const hasPing = await enabled.evaluate((el) => !!el.querySelector(".md-ping") || el.classList.contains("md-pressing"));
      await page.mouse.up();
      expect(hasPing, "可用的分页按钮按下应有涟漪").toBe(true);
    });
  });
}

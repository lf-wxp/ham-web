import AxeBuilder from "@axe-core/playwright";
import type { Locator, Page } from "@playwright/test";
import { expect, settleFrames, test } from "./fixtures";

// 下拉（`ui::Select` 组合式 + `ui::SimpleSelect` 数据式）的行为契约。
//
// 两种写法共用同一个引擎，所以同一套契约要对**每一种用法**各跑一遍、在两套皮肤下各跑一遍 ——
// 以前它们是两份实现，一份键盘完全不可用、两份在 Tab 离开后都会留下全屏遮罩，
// 而唯一的 e2e 辅助函数（`pickOption`）只会点鼠标，一个都拦不住。
//
// 两种用法（站内**真实存在**的，不是为测试造的）：
//   - 数据式·筛选器：日志页的「CAT 协议」（2 项）与「模式」（18 项，会滚动，且选项随值变化）；
//   - 组合式：首页的题库版本选择器（选项带图标 / 徽章 / 两行文字）。**目前站内唯一的组合式用法**，
//     它不在日志页上，也只出现在像素风首页 —— 所以它的场景自带页面与风格限制。
// 以前还有第三种「数据式·无可见标签」（像素风顶栏的语言切换）。语言收进设置对话框后，
// 那里改成了三行单选（可见标签 + `role=radio`），站内就没有无可见标签的 `SimpleSelect` 了。

type Style = "pixel" | "modern";

interface Kind {
  name: string;
  /** 场景所在页面。 */
  url: string;
  /** 只在这些风格里存在（缺省两套都有）。 */
  styles?: Style[];
  pick: (p: Page) => Locator;
  /** 打开列表后，列表框应有的无障碍名（逐字相等）；用来拦「名字有但不对」。 */
  listName: string | RegExp;
  /** 列表至少有几项才测得出回绕等行为。 */
  minRows: number;
}

const KINDS: Kind[] = [
  {
    name: "SimpleSelect · 筛选器（CAT 协议）",
    url: "/log",
    pick: (p) => p.getByRole("combobox", { name: "CAT 协议", exact: true }),
    listName: "CAT 协议",
    minRows: 2,
  },
  {
    name: "SimpleSelect · 长列表（模式）",
    url: "/log",
    pick: (p) => p.getByRole("combobox", { name: "模式", exact: true }),
    listName: "模式",
    minRows: 18,
  },
  {
    name: "Select · 组合式（题库版本）",
    url: "/",
    styles: ["pixel"],
    pick: (p) => p.getByRole("combobox", { name: "选择题库版本", exact: true }),
    // 触发器的内容是**当前选中的版本**（「2025年10月版本 最新」）：列表框的名字必须是标签，
    // 不能是它。axe 只查「有没有名字」，查不出这个。
    listName: "选择题库版本",
    minRows: 1,
  },
];

/** 日志页上的两个数据式下拉：页面级用例（同页不串值、动态选项、外观一致）都在这页做。 */
const LOG = KINDS[0];
const HOME_COMBINED = KINDS[2];

async function open(page: Page, style: Style, kind: Kind) {
  await page.addInitScript((s) => {
    if (window !== window.top) return;
    localStorage.setItem("ui:uiStyle", s);
    localStorage.setItem("theme", s === "modern" ? "dark" : "light");
  }, style);
  await page.goto(kind.url);
  // 整批并发跑时首次加载可能超过默认的 15 秒断言超时（要拉很多资源）：这里给足一倍，
  // 并且先等到应用挂载完成，再等具体控件 —— 超时报错能区分「页面没起来」和「控件不在」。
  await expect(page.locator("#route-content[data-sig]")).toBeAttached({ timeout: 45_000 });
  await expect(kind.pick(page)).toBeVisible({ timeout: 30_000 });
  await settleFrames(page);
}

/** 当前打开的列表框里、按显示顺序排好的选项。 */
function rows(page: Page) {
  return page.locator('[role="listbox"] [role="option"]');
}

/** 触发器当前是否拿着焦点。 */
async function isFocused(trigger: Locator) {
  return trigger.evaluate((el) => el === document.activeElement);
}

for (const style of ["pixel", "modern"] as const) {
  for (const kind of KINDS) {
    if (kind.styles && !kind.styles.includes(style)) continue;
    test.describe(`${style} · ${kind.name}`, () => {
      test("键盘：↓ 打开并高亮选中项，方向键 / Home / End 移动且首尾回绕，Enter 选中", async ({ page }) => {
        await open(page, style, kind);
        const trigger = kind.pick(page);
        await trigger.focus();
        await page.keyboard.press("ArrowDown");

        await expect(trigger).toHaveAttribute("aria-expanded", "true");
        await expect(page.getByRole("listbox")).toBeVisible();

        // 打开时高亮落在当前选中项上，而不是永远从第一行开始。
        const selected = page.locator('[role="listbox"] [role="option"][aria-selected="true"]');
        await expect(selected).toHaveCount(1);
        const selectedId = await selected.getAttribute("id");
        expect(selectedId, "选中项应有 DOM id（aria-activedescendant 要指它）").toBeTruthy();
        await expect(trigger).toHaveAttribute("aria-activedescendant", selectedId!);

        // 全程恰有一行是高亮，且就是 aria-activedescendant 指的那一行。
        const activeRow = page.locator('[role="option"][data-active="true"]');
        await expect(activeRow).toHaveCount(1);
        await expect(activeRow).toHaveAttribute("id", selectedId!);

        const count = await rows(page).count();
        expect(count, `${kind.name} 至少要有 ${kind.minRows} 项`).toBeGreaterThanOrEqual(kind.minRows);
        // 只有 1 项的列表（题库版本）没有「移动 / 回绕 / 换一项」可测：这条用例的后半段不适用，
        // 键盘选中与回绕由其余场景覆盖；这里只验证打开时高亮落在选中项上，并且 Enter 能关闭。
        if (count < 2) {
          await page.keyboard.press("Enter");
          await expect(page.getByRole("listbox")).toHaveCount(0);
          expect(await isFocused(trigger), "Enter 之后焦点应仍在触发器").toBe(true);
          return;
        }
        const selectedIndex = await rows(page).evaluateAll((els) =>
          els.findIndex((e) => e.getAttribute("aria-selected") === "true"),
        );
        const shownBefore = ((await trigger.textContent()) ?? "").trim();
        const idAt = async (i: number) => rows(page).nth(i).getAttribute("id");

        await page.keyboard.press("End");
        await expect(trigger).toHaveAttribute("aria-activedescendant", (await idAt(count - 1))!);
        await page.keyboard.press("ArrowDown");
        await expect(trigger, "末尾再往下回到第一项").toHaveAttribute("aria-activedescendant", (await idAt(0))!);
        await page.keyboard.press("ArrowUp");
        await expect(trigger, "开头再往上回到最后一项").toHaveAttribute("aria-activedescendant", (await idAt(count - 1))!);
        await page.keyboard.press("Home");
        await expect(trigger).toHaveAttribute("aria-activedescendant", (await idAt(0))!);
        await expect(page.locator('[role="option"][data-active="true"]')).toHaveCount(1);

        // 选一个**不是当前值**的项（否则选了等于没选，测不出 Enter 真的生效）：
        // Enter 之后收起、触发器显示新值、焦点仍在触发器上（键盘用户不会被甩出去）。
        const targetIndex = selectedIndex === 0 ? 1 : 0;
        await page.keyboard.press("Home");
        for (let i = 0; i < targetIndex; i++) await page.keyboard.press("ArrowDown");
        const target = rows(page).nth(targetIndex);
        await expect(target).toHaveAttribute("data-active", "true");
        const text = ((await target.textContent()) ?? "").trim();
        expect(text, "目标项文字应不同于当前值").not.toBe(shownBefore);
        await page.keyboard.press("Enter");
        await expect(page.getByRole("listbox")).toHaveCount(0);
        await expect(trigger).toHaveAttribute("aria-expanded", "false");
        await expect(trigger).toHaveText(text);
        expect(await isFocused(trigger), "选中后焦点应仍在触发器").toBe(true);
      });

      test("键盘：Space 选中；Esc 收起且焦点不丢；收起后高亮清空", async ({ page }) => {
        await open(page, style, kind);
        const trigger = kind.pick(page);
        await trigger.focus();
        await page.keyboard.press("ArrowDown");
        await expect(page.getByRole("listbox")).toBeVisible();
        // 只有 1 项的列表没有「再往下」：高亮已经在唯一的一项上，`↓` 会回绕回它自己。
        await page.keyboard.press("ArrowDown");
        const chosen = ((await page.locator('[role="option"][data-active="true"]').textContent()) ?? "").trim();
        await page.keyboard.press(" ");
        await expect(page.getByRole("listbox")).toHaveCount(0);
        // 触发器显示选中项的**名字**；选项行里可能还有更多内容（题库版本带「包含: A, B, C」），
        // 所以不比全文，比「选项文字以触发器上显示的文字开头」。
        //
        // 必须轮询而不是读一次：选语言会重渲整页（英文 / 西语词典是拉回来的），触发器的文案与
        // 列表框的卸载不在同一批更新里 —— 并行满载时能读到「列表已收起、触发器还是旧值」的瞬间。
        const chosenFlat = chosen.replace(/\s+/g, "");
        await expect
          .poll(
            async () => {
              const shown = ((await trigger.textContent()) ?? "").replace(/\s+/g, "");
              return shown.length > 0 && chosenFlat.includes(shown);
            },
            { message: `选中后触发器应显示选中项（${chosen}）` },
          )
          .toBe(true);

        await page.keyboard.press("ArrowUp");
        await expect(page.getByRole("listbox")).toBeVisible();
        await page.keyboard.press("Escape");
        await expect(page.getByRole("listbox")).toHaveCount(0);
        expect(await isFocused(trigger), "Esc 收起后焦点应仍在触发器").toBe(true);
        // 收起后不应留着指向已卸载行的 aria-activedescendant。
        await expect(trigger).not.toHaveAttribute("aria-activedescendant", /.+/);
      });

      test("选中后再次打开：aria-activedescendant 指向新面板里的选中行（回归：以前指向已卸载的旧行）", async ({
        page,
      }) => {
        await open(page, style, kind);
        const trigger = kind.pick(page);
        await trigger.focus();
        // 反复开合几轮：旧版本的症状是「第一次开正常、选中后再开就指到旧面板的行」。
        for (let round = 0; round < 3; round++) {
          await page.keyboard.press("ArrowDown");
          await expect(page.getByRole("listbox")).toBeVisible();
          const selected = page.locator('[role="listbox"] [role="option"][aria-selected="true"]');
          await expect(selected).toHaveCount(1);
          const selectedId = (await selected.getAttribute("id"))!;
          // 指向的 id 必须是当前 DOM 里**真实存在**的行，而不只是「有个值」。
          await expect(trigger, `第 ${round + 1} 次打开时应指向当前选中行`).toHaveAttribute(
            "aria-activedescendant",
            selectedId,
          );
          await expect(page.locator(`[id="${selectedId}"]`)).toHaveCount(1);
          if ((await rows(page).count()) < 2) {
            await page.keyboard.press("Escape");
          } else {
            // 换到下一项再选中，让「选中项」每轮都变。
            await page.keyboard.press("ArrowDown");
            await page.keyboard.press("Enter");
          }
          await expect(page.getByRole("listbox")).toHaveCount(0);
        }
      });

      test("Tab / Shift+Tab 离开：弹层和全屏遮罩都收起（回归：以前会一直盖住整页）", async ({ page }) => {
        await open(page, style, kind);
        const trigger = kind.pick(page);
        for (const key of ["Tab", "Shift+Tab"]) {
          await trigger.focus();
          await page.keyboard.press("ArrowDown");
          await expect(page.getByRole("listbox")).toBeVisible();
          await page.keyboard.press(key);
          await expect(page.getByRole("listbox"), `${key} 之后列表框应收起`).toHaveCount(0);
          await expect(page.locator(".fixed.inset-0.z-40"), `${key} 之后遮罩应撤掉`).toHaveCount(0);
          expect(await isFocused(trigger), `${key} 之后焦点应已离开触发器`).toBe(false);
        }
        // 遮罩真的撤了：页面上别的东西又能点到。
        await kind.pick(page).click();
        await expect(page.getByRole("listbox")).toBeVisible();
      });

      test("鼠标：点选项选中并收起；点遮罩收起；再点触发器可切换", async ({ page }) => {
        await open(page, style, kind);
        const trigger = kind.pick(page);
        await trigger.click();
        await expect(page.getByRole("listbox")).toBeVisible();
        const unselected = page.locator('[role="listbox"] [role="option"][aria-selected="false"]');
        if (kind.minRows > 1) {
          // 选一个**未选中**的项：选当前值等于什么都没发生，测不出选中是否生效。
          const target = unselected.first();
          const text = ((await target.textContent()) ?? "").trim();
          await target.click();
          await expect(page.getByRole("listbox")).toHaveCount(0);
          await expect(trigger).toHaveText(text);
        } else {
          // 只有 1 项（题库版本）：没有「另一项」可选，改验「点已选中项也能收起，且选中不变」。
          const only = page.locator('[role="listbox"] [role="option"]').first();
          const before = ((await trigger.textContent()) ?? "").trim();
          await only.click();
          await expect(page.getByRole("listbox")).toHaveCount(0);
          await expect(trigger).toHaveText(before);
        }

        // 展开时全屏遮罩盖在触发器上面：真实用户点触发器区域落在遮罩上，效果是收起。
        // 用坐标点击而不是 `trigger.click()`（后者的可点性检查会因遮罩拦截而一直重试到超时）。
        await trigger.click();
        await expect(page.getByRole("listbox")).toBeVisible();
        const box = (await trigger.boundingBox())!;
        await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
        await expect(page.getByRole("listbox"), "再点触发器区域应收起").toHaveCount(0);

        await trigger.click();
        await expect(page.getByRole("listbox")).toBeVisible();
        const vp = page.viewportSize()!;
        // 最右 10–12px 是自绘窗口滚动条的拖动区（覆盖式，见 `docs/ui/pixel.md`「代价是拖动区
        // 覆盖最右 12px」）：点到的是 thumb 而不是遮罩，遮罩收不到这次点击；而且 thumb 落在哪个
        // 高度取决于页面滚到哪 —— 贴着右边界点会「有时红有时绿」。内缩 40px 仍在遮罩上，
        // 又稳稳避开拖动区。
        await page.mouse.click(vp.width - 40, Math.round(vp.height * 0.7));
        await expect(page.getByRole("listbox"), "点遮罩应收起").toHaveCount(0);
      });

      test("箭头：展开转到 180°、收起转回 0°，中间有过渡帧（不是瞬切）", async ({ page }) => {
        await open(page, style, kind);
        const trigger = kind.pick(page);
        // 逐帧采样 `rotate`（Tailwind v4 的 `rotate-180` 写的是独立的 rotate 属性，不是 transform）。
        const toggleAndSample = () =>
          trigger.evaluate(
            (btn) =>
              new Promise<number[]>((resolve) => {
                // 下拉箭头：触发器里**最后一个** svg（题库版本的触发器里前面还有自己的状态图标）。
        // 回调是在浏览器里序列化后执行的，引用不到 Node 侧定义的辅助函数，所以这里内联。
        const chevrons = btn.querySelectorAll("svg");
        const svg = chevrons[chevrons.length - 1] as SVGElement;
                const deg = () => {
                  const v = getComputedStyle(svg).rotate;
                  return v === "none" ? 0 : parseFloat(v);
                };
                const out: number[] = [];
                const t0 = performance.now();
                (btn as HTMLElement).click();
                const tick = () => {
                  out.push(deg());
                  if (performance.now() - t0 < 520) requestAnimationFrame(tick);
                  else resolve(out);
                };
                requestAnimationFrame(tick);
              }),
          );
        const opening = await toggleAndSample();
        expect(opening.at(-1), "展开后箭头应翻到 180°").toBe(180);
        expect(
          opening.some((d) => d > 0 && d < 180),
          `展开应有中间角度（渐变），实际只有 ${[...new Set(opening.map(Math.round))].join(",")}`,
        ).toBe(true);
        const closing = await toggleAndSample();
        expect(closing.at(-1), "收起后箭头应转回 0°").toBe(0);
        expect(closing.some((d) => d > 0 && d < 180), "收起也应渐变").toBe(true);
      });

      test("无障碍：aria-controls 指向列表框、行 id 唯一、列表框有名字、选项行没有多余元素、axe 零违规", async ({ page }) => {
        await open(page, style, kind);
        const trigger = kind.pick(page);
        await trigger.click();
        await expect(page.getByRole("listbox")).toBeVisible();

        const info = await page.evaluate(() => {
          const btn = document.querySelector('[role="combobox"][aria-expanded="true"]')!;
          const controls = btn.getAttribute("aria-controls");
          const list = controls ? document.getElementById(controls) : null;
          const ids = [...document.querySelectorAll('[role="option"]')].map((e) => e.id);
          return {
            controls,
            listRole: list?.getAttribute("role") ?? null,
            // 名字取浏览器真正的计算结果（aria-label 优先，其次 aria-labelledby 指向的元素文字）。
            name: (() => {
              if (!list) return null;
              const label = list.getAttribute("aria-label");
              if (label) return label;
              const by = list.getAttribute("aria-labelledby");
              return by ? (document.getElementById(by)?.textContent ?? "").trim().replace(/\s+/g, " ") : null;
            })(),
            ids: ids.length,
            unique: new Set(ids).size === ids.length && ids.every(Boolean),
            // 每行只有一个文字容器：不再有单独的对勾元素（数据式下拉以前多画一枚，两套皮肤里都出过问题）。
            extra: [...document.querySelectorAll('[role="option"]')].filter((o) => o.children.length !== 1).length,
          };
        });
        expect(info.controls, "展开时 aria-controls 应存在").toBeTruthy();
        expect(info.listRole).toBe("listbox");
        // 名字不只要「有」，还要「对」：axe 只查有没有，查不出列表框被命名成了「当前选中项」。
        expect(info.name, "列表框的名字应是控件的标签，而不是当前选中的值").not.toBeNull();
        if (typeof kind.listName === "string") expect(info.name).toBe(kind.listName);
        else expect(info.name).toMatch(kind.listName);
        expect(info.ids).toBeGreaterThanOrEqual(kind.minRows);
        expect(info.unique, "选项 id 必须非空且互不相同").toBe(true);
        expect(info.extra, "选项行不应有文字容器之外的子元素").toBe(0);

        // axe：只看与这个下拉有关的节点，别让页面上别处的存量问题让这条用例变红。
        await settleFrames(page);
        const result = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"]).analyze();
        const mine = result.violations.filter((v) =>
          v.nodes.some((n) => /role="(combobox|listbox|option)"/.test(n.html)),
        );
        expect(mine.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(" | ")}`)).toEqual([]);
      });

      test("typeahead：打开后连打字母跳到以它开头的项；带 Ctrl / Meta 的组合键不拦", async ({ page }) => {
        await open(page, style, kind);
        const trigger = kind.pick(page);
        await trigger.focus();
        await page.keyboard.press("ArrowDown");
        await expect(page.getByRole("listbox")).toBeVisible();

        const labels = (await rows(page).allTextContents()).map((t) => t.trim());
        // 只测首字符在列表里独一无二的项：这样期望行不依赖「从下一项环形找」的实现细节。
        const first = (s: string) => s.slice(0, 1).toLowerCase();
        const unique = labels
          .map((label, i) => ({ label, i }))
          .filter(({ label }) => label && labels.filter((l) => first(l) === first(label)).length === 1);
        // 只有 1 项或首字符全重复的列表没有可测的目标。
        test.skip(unique.length === 0 || labels.length < 2, "列表里没有首字符唯一的项");
        const target = unique[unique.length - 1];
        const targetId = await rows(page).nth(target.i).getAttribute("id");

        await page.keyboard.press("Home");
        await page.keyboard.type(first(target.label));
        await expect(trigger, `打「${first(target.label)}」应跳到「${target.label}」`).toHaveAttribute(
          "aria-activedescendant",
          targetId!,
        );
        // 全程仍只有一行高亮。
        await expect(page.locator('[role="option"][data-active="true"]')).toHaveCount(1);

        // Ctrl / Meta + 字符是浏览器快捷键：不能被 `preventDefault` 吞掉。
        const prevented = await trigger.evaluate((el) => {
          const out: boolean[] = [];
          for (const mod of ["ctrlKey", "metaKey"] as const) {
            const ev = new KeyboardEvent("keydown", { key: "c", bubbles: true, cancelable: true, [mod]: true });
            el.dispatchEvent(ev);
            out.push(ev.defaultPrevented);
          }
          return out;
        });
        expect(prevented, "Ctrl+C / Cmd+C 不应被 typeahead 吞掉").toEqual([false, false]);
      });

      test("长列表：键盘把高亮行滚进列表可视区，页面本身不跟着滚", async ({ page }) => {
        await open(page, style, kind);
        const trigger = kind.pick(page);
        await trigger.focus();
        await page.keyboard.press("ArrowDown");
        await expect(page.getByRole("listbox")).toBeVisible();
        const scrollBefore = await page.evaluate(() => window.scrollY);
        await page.keyboard.press("End");
        await settleFrames(page);
        const geo = await page.evaluate(() => {
          const row = document.querySelector('[role="option"][data-active="true"]')!;
          const list = row.closest('[role="listbox"]') as HTMLElement;
          const r = row.getBoundingClientRect();
          const l = list.getBoundingClientRect();
          return {
            inView: r.top >= l.top - 1 && r.bottom <= l.bottom + 1,
            scrolled: list.scrollHeight > list.clientHeight,
            scrollTop: list.scrollTop,
          };
        });
        // CAT 协议只有两三项，不会滚动：那种情况只要求高亮行在可视区内。
        expect(geo.inView, "高亮行应在列表可视区内").toBe(true);
        if (geo.scrolled) expect(geo.scrollTop, "长列表应已滚动").toBeGreaterThan(0);
        expect(await page.evaluate(() => window.scrollY), "页面不应被带着滚动").toBe(scrollBefore);
      });
    });
  }

  // 回归：Leptos 的组件不会各自开一个上下文作用域，并排的两个 `Select` 曾共用（并互相覆盖）同一份
  // `provide_context`：先创建的那个在打开面板时才构建选项，读到的是兄弟的值，于是它的选项
  // `aria-selected` / `data-active` 全部判错（CAT 协议的选项拿到了「QSL 寄出方式」的值）。
  // 判据：同一页上每个下拉打开后，被标为选中的恰有一行，且就是它自己触发器上显示的那一项。
  test(`${style} · 同一页上多个下拉互不串值（选中项只在自己的列表里）`, async ({ page }) => {
    await open(page, style, LOG);
    const names = ["CAT 协议", "波特率", "模式", "QSL 寄出方式"];
    for (const name of names) {
      const trigger = page.getByRole("combobox", { name, exact: true });
      await expect(trigger, `${name} 应在页面上`).toBeVisible();
      const shown = ((await trigger.textContent()) ?? "").trim();
      await trigger.click();
      await expect(page.getByRole("listbox")).toBeVisible();
      const selected = page.locator('[role="listbox"] [role="option"][aria-selected="true"]');
      await expect(selected, `${name}：应恰有一行被标为选中`).toHaveCount(1);
      expect(((await selected.textContent()) ?? "").trim(), `${name}：选中行应就是触发器上显示的项`).toBe(shown);
      await page.keyboard.press("Escape");
      await expect(page.getByRole("listbox")).toHaveCount(0);
    }
  });

  // 日志表单的「模式」选项是动态的：编辑一条导入的记录时，它的模式可能不在预设列表里（PSK63），
  // 要把它补进列表，否则触发器什么也显示不出来；模式是必填项，列表里不能有「清空」行。
  test(`${style} · 选项随值变化（导入的非预设模式）且必填项没有清空行`, async ({ page }) => {
    await page.addInitScript(() => {
      if (window !== window.top) return;
      if (sessionStorage.getItem("e2e:seeded-select")) return;
      sessionStorage.setItem("e2e:seeded-select", "1");
      localStorage.setItem(
        "logbook",
        JSON.stringify({
          entries: [
            {
              id: 1, date: "2026-01-01", time: "00:00", freq: "14.070", band: "20m", mode: "PSK63",
              callsign: "JA1AA", gridsquare: "", my_sota_ref: "", my_pota_ref: "",
              contact_sota_ref: "", contact_pota_ref: "", lotw_rcvd: false, remark: "",
            },
          ],
        }),
      );
    });
    await open(page, style, LOG);
    const mode = page.getByRole("combobox", { name: "模式", exact: true });
    await expect(mode).toHaveText("SSB");

    await page.getByRole("button", { name: "编辑" }).first().click();
    await expect(mode, "非预设模式也要显示在触发器里").toHaveText("PSK63");

    await mode.click();
    const options = page.locator('[role="listbox"] [role="option"]');
    // 预设 18 项 + 补进来的 PSK63；没有占位文案那一行（必填项不能选回空）。
    await expect(options).toHaveCount(19);
    await expect(options.last()).toHaveText("PSK63");
    await expect(options.last()).toHaveAttribute("aria-selected", "true");
    await expect(options.filter({ hasText: /^模式$/ })).toHaveCount(0);

    // 改选回预设项后，补进来的那一项随之消失（选项是随值算出来的，不是越攒越多）。
    await options.filter({ hasText: /^CW$/ }).click();
    await expect(mode).toHaveText("CW");
    await mode.click();
    await expect(options).toHaveCount(18);
    await expect(options.filter({ hasText: "PSK63" })).toHaveCount(0);
  });

  test(`${style} · 两种下拉的外观与动效完全一致（防止两份实现再次漂移）`, async ({ page }) => {
    await open(page, style, LOG);
    const measure = async (kind: Kind) => {
      const trigger = kind.pick(page);
      await trigger.scrollIntoViewIfNeeded();
      const m = await trigger.evaluate((btn) => {
        // 下拉箭头：触发器里**最后一个** svg（题库版本的触发器里前面还有自己的状态图标）。
        // 回调是在浏览器里序列化后执行的，引用不到 Node 侧定义的辅助函数，所以这里内联。
        const chevrons = btn.querySelectorAll("svg");
        const svg = chevrons[chevrons.length - 1] as SVGElement;
        const cs = getComputedStyle(svg);
        return {
          // 不比触发器高度 / 字号：那是调用方通过 `size` 选的（CAT 协议用 Sm），不是组件差异。
          chevron: `${Math.round(svg.getBoundingClientRect().width)}x${Math.round(svg.getBoundingClientRect().height)}`,
          chevronOpacity: cs.opacity,
          chevronTransition: `${cs.transitionProperty}|${cs.transitionDuration}`,
        };
      });
      await trigger.click();
      await expect(page.getByRole("listbox")).toBeVisible();
      await settleFrames(page);
      const panel = await page.evaluate(() => {
        // 外观类与动画在外层面板上，listbox 是它里面可滚动的那一层。
        const list = document.querySelector('[role="listbox"]')!.parentElement!;
        const cs = getComputedStyle(list);
        // 取最后一行：它在两个列表里都不是选中项（选中项是加粗的，字重不同）。
        const opts = document.querySelectorAll('[role="option"]');
        const opt = opts[opts.length - 1]!;
        const oc = getComputedStyle(opt);
        return {
          // 面板外观类（`pxl-popover` / `md-popover`）与入场动画。
          panelClass: [...list.classList].filter((c) => /^(pxl|md)-/.test(c)).sort().join(" "),
          anim: `${cs.animationName}|${cs.animationDuration}`,
          rowClass: [...opt.classList].filter((c) => /^(pxl|md)-/.test(c)).sort().join(" "),
          // 与内容无关的结构样式。不比像素高度：含中日韩字符的行与纯英文行的行盒高度本来就差零点几像素，
          // 那是内容决定的，不是组件差异（高度单独按 ±1px 比，见下）。
          rowStyle: [oc.paddingTop, oc.paddingBottom, oc.paddingLeft, oc.paddingRight, oc.fontSize, oc.fontWeight, oc.lineHeight, oc.display].join("|"),
          rowHeight: opt.getBoundingClientRect().height,
        };
      });
      await page.keyboard.press("Escape");
      await expect(page.getByRole("listbox")).toHaveCount(0);
      return { ...m, ...panel };
    };
    // 同页的两个数据式下拉（2 项与 18 项）：外观、动效、结构必须逐项相等。
    const [a, b] = [await measure(KINDS[0]), await measure(KINDS[1])];
    const { rowHeight: ha, ...restA } = a;
    const { rowHeight: hb, ...restB } = b;
    expect(restB, `${style}：同是数据式的两个下拉应完全一致`).toEqual(restA);
    expect(Math.abs(ha - hb), `${style}：选项行高度相差 ${ha} vs ${hb}`).toBeLessThanOrEqual(1);
  });

  // 组合式与数据式必须是同一个外观 —— 这是「两个下拉合成一个引擎」的验收：它们曾经是两份实现，
  // 箭头一个渐变一个瞬切、选中行一个多一枚对勾。组合式只在像素风首页，所以只在像素风下对照。
  test(`${style} · 组合式与数据式的下拉外观与动效完全一致`, async ({ page }) => {
    test.skip(style !== "pixel", "组合式下拉（题库版本）只出现在像素风首页");
    await open(page, style, HOME_COMBINED);
    const compare = async (kind: Kind) => {
      const trigger = kind.pick(page);
      await trigger.scrollIntoViewIfNeeded();
      const m = await trigger.evaluate((btn) => {
        // 下拉箭头：触发器里**最后一个** svg（题库版本的触发器里前面还有自己的状态图标）。
        // 回调是在浏览器里序列化后执行的，引用不到 Node 侧定义的辅助函数，所以这里内联。
        const chevrons = btn.querySelectorAll("svg");
        const svg = chevrons[chevrons.length - 1] as SVGElement;
        const cs = getComputedStyle(svg);
        return {
          chevron: `${Math.round(svg.getBoundingClientRect().width)}x${Math.round(svg.getBoundingClientRect().height)}`,
          chevronOpacity: cs.opacity,
          chevronTransition: `${cs.transitionProperty}|${cs.transitionDuration}`,
        };
      });
      await trigger.click();
      await expect(page.getByRole("listbox")).toBeVisible();
      await settleFrames(page);
      const panel = await page.evaluate(() => {
        const list = document.querySelector('[role="listbox"]')!.parentElement!;
        const cs = getComputedStyle(list);
        const opts = document.querySelectorAll('[role="option"]');
        const opt = opts[opts.length - 1]!;
        const oc = getComputedStyle(opt);
        return {
          panelClass: [...list.classList].filter((c) => /^(pxl|md)-/.test(c)).sort().join(" "),
          anim: `${cs.animationName}|${cs.animationDuration}`,
          rowClass: [...opt.classList].filter((c) => /^(pxl|md)-/.test(c)).sort().join(" "),
          // 选项行里除了文字容器不应有别的元素（以前数据式下拉多画一枚绝对定位的像素对勾）。
          rowKids: opt.children.length,
        };
      });
      await page.keyboard.press("Escape");
      await expect(page.getByRole("listbox")).toHaveCount(0);
      return { ...m, ...panel };
    };
    // 组合式只在像素风首页、数据式在日志页：这里只比计算样式，跨页对照不影响结论。
    const combined = await compare(HOME_COMBINED);
    await open(page, style, LOG);
    const simple = await compare(LOG);
    expect(simple, "组合式与数据式的下拉外观 / 动效 / 结构应完全一致").toEqual(combined);
  });
}

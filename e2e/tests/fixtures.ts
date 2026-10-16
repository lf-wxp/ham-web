import {
  expect,
  test as base,
  type Locator,
  type Page,
} from "@playwright/test";

export { expect };

// 练习 / 考试首次进入会自动弹出快捷键说明，默认标记为已看过，避免遮挡后续操作
export const test = base.extend<{ helpSeen: boolean }>({
  helpSeen: [true, { option: true }],
  page: async ({ page, helpSeen }, use) => {
    if (helpSeen) {
      await page.addInitScript(() => {
        // addInitScript 也会进入 axe / 第三方 iframe，种子只属于被测主文档。
        if (window !== window.top || !["http:", "https:"].includes(location.protocol)) return;
        localStorage.setItem("ui:shortcutsHelpSeen:practice", "1");
        localStorage.setItem("ui:shortcutsHelpSeen:exam", "1");
      });
    }
    await use(page);
  },
});

/** 当前题目中字母为 `letter` 的选项：练习 / 考试为 radio 或 checkbox，错题重练为切换按钮。 */
export function option(page: Page, letter: string) {
  const main = page.locator("main");
  // 「A 类」这类题库切换 chip 也是 radio，名字与选项字母前缀相同；负向断言把「类」排除，
  // 只留题目选项（「A.…」或「A …」）。
  const name = new RegExp(`^${letter}(\\s*\\.|\\s(?!类))`);
  return main
    .getByRole("radio", { name })
    .or(main.getByRole("checkbox", { name }))
    .or(main.locator("button[aria-pressed]").filter({ hasText: name }))
    .first();
}

/**
 * 等页面「还在加载」的迹象消失。
 *
 * 挂载完成 ≠ 文案就绪：多数页面要读完 IndexedDB（kv 门面）或一条接口才渲染计数类文案。
 * 以前的写法是固定 `waitForTimeout(150)`，那只是赌这段时间已经够 —— 慢机与 CI 上会扫到
 * 中间态。骨架块（`.skeleton`）与 spinner（`.animate-spin`）是各页面表达「还没好」的
 * 统一方式，等它们消失是一个**可复现**的判据，也天然自适应（快的页面 ~100ms 就过）。
 *
 * 到点仍未消失时不报错：这条路只是「尽量等到稳定」，断言本身仍是各用例自己的事。
 */
export async function waitSettled(page: Page) {
  await page
    .waitForFunction(
      () =>
        !document.querySelector("main .skeleton") &&
        !document.querySelector("main .animate-spin"),
      undefined,
      { timeout: 15_000, polling: 100 },
    )
    .catch(() => undefined);
}

/**
 * 等页面上所有 CSS / Web 动画播完，并再让出两帧。
 *
 * 用途有二：① axe 在入场动画中途（`animate-in` 300 ms）取到的是半透明色，对比度会误报；
 * ② 「断言某件事**没有**发生」的用例（滑动太短不翻页、重载后弹窗不再出现）需要一个确定的
 * 「该发生的都已经发生过了」时刻，而不是赌一个固定毫秒数。
 *
 * 无限循环的动画（spinner、`steps()` 逐帧）没有 `finished`，只等有限时长的那些。
 */
export async function settleFrames(page: Page) {
  await page.evaluate(async () => {
    const finite = document
      .getAnimations()
      .filter((a) => Number.isFinite(a.effect?.getComputedTiming().endTime ?? 0));
    await Promise.allSettled(finite.map((a) => a.finished));
    await new Promise<void>((r) => requestAnimationFrame(() => requestAnimationFrame(() => r())));
  });
}

/** 练习页「正确答案：A, C」中的字母。 */
export async function correctLetters(page: Page): Promise<string[]> {
  const text = await page.getByText(/^正确答案：/).first().innerText();
  return text.replace(/^正确答案：/, "").split(/[,，\s]+/).filter(Boolean);
}

/** 第一个不在正确答案中的选项字母。 */
export function wrongLetter(correct: string[]): string {
  const letter = ["A", "B", "C", "D"].find((l) => !correct.includes(l));
  if (!letter) throw new Error(`no wrong option among ${correct.join(",")}`);
  return letter;
}

/**
 * 弹层式下拉（`ui::Select` / `ui::NativeSelect`）：点触发器 → 在弹层里点选项。
 *
 * 它们不是原生 `<select>`，因此 `selectOption()` 用不了，必须走 `role=combobox` +
 * `role=option` 这条路径（与导航栏语言切换一致，见 navigation.spec.ts）。
 */
export async function pickOption(
  page: Page,
  name: string | RegExp,
  option: string | RegExp,
) {
  await page.getByRole("combobox", { name }).click();
  await page.getByRole("option", { name: option, exact: true }).click();
}

/**
 * 弹层式日期选择器：点触发器 → 翻到目标月份 → 点那一天。
 *
 * 月份标题按中文（`2030年6月`）解析 —— e2e 默认跑中文界面。
 */
export async function pickDate(page: Page, name: string | RegExp, iso: string) {
  const [y, m, d] = iso.split("-").map(Number);
  await page.getByRole("combobox", { name }).click();
  const panel = page.locator('[data-slot="date-picker"]');
  const title = panel.locator('[data-slot="date-picker-title"]');
  const want = y * 12 + m;
  for (let i = 0; i < 240; i += 1) {
    const text = await title.innerText();
    const match = text.match(/(\d+)年(\d+)月/);
    if (!match) throw new Error(`无法解析月份标题：${text}`);
    const cur = Number(match[1]) * 12 + Number(match[2]);
    if (cur === want) break;
    await panel
      .getByRole("button", { name: cur < want ? "下个月" : "上个月" })
      .click();
  }
  await panel.getByRole("button", { name: `${y}年${m}月${d}日` }).click();
  await expect(panel).toBeHidden();
}

/**
 * 一个「近未来」的日期（默认今天 +60 天）。
 *
 * 用例里不要写死 `2030-06-01` 这类远期日期：`pickDate` 是**逐月点击**的，从今天翻到
 * 2030 年要 40 多下，每次点击都过一次渲染与动画。单独跑十来秒能过，
 * `fullyParallel` 全量并行时这几条用例会一起撞 60 秒超时（判据只要求「在未来」，近未来等价）。
 */
export function futureIso(daysAhead = 60): string {
  const now = new Date();
  const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + daysAhead);
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mm}-${dd}`;
}

/**
 * 点一个「可能被吸顶导航盖住」的元素：先滚到视口中间再点。
 *
 * 导航是 `sticky top-0 z-50`（高 64px）：目标一旦落在视口顶部这 64px 里，Playwright 仍判定
 * 它「已在视口内」而不再滚动，点击会被导航接住（`subtree intercepts pointer events`），
 * 之后反复重试到用例超时 —— 手工跑不出来，只有在长交互（翻月、弹层）之后滚动位置漂移时才现。
 */
export async function clickClear(locator: Locator) {
  await locator.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await locator.click();
}

/** 弹层式时间选择器：点触发器 → 依次点「时」「分」。 */
export async function pickTime(page: Page, name: string | RegExp, hhmm: string) {
  const [h, minute] = hhmm.split(":");
  await page.getByRole("combobox", { name }).click();
  const panel = page.locator('[data-slot="time-picker"]');
  const columns = panel.getByRole("listbox");
  await columns.nth(0).getByRole("option", { name: h, exact: true }).click();
  await columns.nth(1).getByRole("option", { name: minute, exact: true }).click();
  await expect(panel).toBeHidden();
}

/** 种子日志的一条记录：只需给出关心的字段，其余必填字段补默认值。 */
export type SeedQso = {
  callsign: string;
  date?: string;
  time?: string;
  freq?: string;
  band?: string;
  mode?: string;
  gridsquare?: string;
  /** 本台所在的山峰 / 公园（`my_*`）与对方的（`contact_*`，S2S / P2P）。 */
  my_sota_ref?: string;
  my_pota_ref?: string;
  contact_sota_ref?: string;
  contact_pota_ref?: string;
};

/**
 * 在页面脚本运行前写入日志快照（`logbook`）。
 *
 * 必须走 `addInitScript`：不少页面只在挂载时读一次日志，先 `goto` 再写就晚了。
 * IndexedDB 在新 context 里是空的，加载时自然以这份快照为准。
 */
export async function seedLog(page: Page, qsos: SeedQso[]) {
  const entries = qsos.map((q, i) => ({
    id: i + 1,
    date: q.date ?? "2026-01-01",
    time: q.time ?? "00:00",
    freq: q.freq ?? "14.074",
    band: q.band ?? "",
    mode: q.mode ?? "FT8",
    callsign: q.callsign,
    gridsquare: q.gridsquare ?? "",
    my_sota_ref: q.my_sota_ref ?? "",
    my_pota_ref: q.my_pota_ref ?? "",
    contact_sota_ref: q.contact_sota_ref ?? "",
    contact_pota_ref: q.contact_pota_ref ?? "",
    remark: "",
  }));
  await page.addInitScript((json) => {
    // 只在首次加载时写：测试里 reload 后应看到页面自己写回的数据，而不是被种子覆盖。
    if (!sessionStorage.getItem("e2e:seeded-log")) {
      localStorage.setItem("logbook", json);
      sessionStorage.setItem("e2e:seeded-log", "1");
    }
  }, JSON.stringify({ entries }));
}

/**
 * 给 `/api/*` 打桩：`routes` 的 key 是不含查询串的路径（如 `/api/spots`），
 * 值是 JSON 响应体，或按完整 URL 返回响应体的函数。未登记的接口一律 503 ——
 * 页面后台的轮询（卫星提醒、太阳活动等）不会偷偷打到真实网络。
 */
export async function stubApi(
  page: Page,
  routes: Record<string, unknown | ((url: URL) => unknown)>,
) {
  await page.route("**/api/**", (route) => {
    const url = new URL(route.request().url());
    const hit = routes[url.pathname];
    if (hit === undefined) {
      return route.fulfill({ status: 503, contentType: "text/plain", body: "stubbed: not registered" });
    }
    const body = typeof hit === "function" ? (hit as (u: URL) => unknown)(url) : hit;
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
}

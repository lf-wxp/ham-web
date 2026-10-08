import type { Page } from "@playwright/test";
import { expect, test, waitSettled } from "./fixtures";

/**
 * 复数回归：语种切到 en / es，把本地数据种成「每项恰好 1 条」，
 * 断言页面上不再出现 `1 days` / `1 questions` / `1 cards` 这类「数量为一却用复数」的文案。
 *
 * 种子写在 `addInitScript` 里：日志 1 条通联 + 错题本 1 条已到期错题，
 * 于是首页的「今日待复习」、错题本的「共 N 道错题」、日志的「共 N 条」都取到 count = 1。
 * 判据是**纯文本扫描**（可见文案 + `title` / `aria-label` 悬浮提示），
 * 因此将来任何一条计数型文案漏迁 `tp()`，都会被这条用例抓住。
 */

const LANGS = [
  { code: "en", name: "English" },
  { code: "es", name: "Español" },
] as const;

/** 数量为一却跟着复数名词 —— 迁移前的 `1 days` / `1 preguntas` 正是这类语法错误。 */
const PLURAL_AFTER_ONE: Record<string, string> = {
  en: "days|questions|cards|items|matches|mistakes|records|QSOs|grids|labels|pages|hours|times|attempts|points|lines|entries|zones|squares|entities|passes|months|stages|reports|countdowns|callsigns",
  es: "días|preguntas|tarjetas|elementos|coincidencias|errores|registros|cuadrículas|etiquetas|páginas|horas|veces|intentos|puntos|líneas|entradas|zonas|contactados|entidades|pasos|meses|etapas|informes|cuentas|indicativos",
};

/** 对照：同一批名词的单数形式。至少命中一次，才说明种子真的造出了 count = 1 的文案。 */
const SINGULAR_AFTER_ONE: Record<string, string> = {
  en: "day|question|card|item|match|mistake|record|QSO|grid|label|page|hour|time|attempt|point|line|entry|zone|square|entity|pass|month|stage|report|countdown|callsign",
  es: "día|pregunta|tarjeta|elemento|coincidencia|error|registro|cuadrícula|etiqueta|página|hora|vez|intento|punto|línea|entrada|zona|contactado|entidad|paso|mes|etapa|informe|cuenta|indicativo",
};

/**
 * 会渲染计数型文案的路由（与 `main_content.rs` 保持一致）。
 * 覆盖面决定这条用例的杀伤力：第三批迁的 `stats` / `dxcc-map` / `zone-map` /
 * `notifications` 等页面原先不在扫描范围内，漏迁是抓不到的。
 */
const ROUTES = [
  "/",
  "/practice",
  "/daily-challenge",
  "/print",
  "/mistakes",
  "/mistake-topics",
  "/bookmarks",
  "/flashcards",
  "/log",
  "/grid-map",
  "/dxcc-map",
  "/zone-map",
  "/qsl-labels",
  "/cabrillo",
  "/cards",
  "/progress",
  "/stats",
  "/weekly",
  "/report",
  "/achievements",
  "/study-calendar",
  "/exam-review",
  "/contest",
  "/contest-log",
  "/countdown",
  "/satellites",
  "/dx-spots",
  "/repeater",
  "/cheat-sheet",
  "/gear",
  "/morse",
  "/tools",
  "/notifications",
  "/dashboard",
];

/** count = 1 的种子：日志 1 条通联、错题本 1 条已到期错题。 */
const SEED = (lang: string) => {
  const now = Date.now();
  localStorage.setItem("locale", lang);
  // 关掉首次启动的历史回填，保证计数只来自下面的种子
  localStorage.setItem("mistake-book:seeded", "1");
  localStorage.setItem("study-stats:seen-seeded", "1");
  localStorage.setItem(
    "logbook",
    JSON.stringify({
      entries: [
        {
          id: 1,
          date: "2026-01-01",
          time: "00:00",
          time_off: "",
          freq: "14.070",
          band: "20m",
          mode: "FT8",
          callsign: "BH1ABC",
          rst_sent: "",
          rst_rcvd: "",
          tx_pwr: "",
          gridsquare: "PM95",
          name: "",
          qth: "",
          prop_mode: "",
          sat_name: "",
          sota_ref: "",
          pota_ref: "",
          contest_id: "",
          stx: "",
          srx: "",
          dxcc: "",
          cqz: "",
          ituz: "",
          state: "",
          iota: "",
          remark: "",
          qsl_sent: false,
          qsl_rcvd: false,
        },
      ],
    }),
  );
  localStorage.setItem(
    "mistake-book",
    JSON.stringify({
      records: [
        {
          key: "seed-one",
          question: {
            id: "A-1",
            question: "种子题",
            options: [{ key: "A", text: "选项 A" }],
            answer_keys: ["A"],
            type: "single",
          },
          my_answer: [],
          wrong_count: 1,
          streak: 0,
          last_wrong_ms: now,
          // 已到期 → 计入「今日待复习」
          due_ms: now - 1000,
          banks: ["A"],
          ease: 2.5,
          interval_days: 1,
          last_review_ms: now,
          cause: null,
        },
      ],
    }),
  );
};

/** 收集页面上的文案：叶子节点的可见文本 + `title` / `aria-label`（悬浮提示里也有计数文案）。 */
const SWEEP = () => {
  const texts: string[] = [];
  for (const el of Array.from(document.querySelectorAll("main *"))) {
    if (el.children.length === 0 && el.textContent) texts.push(el.textContent);
    for (const attr of ["title", "aria-label"]) {
      const v = el.getAttribute(attr);
      if (v) texts.push(v);
    }
  }
  return texts.map((t) => t.replace(/\s+/g, " ").trim()).filter(Boolean);
};

/**
 * 标题写成字符串字面量而非模板串：`scripts/coverage.mjs` 是拿源码里的标题做子串匹配的，
 * 模板串会保留 `${lang.name}`，旅程就永远匹配不上。
 */
async function assertSingularForOne(page: Page, lang: { code: string; name: string }) {
  await page.addInitScript(SEED, lang.code);

  const bad: string[] = [];
  let singular = 0;
  const badRe = new RegExp(`\\b1\\s+(?:${PLURAL_AFTER_ONE[lang.code]})\\b`);
  const okRe = new RegExp(`\\b1\\s+(?:${SINGULAR_AFTER_ONE[lang.code]})\\b`);

  for (const route of ROUTES) {
    await page.goto(route, { waitUntil: "domcontentloaded" });
    await page.waitForSelector("main", { timeout: 15_000 }).catch(() => undefined);
    // 等加载占位消失，而不是固定 sleep 150ms：计数文案是页面读完 IndexedDB（kv 门面）之后
    // 才渲染的，睡固定时长只是赌这段时间够用 —— 赌输时该路由整块文案还没出现，
    // 复数违规就被「没扫到」放过去了（假绿）。
    await waitSettled(page);

    for (const text of await page.evaluate(SWEEP)) {
      if (badRe.test(text)) bad.push(`${route} :: ${text}`);
      if (okRe.test(text)) singular += 1;
    }
  }

  expect(
    bad,
    `${lang.name}：count = 1 却用了复数形式，共 ${bad.length} 处\n${bad.map((b) => `  - ${b}`).join("\n")}`,
  ).toEqual([]);
  expect(
    singular,
    `${lang.name}：种子没造出任何 count = 1 的文案，断言等于没跑（检查种子数据格式）`,
  ).toBeGreaterThan(0);
}

test("复数：English 下 count = 1 的文案用单数形式", async ({ page }) => {
  await assertSingularForOne(page, LANGS[0]);
});

test("复数：Español 下 count = 1 的文案用单数形式", async ({ page }) => {
  await assertSingularForOne(page, LANGS[1]);
});

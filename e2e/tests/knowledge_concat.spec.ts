import { expect, test } from "./fixtures";

// 回归：`concat!` 拼出来的正文曾因译文 key 是片段、运行时拿到整句而命中不到，
// 英文 / 西语界面整段停在中文。现在含中文的正文一律是单个字面量（`knowledge-i18n-check` 守着）。
// 这里每页取一条原先用 `concat!` 写的句子：译文必须出现，原中文必须消失。
const CASES: {
  path: string;
  zh: string;
  en: RegExp;
  es: RegExp;
}[] = [
  {
    path: "/balun",
    zh: "注意与 Windom",
    en: /do not confuse the two/,
    es: /no las confunda/,
  },
  {
    path: "/safety",
    zh: "导线绝缘层不保证安全",
    en: /Wire insulation is no guarantee of safety/,
    es: /El aislamiento de los conductores no garantiza la seguridad/,
  },
  {
    path: "/grounding",
    zh: "不可独立接地",
    en: /not grounded independently/,
    es: /no con tierra independiente/,
  },
  {
    path: "/receiver",
    zh: "只提 P1dB",
    en: /Quoting only P1dB \/ IP3 misses the other half/,
    es: /Citar solo P1dB \/ IP3 omite la otra mitad/,
  },
  {
    path: "/ft8",
    zh: "mW 级只在极好传播",
    en: /milliwatt level only holds in beacon-like cases/,
    es: /el nivel de miliwatios solo se da en casos tipo baliza/,
  },
  {
    path: "/aurora",
    zh: "法拉第旋转量",
    en: /Faraday rotation scales as f⁻²/,
    es: /la rotación de Faraday es proporcional a f⁻²/,
  },
];

for (const lang of ["en", "es"] as const) {
  for (const c of CASES) {
    test(`知识库正文：concat 拼接的句子在 ${lang} 界面显示译文（${c.path}）`, async ({ page }) => {
      await page.addInitScript((l) => localStorage.setItem("locale", l), lang);
      await page.goto(c.path);
      await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
      // 先等译文出现（词典是异步拉取的），再断言原中文已消失；
      // 用 textContent 以免被折叠的区块排除在 innerText 之外。
      await expect(page.locator("main")).toContainText(lang === "en" ? c.en : c.es);
      const text = (await page.locator("main").textContent()) ?? "";
      expect(text).not.toContain(c.zh);
    });
  }
}

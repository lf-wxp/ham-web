import { expect, test } from "./fixtures";

// 阅读型专题页：没有交互，要钉死的是「八个板块都在、写进去的核对过的数字没被改坏、三语都译过」。
// 数字的来源见 `crates/core/src/antenna_topics.rs` 与 `crates/nec/reference/antenna_topics_necpp.py`。

const SECTIONS = {
  zh: [
    "垂直天线与地网",
    "垂直天线要点",
    "天线陷阱（Trap）",
    "陷阱要点",
    "对数周期天线（LPDA）",
    "对数周期天线要点",
    "dBi 与 dBd：增益的口径",
    "增益口径要点",
  ],
  en: [
    "Vertical antennas and radials",
    "Vertical antenna notes",
    "Antenna traps",
    "Trap notes",
    "Log-periodic dipole arrays (LPDA)",
    "LPDA notes",
    "dBi and dBd: the basis of a gain figure",
    "Gain-basis notes",
  ],
  es: [
    "Antenas verticales y radiales",
    "Notas sobre antenas verticales",
    "Trampas de antena",
    "Notas sobre trampas",
    "Arreglos de dipolos log-periódicos (LPDA)",
    "Notas sobre LPDA",
    "dBi y dBd: la referencia de una cifra de ganancia",
    "Notas sobre la referencia de ganancia",
  ],
} as const;

test("天线补充专题：八个板块与核对过的数字", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("locale", "zh"));
  await page.goto("/antenna-topics");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("天线补充专题");
  for (const title of SECTIONS.zh) {
    await expect(page.getByRole("heading", { level: 2, name: title, exact: true })).toBeVisible();
  }
  // `<main>` 里还有题库抽出来的「专题自测」（题目只有中文原文），只看路由内容这一层。
  const main = await page.locator("#route-content").innerText();
  for (const needle of [
    // 镜像原理：necpp 核对
    "72.1 Ω / 2.14 dBi",
    "36.0 Ω / 5.15 dBi",
    // 地网：效率算术与下垂地网
    "效率只剩 50%（−3 dB）",
    "效率却只有 72%",
    "30° 约 43 Ω",
    "45° 约 53 Ω",
    // FCC 条文
    "至少 90 根",
    "47 CFR §73.189(b)(4)",
    // LPDA 算例（由 `lpda_design` 的单测对着核对）
    "cot α = 4σ ÷ (1 − τ) = 6.40",
    "α = 8.9°",
    "N = 12.0，取 12 根",
    "L = (λ_max ÷ 4)(1 − 1/B_s)·cot α = 23.5 m",
    "臂长缩到 6.4 m",
    // 增益口径
    "EIRP = ERP × 1.64（+2.15 dB）",
    "4.8–6.9 dB",
    "3.2–5.6 dB",
  ]) {
    expect(main, `缺 ${needle}`).toContain(needle);
  }
});

for (const lang of ["en", "es"] as const) {
  test(`天线补充专题：${lang} 界面的标题与正文都已翻译（不漏中文）`, async ({ page }) => {
    await page.addInitScript((l) => localStorage.setItem("locale", l), lang);
    await page.goto("/antenna-topics");
    for (const title of SECTIONS[lang]) {
      await expect(
        page.getByRole("heading", { level: 2, name: title, exact: true }),
        title,
      ).toBeVisible();
    }
    // 正文译文是异步拉的：等到至少一条译文出现再取全文。
    const probe = lang === "en" ? "Image principle" : "Principio de la imagen";
    await expect(page.getByText(probe, { exact: true })).toBeVisible();
    const main = await page.locator("#route-content").innerText();
    const leaked = main.match(/[一-鿿]+/g) ?? [];
    expect(leaked, `${lang} 界面里漏出的中文`).toEqual([]);
    // 数字没有因翻译而改动：同一批关键数字在译文里原样出现。
    for (const needle of ["36.0 Ω / 5.15 dBi", "B_s = 3.19", "23.5 m", "4.8–6.9 dB", "3.2–5.6 dB"]) {
      expect(main, `${lang} 缺 ${needle}`).toContain(needle);
    }
  });
}

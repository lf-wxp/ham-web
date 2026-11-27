import { expect, stubApi, test, type Page } from "./fixtures";

/**
 * `/dashboard` 的「现在守哪一段」：这个钟点、这张操作证，该守哪几个波段窗口。
 *
 * - 证不允许的窗口不出现（A 类只有 6 m / 2 m / 70 cm）；功率上限按办法第三十条；
 * - 有本台网格 + 对方网格 + 太阳黑子数才给可靠度并排序，缺任何一样就只列窗口、不编「好 / 差」；
 * - 可靠度与服务端 `/api/voacap` 逐波段对照（页面直接用核心，另一条代码路径）；
 * - K ≥ 4 不折减可靠度，只有传播等级提示随 K 变；
 * - 灰线只是几何：用这里独立写的球面公式核对距离与黎明 / 黄昏侧。
 */

const BANDS = ["160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm"];
const HF = BANDS.slice(0, 10);
const VHF = ["6m", "2m", "70cm"];

const card = (page: Page) => page.getByTestId("watch-card");
const rows = (page: Page) => card(page).locator("[data-window]");
const bandsShown = async (page: Page) =>
  page.locator('[data-testid="watch-card"] [data-window]').evaluateAll((els) => els.map((e) => e.getAttribute("data-window")));
const chip = (page: Page, cls: "A" | "B" | "C") => card(page).getByRole("button", { name: `${cls} 类`, exact: true });

type Solar = { k_index: number | null; solar_flux: number | null; sunspots: number | null };
const QUIET: Solar = { k_index: 2, solar_flux: 150, sunspots: 100 };

async function open(
  page: Page,
  opts: { grid?: string | null; solar?: Solar; when?: string; cls?: "A" | "B" | "C" } = {},
) {
  const { grid = "OM89", solar = QUIET, when = "2026-10-15T12:00:00Z", cls } = opts;
  await page.addInitScript(
    ([g, c]) => {
      if (g) {
        localStorage.setItem(
          "station-info",
          JSON.stringify({ callsign: "BG4XYZ", operator: "", gridsquare: g, rig: "", antenna: "" }),
        );
      }
      if (c) localStorage.setItem("cat-settings", JSON.stringify({ class: c }));
    },
    [grid, cls ?? null],
  );
  await stubApi(page, { "/api/solar": solar });
  // 只钉住 `Date`：定时器照常跑，页面每 30 秒刷新的「此刻」永远是这一刻。
  await page.clock.setFixedTime(new Date(when));
  await page.goto("/dashboard");
  await expect(card(page)).toBeVisible();
}

/** 独立写的功率期望（办法第三十条与 ITU 脚注 5.133B）。 */
function expectedPower(cls: "A" | "B" | "C", band: string): number {
  if (VHF.includes(band)) return 25;
  if (band === "60m") return 15; // 5.133B：15 W e.i.r.p.，比类别上限小
  return cls === "B" ? 15 : 1000;
}

test("现在守哪一段：没选操作证类别时只给提示，不列任何窗口", async ({ page }) => {
  await open(page);
  await expect(card(page).getByTestId("watch-pick")).toContainText("先选上你的操作证类别");
  await expect(rows(page)).toHaveCount(0);
});

test("现在守哪一段：A 类只有 30 MHz 以上的窗口，HF 一个都不出现", async ({ page }) => {
  await open(page);
  await chip(page, "A").click();
  expect(await bandsShown(page)).toEqual(VHF);
  for (const row of await rows(page).all()) await expect(row).toContainText("≤ 25 W");
  // 没选类别时的提示消失。
  await expect(card(page).getByTestId("watch-pick")).toHaveCount(0);
});

test("现在守哪一段：B / C 类窗口齐全，功率上限各按各的（60 m 受脚注限制）", async ({ page }) => {
  await open(page);
  for (const cls of ["B", "C"] as const) {
    await chip(page, cls).click();
    expect(await bandsShown(page), cls).toEqual(BANDS);
    for (const band of BANDS) {
      await expect(card(page).locator(`[data-window="${band}"]`), `${cls} ${band}`).toContainText(
        `≤ ${expectedPower(cls, band)} W`,
      );
    }
  }
});

test("现在守哪一段：类别记在本机，刷新后还在，再点一下取消选择", async ({ page }) => {
  await open(page);
  await chip(page, "C").click();
  await expect(chip(page, "C")).toHaveAttribute("aria-pressed", "true");
  await expect
    .poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("cat-settings") ?? "{}").class))
    .toBe("C");
  await page.reload();
  await expect(chip(page, "C")).toHaveAttribute("aria-pressed", "true");
  expect(await bandsShown(page)).toEqual(BANDS);

  await chip(page, "C").click();
  await expect(card(page).getByTestId("watch-pick")).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("cat-settings") ?? "{}").class ?? null))
    .toBeNull();
});

test("现在守哪一段：类别与 CAT 频率护栏共用一份设置，只改 class、不动串口参数", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem("cat-settings", JSON.stringify({ protocol: "icom", baud: 19200, civ_addr: 148, class: "A" })),
  );
  await open(page);
  await expect(chip(page, "A")).toHaveAttribute("aria-pressed", "true"); // 读到了护栏存的类别
  await chip(page, "B").click();
  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem("cat-settings") ?? "{}"));
  expect(saved.class).toBe("B");
  expect(saved.protocol).toBe("icom");
  expect(saved.baud).toBe(19200);
  expect(saved.civ_addr).toBe(148);
});

test("现在守哪一段：没有对方网格时不给可靠度、不排序，也没有路径", async ({ page }) => {
  await open(page, { cls: "C" });
  await expect(card(page).getByTestId("watch-windows")).toHaveAttribute("data-ranked", "false");
  expect(await bandsShown(page)).toEqual(BANDS); // 按频率从低到高
  for (const r of await rows(page).all()) expect(await r.getAttribute("data-reliability")).toBe("");
  await expect(card(page).getByTestId("watch-path")).toHaveCount(0);
});

test("现在守哪一段：缺太阳黑子数时即使有双方网格也不编好坏", async ({ page }) => {
  await open(page, { cls: "C", solar: { k_index: 2, solar_flux: 150, sunspots: null } });
  await card(page).getByLabel("对方网格（可选）").fill("IO91");
  await expect(card(page).getByTestId("watch-windows")).toHaveAttribute("data-ranked", "false");
  for (const r of await rows(page).all()) expect(await r.getAttribute("data-reliability")).toBe("");
});

test("现在守哪一段：缺本台网格时没有灰线、没有路径、没有可靠度", async ({ page }) => {
  await open(page, { cls: "C", grid: null });
  await card(page).getByLabel("对方网格（可选）").fill("IO91");
  await expect(card(page).getByTestId("watch-grey")).toHaveCount(0);
  await expect(card(page).getByTestId("watch-path")).toHaveCount(0);
  await expect(card(page).getByTestId("watch-windows")).toHaveAttribute("data-ranked", "false");
});

test("现在守哪一段：可靠度与服务端 /api/voacap 逐波段一致，且按可靠度降序", async ({ page, request }) => {
  await open(page, { cls: "C", grid: "OM89", solar: { k_index: 2, solar_flux: 150, sunspots: 100 } });
  await card(page).getByLabel("对方网格（可选）").fill("IO91");
  await expect(card(page).getByTestId("watch-windows")).toHaveAttribute("data-ranked", "true");

  // 与页面完全独立的一条路径：服务端的同名接口（2026-10-15 12:00 UTC → 10 月、12 点、SSN 100）。
  const api = await request.get("/api/voacap?tx=OM89&rx=IO91&month=10&ssn=100&hour=12");
  expect(api.ok()).toBeTruthy();
  const pred = (await api.json()) as { bands: { band: string; reliability: number }[]; distance_km: number; bearing_deg: number };

  const shown = await page.locator('[data-testid="watch-card"] [data-window]').evaluateAll((els) =>
    els.map((e) => [e.getAttribute("data-window"), e.getAttribute("data-reliability")]),
  );
  // HF 十个波段在前、可靠度降序；6 m / 2 m / 70 cm 没有电离层模型，殿后且没有数字。
  expect(shown.slice(0, 10).map((s) => s[0]).sort()).toEqual([...HF].sort());
  expect(shown.slice(10)).toEqual([["6m", ""], ["2m", ""], ["70cm", ""]]);
  const percents = shown.slice(0, 10).map((s) => Number(s[1]));
  expect([...percents].sort((a, b) => b - a)).toEqual(percents);
  for (const [band, pct] of shown.slice(0, 10)) {
    const want = pred.bands.find((b) => b.band === band)!;
    expect(Number(pct), band!).toBe(Math.round(want.reliability * 100));
  }
  // 路径：方位与距离也与接口一致。
  await expect(card(page).getByTestId("watch-path")).toContainText(
    `方位 ${pred.bearing_deg.toFixed(0)}°，${pred.distance_km.toFixed(0)} km`,
  );
});

test("现在守哪一段：时刻与太阳黑子数都会改变可靠度（不是写死的）", async ({ browser }) => {
  const run = async (solar: Solar, when: string) => {
    const ctx = await browser.newContext();
    const page = await ctx.newPage();
    await open(page, { cls: "C", solar, when });
    await card(page).getByLabel("对方网格（可选）").fill("IO91");
    await expect(card(page).getByTestId("watch-windows")).toHaveAttribute("data-ranked", "true");
    const rel = Object.fromEntries(
      await page
        .locator('[data-testid="watch-card"] [data-window]')
        .evaluateAll((els) => els.map((e) => [e.getAttribute("data-window"), e.getAttribute("data-reliability")])),
    ) as Record<string, string>;
    await ctx.close();
    return rel;
  };
  // 黑子数 0、深夜：10 m 过不去（北京 → 伦敦）。
  const quiet = await run({ k_index: 2, solar_flux: 70, sunspots: 0 }, "2026-10-15T02:00:00Z");
  expect(quiet["10m"]).toBe("0");
  // 黑子数 200、正午：20 m 总有可靠度；同一波段与上面的数不同。
  const busy = await run({ k_index: 2, solar_flux: 200, sunspots: 200 }, "2026-10-15T12:00:00Z");
  expect(Number(busy["20m"])).toBeGreaterThan(0);
  expect(busy).not.toEqual(quiet);
});

test("现在守哪一段：K ≥ 4 不折减可靠度，只有传播等级提示随 K 变", async ({ browser }) => {
  const run = async (k: number) => {
    const ctx = await browser.newContext();
    const page = await ctx.newPage();
    await open(page, { cls: "C", solar: { k_index: k, solar_flux: 150, sunspots: 100 } });
    await card(page).getByLabel("对方网格（可选）").fill("IO91");
    await expect(card(page).getByTestId("watch-windows")).toHaveAttribute("data-ranked", "true");
    const rel = await page
      .locator('[data-testid="watch-card"] [data-window]')
      .evaluateAll((els) => els.map((e) => [e.getAttribute("data-window"), e.getAttribute("data-reliability")]));
    const level = (await card(page).getByTestId("watch-level").textContent()) ?? "";
    await ctx.close();
    return { rel, level };
  };
  const calm = await run(2);
  const storm = await run(6);
  expect(storm.rel, "K=6 与 K=2 的可靠度逐波段相同").toEqual(calm.rel);
  expect(calm.level).toContain("优秀"); // SFI 150、SSN 100、K 平静
  expect(storm.level).toContain("差"); // K ≥ 5：沿用现有等级判定
  expect(storm.level).not.toEqual(calm.level);
});

test("现在守哪一段：缺 K 时传播等级写「—」，不编等级", async ({ page }) => {
  await open(page, { cls: "C", solar: { k_index: null, solar_flux: 150, sunspots: 100 } });
  await expect(card(page).getByTestId("watch-level")).toContainText("—");
});

// ---------------- 灰线 ----------------

/** 独立写的晨昏圈距离：直射点（赤纬、经度）到本台的角距离 D，晨昏圈离它 90°。 */
function terminatorKm(lat: number, lon: number, doy: number, utcHour: number): number {
  const rad = (d: number) => (d * Math.PI) / 180;
  const decl = 23.44 * Math.sin((2 * Math.PI * (doy - 81)) / 365.25);
  const subLon = 15 * (12 - utcHour);
  const cosD =
    Math.sin(rad(lat)) * Math.sin(rad(decl)) + Math.cos(rad(lat)) * Math.cos(rad(decl)) * Math.cos(rad(lon - subLon));
  const d = (Math.acos(Math.max(-1, Math.min(1, cosD))) * 180) / Math.PI;
  return Math.abs(d - 90) * 111.19492664455873;
}

test("现在守哪一段：本台落在晨昏圈上——黄昏侧，距离与独立公式一致，标「贴近晨昏圈」", async ({ page }) => {
  // 春分日 12:00 UTC：直射点在经度 0，东经 91° 的 NJ50（中心 0.5°N 91°E）是当地 18:04，正在日落线上。
  await open(page, { cls: "C", grid: "NJ50", when: "2026-03-21T12:00:00Z" });
  const grey = card(page).getByTestId("watch-grey");
  await expect(grey).toContainText("黄昏侧");
  await expect(grey).toContainText("贴近晨昏圈");
  const km = Number(/离晨昏圈 (\d+) km/.exec((await grey.textContent()) ?? "")?.[1]);
  const want = terminatorKm(0.5, 91, 80, 12);
  expect(Math.abs(km - want), `页面 ${km} km，公式 ${want.toFixed(1)} km`).toBeLessThan(2);
});

test("现在守哪一段：本台落在晨昏圈上——黎明侧", async ({ page }) => {
  // 同一时刻，西经 89° 的 EJ50（中心 0.5°N 89°W）是当地 06:04，正在日出线上。
  await open(page, { cls: "C", grid: "EJ50", when: "2026-03-21T12:00:00Z" });
  const grey = card(page).getByTestId("watch-grey");
  await expect(grey).toContainText("黎明侧");
  await expect(grey).toContainText("贴近晨昏圈");
  const km = Number(/离晨昏圈 (\d+) km/.exec((await grey.textContent()) ?? "")?.[1]);
  expect(Math.abs(km - terminatorKm(0.5, -89, 80, 12))).toBeLessThan(2);
});

test("现在守哪一段：正午的本台离晨昏圈很远，不标贴近", async ({ page }) => {
  // IJ90 的中心约 0.5°N、1°W：春分日正午几乎在直射点下。
  await open(page, { cls: "C", grid: "IJ90", when: "2026-03-21T12:00:00Z" });
  const grey = card(page).getByTestId("watch-grey");
  await expect(grey).toContainText("离晨昏圈");
  await expect(grey).not.toContainText("贴近晨昏圈");
  const km = Number(/离晨昏圈 (\d+) km/.exec((await grey.textContent()) ?? "")?.[1]);
  expect(Math.abs(km - terminatorKm(0.5, -1, 80, 12))).toBeLessThan(2);
  expect(km).toBeGreaterThan(9000);
});

test("现在守哪一段：英文 / 西语界面都有译文，不留中文", async ({ page }) => {
  for (const code of ["en", "es"]) {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    await open(page, { cls: "C" });
    await expect(card(page)).not.toContainText(/[\u4e00-\u9fff]/);
  }
});

/** 整页与卡片各自的横向溢出（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: Page) {
  // 现代风点按钮会放一圈 560 ms 的按压涟漪（`.md-ping`），动画放大时短暂伸出视口，之后自清理。
  await expect(page.locator(".md-ping")).toHaveCount(0);
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="watch-card"]') as HTMLElement;
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      card: el.scrollWidth - el.clientWidth,
    };
  });
}

test("现在守哪一段：溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await open(page, { cls: "C" });
  await card(page).getByLabel("对方网格（可选）").fill("IO91");
  await expect(card(page).getByTestId("watch-windows")).toHaveAttribute("data-ranked", "true");
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.card).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-testid="watch-card"]')!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.card, "卡片里放 900px 宽的元素，量到的溢出应远大于 1").toBeGreaterThan(100);
  expect(after.page, "整页同理").toBeGreaterThan(100);
});

// 三语里最长的说明与每行的标记在 320 px 下不能把整页或卡片撑出横向滚动；两套皮肤各量一次。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    test(`现在守哪一段：320px 下不横向溢出（${lang} · ${style}）`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.addInitScript(
        ([code, ui]) => {
          localStorage.setItem("locale", code);
          localStorage.setItem("ui:uiStyle", ui);
          if (ui === "modern") localStorage.setItem("theme", "dark");
        },
        [lang, style],
      );
      await open(page, { cls: "C", grid: "NJ50", when: "2026-03-21T12:00:00Z" });
      await card(page).locator("input").nth(1).fill("IO91");
      await expect(card(page).getByTestId("watch-windows")).toHaveAttribute("data-ranked", "true");
      const over = await overflowOf(page);
      expect(over.page, "整页横向溢出").toBeLessThanOrEqual(1);
      expect(over.card, "守窗口卡片自身横向溢出").toBeLessThanOrEqual(1);
    });
  }
}

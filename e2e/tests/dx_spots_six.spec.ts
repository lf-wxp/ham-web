import { expect, stubApi, test, type Page } from "./fixtures";

/**
 * 6 m 开口（`/dx-spots`）：6 m 的 spot 显示从本台指向 DX 的方位与距离，并给出方向分布。
 *
 * 期望值由独立脚本按「Maidenhead 网格中心点 + haversine（R = 6371 km）」算出，不经过被测代码。
 * 本台 FN31（中心 41.5°N 73.0°W）：IO91 → 52° 5393 km；PM95 → 334° 10853 km；QF22 → 265° 16751 km；
 * FM18 → 227° 477 km；KG33 → 105° 12713 km。
 */

const SPOTS = [
  { spotter: "DL1AA", freq_khz: 50313, dx: "G4ABC", country: "England", comment: "IO91 FT8 -10", time: "1230Z" },
  { spotter: "DL1AA", freq_khz: 50313, dx: "JA1ABC", country: "Japan", comment: "PM95 CQ", time: "1231Z" },
  { spotter: "DL1AA", freq_khz: 50125, dx: "VK2XX", country: "Australia", comment: "QF22", time: "1232Z" },
  { spotter: "DL1AA", freq_khz: 50125, dx: "W3XX", country: "United States", comment: "FM18", time: "1233Z" },
  { spotter: "DL1AA", freq_khz: 50313, dx: "ZS6AA", country: "South Africa", comment: "KG33", time: "1234Z" },
  // 两个网格：「FN41<F2>EM63」是两端各一个，分不出哪个是 DX 台 —— 不猜，退回实体中心。
  { spotter: "N1MIW", freq_khz: 50313, dx: "KI4R", country: "United States", comment: "FN41<F2>EM63 FT8", time: "1235Z" },
  // RR73 是 FT8 的结束报文，不是南极的网格。
  { spotter: "DL1AA", freq_khz: 50313, dx: "OE1ABC", country: "Austria", comment: "FT8 -07 RR73", time: "1236Z" },
  // 20 m：不是这个功能的范围，不该有方位行。
  { spotter: "DL1AA", freq_khz: 14025, dx: "PY2AA", country: "Brazil", comment: "CW 599", time: "1237Z" },
];

async function open(page: Page, grid: string | null) {
  await page.addInitScript((g) => {
    if (g) {
      localStorage.setItem(
        "station-info",
        JSON.stringify({ callsign: "BG1AAA", operator: "", gridsquare: g, rig: "", antenna: "" }),
      );
    }
  }, grid);
  await stubApi(page, { "/api/spots": SPOTS });
  await page.goto("/dx-spots");
  await expect(page.locator('[data-slot="spot-row"]')).toHaveCount(SPOTS.length);
}

const rowOf = (page: Page, dx: string) =>
  page.locator('[data-slot="spot-row"]', { has: page.getByRole("button", { name: new RegExp(`^${dx} `) }) });
const headingOf = (page: Page, dx: string) => rowOf(page, dx).locator('[data-slot="spot-heading"]');

test("6 m 开口：备注里恰好一个网格就按它算方位与距离，并写明依据", async ({ page }) => {
  await open(page, "FN31");
  const expected: Record<string, string> = {
    G4ABC: "方位 52° NE · 5393 km",
    JA1ABC: "方位 334° NW · 10853 km",
    VK2XX: "方位 265° W · 16751 km",
    W3XX: "方位 227° SW · 477 km",
    ZS6AA: "方位 105° E · 12713 km",
  };
  for (const [dx, text] of Object.entries(expected)) {
    await expect(headingOf(page, dx), dx).toContainText(text);
    await expect(headingOf(page, dx), dx).toContainText("按备注里的网格");
  }
  await expect(headingOf(page, "G4ABC")).toContainText("IO91");
});

test("6 m 开口：两个网格有歧义、备注只有 RR73、没有网格——都退回实体中心并标「粗定位」", async ({ page }) => {
  await open(page, "FN31");
  for (const dx of ["KI4R", "OE1ABC"]) {
    const h = headingOf(page, dx);
    await expect(h, dx).toContainText("粗定位：按 DXCC 实体中心");
    await expect(h, dx).not.toContainText("按备注里的网格");
    await expect(h, dx).toContainText(/方位 \d+° (N|NE|E|SE|S|SW|W|NW) · \d+ km/);
  }
  // RR73 若被当成网格，会得到北极附近（83.5°N 175°E）的假方位，距离约 5693 km；
  // 按奥地利实体中心算是 6525 km 上下（独立脚本，实体中心取 47.3°N 13.3°E，留出实体表取点的差别）。
  const text = (await headingOf(page, "OE1ABC").textContent()) ?? "";
  expect(text).not.toContain("RR73");
  const km = Number(/· (\d+) km/.exec(text)?.[1]);
  expect(km, "应是奥地利的距离，而不是 RR73 当网格的 5693 km").toBeGreaterThan(6200);
  expect(km).toBeLessThan(6900);
  // 不是 6 m 的 spot 没有方位行。
  await expect(headingOf(page, "PY2AA")).toHaveCount(0);
});

test("6 m 开口：筛到 6 m 才出方向分布，数字与各行的方位字母一一对得上", async ({ page }) => {
  await open(page, "FN31");
  await expect(page.getByTestId("six-summary")).toHaveCount(0);
  await page.getByRole("button", { name: "6m", exact: true }).click();
  const summary = page.getByTestId("six-summary");
  await expect(summary).toBeVisible();
  await expect(summary).toContainText("6 m 方向分布");

  // 两条路径互相印证：汇总的每个方位数，必须等于各行方位字母里该字母出现的次数。
  const letters = await page.locator('[data-slot="spot-heading"]').evaluateAll((els) =>
    els.map((e) => /\d+° (N|NE|E|SE|S|SW|W|NW) ·/.exec(e.textContent ?? "")?.[1] ?? "?"),
  );
  expect(letters).toHaveLength(7);
  const tally: Record<string, number> = {};
  for (const l of letters) tally[l] = (tally[l] ?? 0) + 1;
  // 五条有网格的条目的方位是独立算出来的：NE / NW / W / SW / E 各至少 1。
  for (const l of ["NE", "NW", "W", "SW", "E"]) expect(tally[l], l).toBeGreaterThanOrEqual(1);
  for (const [l, n] of Object.entries(tally)) {
    await expect(summary.locator(`[data-sector="${l}"]`), l).toHaveText(`${l} ${n}`);
  }
  await expect(summary.locator("[data-sector]")).toHaveCount(Object.keys(tally).length);
  // 说明：粗定位、不是预报。
  await expect(summary).toContainText("不是突发 E 层预报");
});

test("6 m 开口：没填本台网格就没有方位，只提示去哪里填", async ({ page }) => {
  await open(page, null);
  await expect(page.locator('[data-slot="spot-heading"]')).toHaveCount(0);
  await page.getByRole("button", { name: "6m", exact: true }).click();
  await expect(page.getByTestId("six-summary")).toContainText("填上本台网格");
  await expect(page.getByTestId("six-summary").locator("[data-sector]")).toHaveCount(0);
});

test("6 m 开口：英文 / 西语界面都有译文，不留中文", async ({ page }) => {
  for (const [code, title] of [
    ["en", "6 m directions"],
    ["es", "Direcciones en 6 m"],
  ] as const) {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    await open(page, "FN31");
    await page.getByRole("button", { name: "6m", exact: true }).click();
    await expect(page.getByTestId("six-summary")).toContainText(title);
    await expect(page.getByTestId("six-summary")).not.toContainText(/[\u4e00-\u9fff]/);
    await expect(page.locator('[data-slot="spot-heading"]').first()).not.toContainText(/[\u4e00-\u9fff]/);
    for (const h of await page.locator('[data-slot="spot-heading"]').all()) {
      await expect(h).not.toContainText(/[\u4e00-\u9fff]/);
    }
  }
});

/** 整页与 6 m 汇总各自的横向溢出（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: Page) {
  // 现代风点筛选 chip 会放一圈 560 ms 的按压涟漪（`.md-ping`），动画放大时短暂伸出视口，之后自清理。
  await expect(page.locator(".md-ping")).toHaveCount(0);
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="six-summary"]') as HTMLElement;
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      summary: el.scrollWidth - el.clientWidth,
    };
  });
}

test("6 m 开口：溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await open(page, "FN31");
  await page.getByRole("button", { name: "6m", exact: true }).click();
  await expect(page.getByTestId("six-summary")).toBeVisible();
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.summary).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-testid="six-summary"]')!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.summary, "汇总里放 900px 宽的元素，量到的溢出应远大于 1").toBeGreaterThan(100);
  expect(after.page, "整页同理").toBeGreaterThan(100);
});

// 三语最长的说明在 320 px 下不能把整页或汇总撑出横向滚动；两套皮肤各量一次。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    test(`6 m 开口：320px 下不横向溢出（${lang} · ${style}）`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.addInitScript(
        ([code, ui]) => {
          localStorage.setItem("locale", code);
          localStorage.setItem("ui:uiStyle", ui);
          if (ui === "modern") localStorage.setItem("theme", "dark");
        },
        [lang, style],
      );
      await open(page, "FN31");
      await page.getByRole("button", { name: "6m", exact: true }).click();
      await expect(page.getByTestId("six-summary").locator("[data-sector]").first()).toBeVisible();
      const over = await overflowOf(page);
      expect(over.page, "整页横向溢出").toBeLessThanOrEqual(1);
      expect(over.summary, "6 m 汇总自身横向溢出").toBeLessThanOrEqual(1);
    });
  }
}

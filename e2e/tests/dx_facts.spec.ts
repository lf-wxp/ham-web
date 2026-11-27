import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, seedLog, stubApi, test, type Page } from "./fixtures";

/**
 * 这条 DX 值不值得叫（`/callsign` 的结果区）：方位与距离、这一小时的可靠度、路径与晨昏圈、
 * 日志里的新实体 / 新波段、已保存的 NEC 设计在该方位的增益——只摆事实，缺输入就留空，不合成总分。
 *
 * 期望值来源：可靠度逐波段对照服务端 `/api/voacap`（另一条代码路径），实体中心的网格用这里独立写的
 * Maidenhead 换算；NEC 增益用天线理论的相对关系（东西向水平偶极子：南北向是宽边、东西向是端射零点）；
 * 日志的新波段用这里独立列出的波段表求差集。
 */

const BANDS = ["160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m", "70cm"];
const block = (page: Page) => page.getByTestId("dx-facts");
const item = (page: Page, id: string) => block(page).getByTestId(id);

/** DXCC 实体中心（读仓库里的数据表，不经过被测代码）。 */
function entityCentre(prefix: string): [number, number] {
  const text = readFileSync(join(__dirname, "../../crates/core/data/dxcc.txt"), "utf8");
  for (const line of text.split("\n")) {
    if (line.startsWith("#")) continue;
    const f = line.split("|");
    if (f[1] === prefix) return [Number(f[6]), Number(f[7])];
  }
  throw new Error(`没有实体 ${prefix}`);
}

/** 独立写的 Maidenhead 六位网格。 */
function grid6(lat: number, lon: number): string {
  const L = "ABCDEFGHIJKLMNOPQR";
  const l = "ABCDEFGHIJKLMNOPQRSTUVWX";
  const x = lon + 180;
  const y = lat + 90;
  const f = [Math.floor(x / 20), Math.floor(y / 10)];
  const s = [Math.floor((x % 20) / 2), Math.floor(y % 10)];
  const u = [Math.floor(((x % 2) / 2) * 24), Math.floor((y % 1) * 24)];
  return `${L[f[0]]}${L[f[1]]}${s[0]}${s[1]}${l[u[0]]}${l[u[1]]}`;
}

type Solar = { k_index: number | null; solar_flux: number | null; sunspots: number | null };
const QUIET: Solar = { k_index: 2, solar_flux: 150, sunspots: 100 };

/** 一个东西向的半波偶极子（架高 10 m、理想大地、14.2 MHz）：宽边朝南北、端射方向在东西。 */
const dipoleBook = (name: string) =>
  JSON.stringify({
    designs: [
      {
        id: 1,
        name,
        saved_at: "2026-11-01",
        model: {
          wires: [{ a: [-5, 0, 10], b: [5, 0, 10], radius: 0.001, segments: 21 }],
          feeds: [{ wire: 0, at: 0.5, volts: 1 }],
          loads: [],
          freq_hz: 14200000,
          ground: { kind: "perfect" },
          conductivity: null,
          lines: [],
        },
      },
    ],
  });

async function open(
  page: Page,
  opts: {
    call?: string;
    grid?: string | null;
    solar?: Solar;
    when?: string;
    log?: { callsign: string; freq: string }[];
    designs?: boolean;
    /** 存档里那份设计的名字（用户自己起的，不属于界面文案）。 */
    designName?: string;
  } = {},
) {
  const {
    call = "JA1ABC",
    grid = "OM89",
    solar = QUIET,
    when = "2026-10-15T12:00:00Z",
    log = [],
    designs = false,
    designName = "东西向偶极子",
  } = opts;
  await page.addInitScript(
    ([g, d, book]) => {
      if (g) {
        localStorage.setItem(
          "station-info",
          JSON.stringify({ callsign: "BG4XYZ", operator: "", gridsquare: g, rig: "", antenna: "" }),
        );
      }
      if (d) localStorage.setItem("nec-designs", book as string);
    },
    [grid, designs, dipoleBook(designName)],
  );
  if (log.length) await seedLog(page, log);
  await stubApi(page, { "/api/solar": solar, "/api/callsign": { name: "", grid: "", qth: "", country: "", source: "DXCC" } });
  await page.clock.setFixedTime(new Date(when));
  await page.goto(`/callsign?call=${call}`);
  await expect(block(page)).toBeVisible();
}

test("这条 DX 值不值得叫：方位与距离与呼号页原有的两行是同一个数，量级符合地理", async ({ page }) => {
  await open(page);
  const text = (await item(page, "dxf-bearing").textContent()) ?? "";
  const m = /方位 (\d+)° · (\d+) km/.exec(text);
  expect(m, text).not.toBeNull();
  const [bearing, km] = [Number(m![1]), Number(m![2])];
  // 北京 → 日本：向东偏南、两三千公里。
  expect(bearing).toBeGreaterThan(70);
  expect(bearing).toBeLessThan(120);
  expect(km).toBeGreaterThan(1500);
  expect(km).toBeLessThan(3500);
  // 呼号页原有的「方位 / 距离」两行（同一份 insight）：数字一致。
  await expect(page.getByText(`${bearing}°`, { exact: true }).first()).toBeVisible();
  await expect(page.getByText(`${km} km`, { exact: true }).first()).toBeVisible();
  // 标明是粗定位。
  await expect(item(page, "dxf-bearing")).toContainText("粗定位");
});

test("这条 DX 值不值得叫：认不出实体就只说认不出，各项都不给", async ({ page }) => {
  await open(page, { call: "%3F%3F%3F" });
  await expect(item(page, "dxf-no-entity")).toContainText("认不出这个呼号所属的 DXCC 实体");
  for (const id of ["dxf-bearing", "dxf-reliability", "dxf-terminator", "dxf-standing", "dxf-nec"]) {
    await expect(item(page, id)).toHaveCount(0);
  }
});

test("这条 DX 值不值得叫：没有本台网格时方位、可靠度、晨昏圈都留空并说明，日志部分照给", async ({ page }) => {
  await open(page, { grid: null, log: [{ callsign: "W1AW", freq: "14.074" }] });
  for (const id of ["dxf-bearing", "dxf-reliability", "dxf-terminator"]) {
    await expect(item(page, id)).toContainText("先在台站信息里填网格");
  }
  await expect(item(page, "dxf-standing")).toContainText("新实体");
  await expect(item(page, "dxf-nec")).toBeVisible();
});

test("这条 DX 值不值得叫：没有太阳黑子数时只有可靠度留空，方位与晨昏圈照给", async ({ page }) => {
  await open(page, { solar: { k_index: 2, solar_flux: 150, sunspots: null } });
  await expect(item(page, "dxf-reliability")).toContainText("没有取到太阳黑子数");
  await expect(item(page, "dxf-reliability").locator("[data-band]")).toHaveCount(0);
  await expect(item(page, "dxf-bearing")).toContainText("方位");
  await expect(item(page, "dxf-terminator")).toContainText("晨昏圈");
});

test("这条 DX 值不值得叫：可靠度是该小时的 HF 十个波段，与服务端 /api/voacap 逐波段一致", async ({ page, request }) => {
  await open(page, { call: "JA1ABC", grid: "OM89" });
  const chips = item(page, "dxf-reliability").locator("[data-band]");
  await expect(chips).toHaveCount(10);
  const shown = await chips.evaluateAll((els) => els.map((e) => [e.getAttribute("data-band"), e.getAttribute("data-reliability")]));
  // 按频率从低到高，不排序、不挑「最好的」。
  expect(shown.map((s) => s[0])).toEqual(["160m", "80m", "60m", "40m", "30m", "20m", "17m", "15m", "12m", "10m"]);
  // 与页面完全独立的一条路径：服务端的同名接口。对方网格取实体中心所在的六位网格（独立换算）。
  const [lat, lon] = entityCentre("JA");
  const rx = grid6(lat, lon);
  const api = await request.get(`/api/voacap?tx=OM89&rx=${rx}&month=10&ssn=100&hour=12`);
  expect(api.ok()).toBeTruthy();
  const pred = (await api.json()) as { bands: { band: string; reliability: number }[] };
  for (const [band, pct] of shown) {
    const want = pred.bands.find((b) => b.band === band)!;
    expect(Number(pct), band!).toBe(Math.round(want.reliability * 100));
  }
  // 标出是哪一刻、哪个黑子数：不让数字悬在半空。
  await expect(item(page, "dxf-reliability")).toContainText("UTC 12:00 · 2026-10-15 · SSN 100");
});

test("这条 DX 值不值得叫：小时与黑子数确实起作用（不是写死的）", async ({ browser }) => {
  const run = async (solar: Solar, when: string) => {
    const ctx = await browser.newContext();
    const page = await ctx.newPage();
    await open(page, { call: "G4ABC", solar, when });
    const rel = Object.fromEntries(
      await item(page, "dxf-reliability")
        .locator("[data-band]")
        .evaluateAll((els) => els.map((e) => [e.getAttribute("data-band"), e.getAttribute("data-reliability")])),
    ) as Record<string, string>;
    await ctx.close();
    return rel;
  };
  const quietNight = await run({ k_index: 2, solar_flux: 70, sunspots: 0 }, "2026-10-15T02:00:00Z");
  expect(quietNight["10m"]).toBe("0");
  const busyNoon = await run({ k_index: 2, solar_flux: 200, sunspots: 200 }, "2026-10-15T12:00:00Z");
  expect(busyNoon).not.toEqual(quietNight);
});

test("这条 DX 值不值得叫：路径贴着晨昏圈的算「贴着」，只横穿一次的不算", async ({ page }) => {
  // 春分日 12:00 UTC：晨昏圈是东经 90° 的经线附近。本台 NJ50（0.5°N 91°E）→ 蒙古（46.8°N 102.2°E）几乎顺着它走。
  await open(page, { call: "JT1ABC", grid: "NJ50", when: "2026-03-21T12:00:00Z" });
  await expect(item(page, "dxf-terminator").locator("[data-along]")).toHaveAttribute("data-along", "true");
  await expect(item(page, "dxf-terminator")).toContainText("贴着晨昏圈");

  // 同一本台 → 日本（36.4°N 138.4°E）：向东北横穿晨昏圈，只有开头一小段贴近。
  await page.goto("/callsign?call=JA1ABC");
  await expect(item(page, "dxf-terminator").locator("[data-along]")).toHaveAttribute("data-along", "false");
  await expect(item(page, "dxf-terminator")).toContainText("没有贴着晨昏圈");
  const share = Number(/只有 (\d+)% 的点/.exec((await item(page, "dxf-terminator").textContent()) ?? "")?.[1]);
  expect(share).toBeGreaterThan(0);
  expect(share).toBeLessThan(50);
});

test("这条 DX 值不值得叫：日志里的新实体、新波段与联过的波段", async ({ page }) => {
  // 20 m 联过日本（另一个呼号）、40 m 联过 JA1ABC 本人、15 m 联过美国。
  await open(page, {
    log: [
      { callsign: "JA2XYZ", freq: "14.074" },
      { callsign: "JA1ABC", freq: "7.074" },
      { callsign: "W1AW", freq: "21.074" },
    ],
  });
  await expect(item(page, "dxf-standing")).toContainText("这个 DXCC 实体联过了");
  await expect(item(page, "dxf-worked-bands")).toContainText("这个呼号本人联过的波段：40m");
  // 新波段 = 联过该实体、但没在这个波段联过：除去 20 m（别的呼号）与 40 m（本人）。
  const expected = BANDS.filter((b) => !["20m", "40m"].includes(b)).join(" ");
  await expect(item(page, "dxf-new-bands")).toContainText(`这些波段还没联过该实体：${expected}`);

  // 没联过的实体：新实体，没有「新波段」之说。
  await page.goto("/callsign?call=VK2XX");
  await expect(item(page, "dxf-standing")).toContainText("新实体");
  await expect(item(page, "dxf-standing").locator('[data-new-entity="true"]')).toBeVisible();
  await expect(item(page, "dxf-new-bands")).toHaveCount(0);
});

test("这条 DX 值不值得叫：日志是空的时不判新实体 / 新波段", async ({ page }) => {
  await open(page);
  await expect(item(page, "dxf-standing")).toContainText("日志里还没有通联，无从判断新实体 / 新波段");
  await expect(item(page, "dxf-standing").locator("[data-new-entity]")).toHaveCount(0);
});

test("这条 DX 值不值得叫：没有已保存的 NEC 设计时只指路，不给增益", async ({ page }) => {
  await open(page);
  await expect(item(page, "dxf-nec")).toContainText("没有已保存的 NEC 设计");
  await expect(item(page, "dxf-nec").getByRole("link", { name: "去 NEC 页建一个" })).toHaveAttribute("href", "/nec");
  await expect(item(page, "dxf-nec").getByRole("button")).toHaveCount(0);
});

test("这条 DX 值不值得叫：已保存的 NEC 设计在该方位的增益——东西向偶极子朝南北是宽边、朝东西是零点", async ({ page }) => {
  await open(page, { call: "VK2XX", designs: true }); // 北京 → 澳大利亚：偏南，接近宽边
  const compute = item(page, "dxf-nec").getByRole("button", { name: "算该方位的增益" });
  await expect(item(page, "dxf-nec-result")).toHaveCount(0);
  await compute.click();
  const south = item(page, "dxf-nec-result");
  await expect(south).toContainText("东西向偶极子（14.200 MHz）朝");
  const southLoss = Number(await south.getAttribute("data-loss"));
  const southGain = Number(await south.getAttribute("data-gain"));
  expect(southGain).toBeGreaterThan(3); // 水平偶极子的宽边方向增益是正的、不小
  expect(southLoss).toBeLessThan(3); // 离宽边不远：比峰值只低一点

  // 换成日本（接近正东，端射方向）：增益明显更低。
  await page.goto("/callsign?call=JA1ABC");
  await item(page, "dxf-nec").getByRole("button", { name: "算该方位的增益" }).click();
  const east = item(page, "dxf-nec-result");
  await expect(east).toContainText("朝");
  const eastLoss = Number(await east.getAttribute("data-loss"));
  // 端射方向并非深零点：求解口径是「该方位全部仰角里的最大值」，高仰角仍有增益。
  // 按偶极子方向图 `cos(π/2·cosα)/sinα` 估算，仰角 20–30° 处比峰值低约 5–8 dB，所以只要求「明显更低」。
  expect(eastLoss).toBeGreaterThan(southLoss + 3);
  expect(Number(await east.getAttribute("data-gain"))).toBeLessThan(southGain - 3);
});

test("这条 DX 值不值得叫：换呼号后旧的增益结果不再留着", async ({ page }) => {
  await open(page, { call: "VK2XX", designs: true });
  await item(page, "dxf-nec").getByRole("button", { name: "算该方位的增益" }).click();
  await expect(item(page, "dxf-nec-result")).toBeVisible();
  await page.getByRole("textbox", { name: "呼号", exact: true }).fill("JA1ABC");
  await expect(item(page, "dxf-nec-result")).toHaveCount(0);
});

test("这条 DX 值不值得叫：改呼号时整块不被重建（不重拉太阳数据、不丢状态）", async ({ page }) => {
  let solarCalls = 0;
  await page.route("**/api/solar", async (route) => {
    solarCalls += 1;
    await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(QUIET) });
  });
  await open(page, { call: "VK2XX", designs: true });
  const before = solarCalls;
  // 给这块打个标记：若被重建，标记会随旧节点一起消失。
  await page.evaluate(() => document.querySelector('[data-testid="dx-facts"]')!.setAttribute("data-probe", "1"));
  const input = page.getByRole("textbox", { name: "呼号", exact: true });
  await input.fill("JA1AB");
  await input.fill("JA1ABC");
  await expect(item(page, "dxf-bearing")).toContainText("方位");
  await expect(page.locator('[data-testid="dx-facts"][data-probe="1"]')).toHaveCount(1);
  expect(solarCalls, "改呼号不该再去拉一次太阳数据").toBe(before);
});

test("这条 DX 值不值得叫：英文 / 西语界面都有译文，不留中文", async ({ page }) => {
  for (const code of ["en", "es"]) {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    await open(page, { log: [{ callsign: "JA2XYZ", freq: "14.074" }], designs: true, designName: "EW dipole" });
    await item(page, "dxf-nec").getByRole("button").click();
    await expect(item(page, "dxf-nec-result")).toBeVisible();
    await expect(block(page)).not.toContainText(/[\u4e00-\u9fff]/);
  }
});

/** 整页与块各自的横向溢出（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: Page) {
  // 现代风点按钮会放一圈 560 ms 的按压涟漪（`.md-ping`），动画放大时短暂伸出视口，之后自清理。
  await expect(page.locator(".md-ping")).toHaveCount(0);
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="dx-facts"]') as HTMLElement;
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      block: el.scrollWidth - el.clientWidth,
    };
  });
}

test("这条 DX 值不值得叫：溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await open(page, { log: [{ callsign: "JA2XYZ", freq: "14.074" }], designs: true });
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.block).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-testid="dx-facts"]')!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.block, "块里放 900px 宽的元素，量到的溢出应远大于 1").toBeGreaterThan(100);
  expect(after.page, "整页同理").toBeGreaterThan(100);
});

// 十个波段的可靠度、很长的新波段列表与说明，在 320 px 下不能把整页或块撑出横向滚动；三语 × 两套皮肤各量一次。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    test(`这条 DX 值不值得叫：320px 下不横向溢出（${lang} · ${style}）`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.addInitScript(
        ([code, ui]) => {
          localStorage.setItem("locale", code);
          localStorage.setItem("ui:uiStyle", ui);
          if (ui === "modern") localStorage.setItem("theme", "dark");
        },
        [lang, style],
      );
      await open(page, { log: [{ callsign: "JA2XYZ", freq: "14.074" }], designs: true });
      await item(page, "dxf-nec").getByRole("button").click();
      await expect(item(page, "dxf-nec-result")).toBeVisible();
      const over = await overflowOf(page);
      expect(over.page, "整页横向溢出").toBeLessThanOrEqual(1);
      expect(over.block, "DX 事实块自身横向溢出").toBeLessThanOrEqual(1);
    });
  }
}

import { expect, stubApi, test, type Page } from "./fixtures";

/**
 * 中继台的无线电地平线（`/repeater`）：选一台，填天线高度，地图上画点和一圈。
 *
 * 期望值全部与被测代码无关：半径用物理公式 d = √(2·K·R·h)（K = 4/3、R = 6371 km，另有业余口算
 * 4.12·√h 互相印证），地图坐标按等距圆柱投影的约定 `x = (lon+180)·2`、`y = (90−lat)·2` 反算回经纬度，
 * 再用 JS 里独立写的 haversine 量到圆心的距离。
 */

const REPEATERS = [
  { callsign: "BR1BJ", frequency_mhz: "145.6500", offset_mhz: "-0.6000", pl: "88.5", city: "Beijing", use: "OPEN", status: "On-air", lat: 39.9042, lon: 116.4074 },
  { callsign: "BR3SH", frequency_mhz: "439.4000", offset_mhz: "-5.0000", pl: "", city: "Shanghai", use: "OPEN", status: "On-air", lat: 31.2304, lon: 121.4737 },
  // 数据源缺坐标时服务端补 0 / 0：不能把它画到几内亚湾。
  { callsign: "BR0NOLL", frequency_mhz: "145.7000", offset_mhz: "-0.6000", pl: "", city: "Nowhere", use: "OPEN", status: "On-air", lat: 0, lon: 0 },
  // 靠近日期变更线：圆跨到 180° 之外。
  { callsign: "RU0FE", frequency_mhz: "145.5000", offset_mhz: "-0.6000", pl: "", city: "Anadyr", use: "OPEN", status: "On-air", lat: 64.7, lon: 177.5 },
];

const K = 4 / 3;
const R = 6371;
/** 标准大气下的无线电地平线（km）。 */
const horizonKm = (h: number) => Math.sqrt((2 * K * R * h) / 1000);

/** 两点大圆距离（km，haversine）。 */
function haversine(lat1: number, lon1: number, lat2: number, lon2: number) {
  const rad = (d: number) => (d * Math.PI) / 180;
  const a =
    Math.sin(rad(lat2 - lat1) / 2) ** 2 +
    Math.cos(rad(lat1)) * Math.cos(rad(lat2)) * Math.sin(rad(lon2 - lon1) / 2) ** 2;
  return 2 * R * Math.asin(Math.sqrt(a));
}

async function lookup(page: Page) {
  await stubApi(page, {
    "/api/repeaters": { country: "China", count: REPEATERS.length, results: REPEATERS },
  });
  await page.goto("/repeater");
  // 限定在内容区：顶栏的全局搜索按钮在西语下同名（Buscar）。
  await page.locator("#route-content").getByRole("button", { name: /^(查询|Look up|Buscar)$/ }).click();
  await expect(page.getByRole("cell", { name: "BR1BJ", exact: true })).toBeVisible();
}

const panel = (page: Page) => page.getByTestId("repeater-horizon");
/** 表格里那台中继的「画出」按钮（无障碍名里带呼号，三语都一样）。 */
const drawButton = (page: Page, callsign: string) => page.getByRole("button", { name: new RegExp(callsign) });

/** 地图上圆环各点按投影反算回的 `(lat, lon)`；经度保持连续（可能 > 180）。 */
async function ringLatLon(page: Page) {
  const pts = await panel(page).getByTestId("horizon-ring").first().getAttribute("points");
  return (pts ?? "")
    .trim()
    .split(/\s+/)
    .map((p) => p.split(",").map(Number))
    .map(([x, y]) => ({ lat: 90 - y / 2, lon: x / 2 - 180 }));
}

async function centreLatLon(page: Page) {
  const c = panel(page).getByTestId("horizon-centre").first();
  const x = Number(await c.getAttribute("cx"));
  const y = Number(await c.getAttribute("cy"));
  return { lat: 90 - y / 2, lon: x / 2 - 180 };
}

const heightInput = (page: Page) => panel(page).getByRole("textbox", { name: /天线离地高度|Antenna height|Altura de la antena/ });

test("中继台地平线：选一台后先画点，不编天线高度；填了才出圈，半径与物理公式一致", async ({ page }) => {
  await lookup(page);
  await expect(panel(page)).toHaveCount(0);

  await page.getByRole("button", { name: /画出 BR1BJ/ }).click();
  await expect(panel(page)).toContainText("BR1BJ 145.6500 MHz 的无线电地平线");
  // 高度默认留空：数据源没有这一项，页面不替用户编一个。
  await expect(heightInput(page)).toHaveValue("");
  await expect(page.getByTestId("horizon-status")).toContainText("填入这台中继的天线离地高度");
  await expect(panel(page).getByTestId("horizon-ring")).toHaveCount(0);

  // 中心点就在北京：投影反算回去应是 (39.9042, 116.4074)。
  const c0 = await centreLatLon(page);
  expect(c0.lat).toBeCloseTo(39.9042, 6);
  expect(c0.lon).toBeCloseTo(116.4074, 6);

  await heightInput(page).fill("30");
  // 22.6 km：√(2·(4/3)·6371·0.030) = 22.58；业余口算 4.12·√30 = 22.57，三者互相印证。
  expect(horizonKm(30)).toBeCloseTo(4.12 * Math.sqrt(30), 0);
  await expect(page.getByTestId("horizon-status")).toContainText("约 22.6 km");

  const ring = await ringLatLon(page);
  expect(ring).toHaveLength(72);
  for (const p of ring) {
    expect(haversine(c0.lat, c0.lon, p.lat, p.lon)).toBeCloseTo(horizonKm(30), 2);
  }
});

test("中继台地平线：换高度半径跟着变，换一台中继圆心跟着走而高度保留", async ({ page }) => {
  await lookup(page);
  await page.getByRole("button", { name: /画出 BR1BJ/ }).click();
  await heightInput(page).fill("30");
  await expect(page.getByTestId("horizon-status")).toContainText("约 22.6 km");

  await heightInput(page).fill("120");
  await expect(page.getByTestId("horizon-status")).toContainText("约 45.2 km");
  const c = await centreLatLon(page);
  for (const p of await ringLatLon(page)) {
    expect(haversine(c.lat, c.lon, p.lat, p.lon)).toBeCloseTo(horizonKm(120), 2);
  }

  // 换上海：标题、圆心换了；填过的高度还在，半径不变。
  await page.getByRole("button", { name: /画出 BR3SH/ }).click();
  await expect(panel(page)).toContainText("BR3SH 439.4000 MHz 的无线电地平线");
  await expect(heightInput(page)).toHaveValue("120");
  await expect(page.getByTestId("horizon-status")).toContainText("约 45.2 km");
  const sh = await centreLatLon(page);
  expect(sh.lat).toBeCloseTo(31.2304, 6);
  expect(sh.lon).toBeCloseTo(121.4737, 6);
  for (const p of await ringLatLon(page)) {
    expect(haversine(sh.lat, sh.lon, p.lat, p.lon)).toBeCloseTo(horizonKm(120), 2);
  }
  // 已选中的那一行标出来，上一台恢复成「画出」。
  await expect(page.getByRole("button", { name: /画出 BR3SH/ })).toContainText("已选中");
  await expect(page.getByRole("button", { name: /画出 BR1BJ/ })).toContainText("画出");
});

test("中继台地平线：缺坐标的中继不给按钮，高度不合法时提示并不画圈", async ({ page }) => {
  await lookup(page);
  const row = page.getByRole("row", { name: /BR0NOLL/ });
  await expect(row).toContainText("没有坐标");
  await expect(row.getByRole("button")).toHaveCount(0);

  await page.getByRole("button", { name: /画出 BR1BJ/ }).click();
  for (const bad of ["abc", "0", "-5", "9000.5", "3 m"]) {
    await heightInput(page).fill(bad);
    await expect(page.getByTestId("horizon-status"), `高度 ${bad}`).toContainText("大于 0、不超过 9000");
    await expect(heightInput(page)).toHaveAttribute("aria-invalid", "true");
    await expect(panel(page).getByTestId("horizon-ring")).toHaveCount(0);
  }
  // 改回合法值：圈回来、错误态消失。
  await heightInput(page).fill("30");
  await expect(panel(page).getByTestId("horizon-ring").first()).toBeAttached();
  await expect(heightInput(page)).not.toHaveAttribute("aria-invalid", "true");
});

test("中继台地平线：圆跨过日期变更线时经度连续、不拦腰拉线", async ({ page }) => {
  await lookup(page);
  await page.getByRole("button", { name: /画出 RU0FE/ }).click();
  await heightInput(page).fill("9000");
  // √(2·(4/3)·6371·9) = 391.0 km（口算 4.12·√9000 = 390.9）；纬度 64.7° 上每度经度约 47.6 km，向东跨出 180° 约 8°。
  await expect(page.getByTestId("horizon-status")).toContainText(`约 ${horizonKm(9000).toFixed(1)} km`);
  const ring = await ringLatLon(page);
  const maxLon = Math.max(...ring.map((p) => p.lon));
  expect(maxLon, "东侧应越过 180°（连续经度 > 180）").toBeGreaterThan(180);
  for (let i = 0; i < ring.length; i++) {
    const next = ring[(i + 1) % ring.length];
    expect(Math.abs(next.lon - ring[i].lon), `第 ${i} → ${i + 1} 点经度不该跳变`).toBeLessThan(2);
  }
  const c = await centreLatLon(page);
  for (const p of ring) expect(haversine(c.lat, c.lon, p.lat, p.lon)).toBeCloseTo(horizonKm(9000), 1);
});

test("中继台地平线：关闭收起面板，重新查询清掉选中", async ({ page }) => {
  await lookup(page);
  await page.getByRole("button", { name: /画出 BR1BJ/ }).click();
  await expect(panel(page)).toBeVisible();
  await panel(page).getByRole("button", { name: "关闭" }).click();
  await expect(panel(page)).toHaveCount(0);
  await expect(page.getByRole("button", { name: /画出 BR1BJ/ })).toContainText("画出");

  await page.getByRole("button", { name: /画出 BR1BJ/ }).click();
  await expect(panel(page)).toBeVisible();
  await page.locator("#route-content").getByRole("button", { name: "查询", exact: true }).click();
  await expect(panel(page)).toHaveCount(0);
});

/** 整页与面板各自的横向溢出（像素，≤ 1 视为没有溢出）。 */
async function overflowOf(page: Page) {
  // 现代风点按钮会放一圈 560 ms 的按压涟漪（`.md-ping`），动画放大时短暂伸出视口，之后自清理。
  await expect(page.locator(".md-ping")).toHaveCount(0);
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="repeater-horizon"]') as HTMLElement;
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      panel: el.scrollWidth - el.clientWidth,
    };
  });
}

test("中继台地平线：溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await lookup(page);
  await page.getByRole("button", { name: /画出 BR1BJ/ }).click();
  await heightInput(page).fill("30");
  const before = await overflowOf(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.panel).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-testid="repeater-horizon"]')!.appendChild(wide);
  });
  const after = await overflowOf(page);
  expect(after.panel, "面板里放 900px 宽的元素，量到的溢出应远大于 1").toBeGreaterThan(100);
  expect(after.page, "整页同理").toBeGreaterThan(100);
});

// 三语最长的说明与标题在 320 px 下不能把整页或面板撑出横向滚动；两套皮肤各量一次。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    test(`中继台地平线：320px 下不横向溢出（${lang} · ${style}）`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.addInitScript(
        ([code, ui]) => {
          localStorage.setItem("locale", code);
          localStorage.setItem("ui:uiStyle", ui);
          if (ui === "modern") localStorage.setItem("theme", "dark");
        },
        [lang, style],
      );
      await lookup(page);
      await drawButton(page, "BR1BJ").click();
      await expect(panel(page)).toBeVisible();
      await heightInput(page).fill("30");
      await expect(page.getByTestId("horizon-status")).toContainText("22.6");
      const over = await overflowOf(page);
      expect(over.page, "整页横向溢出").toBeLessThanOrEqual(1);
      expect(over.panel, "地平线面板自身横向溢出").toBeLessThanOrEqual(1);
    });
  }
}

test("中继台地平线：英文 / 西语界面都有译文，不留中文", async ({ page }) => {
  for (const code of ["en", "es"]) {
    await page.addInitScript((c) => localStorage.setItem("locale", c), code);
    await lookup(page);
    await drawButton(page, "BR1BJ").click();
    await expect(panel(page)).toBeVisible();
    await expect(panel(page)).not.toContainText(/[\u4e00-\u9fff]/);
    await expect(page.getByRole("row", { name: /BR0NOLL/ })).not.toContainText(/[\u4e00-\u9fff]/);
  }
});

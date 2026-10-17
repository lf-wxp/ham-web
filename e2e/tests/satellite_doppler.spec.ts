import type { Page } from "@playwright/test";
import { expect, stubApi, test } from "./fixtures";

// ---- 时间线：固定时钟 + 一次 10 分钟的 ISS 过境，过顶（径向速度为 0）在正中 ----
const NOW = new Date("2026-10-14T10:00:00+08:00");
const T0 = Math.floor(NOW.getTime() / 1000);
const AOS = T0 + 600;
const LOS = T0 + 1200;
const TCA = T0 + 900;
/** 请求轨迹的窗口：过境前后各多 300 s。 */
const TRACK_START = AOS - 300;
const TRACK_SECS = LOS - AOS + 600;

const C = 299_792.458;
/** 与 Rust 实现同一公式、同一运算顺序的独立实现（TS 一侧）。 */
const rxHz = (f: number, rr: number) => f / (1 + rr / C);
const txHz = (f: number, rr: number) => f / (1 - rr / C);
const roundTo10 = (hz: number) => Math.round(hz / 10) * 10;

/** 直线飞过头顶的解析模型：高度 400 km、7.6 km/s，t = TCA 时径向速度为 0。 */
function model(t: number) {
  const x = 7.6 * (t - TCA);
  const range = Math.sqrt(400 * 400 + x * x);
  return { range, rr: (7.6 * x) / range };
}

/** 仰角：过顶 60°，每偏离 1 s 降 0.2°，所以 AOS / LOS（偏离 300 s）时恰为 0°，再往外在地平线下。 */
const elevation = (t: number) => 60 - 0.2 * Math.abs(t - TCA);

function trackSamples(start: number, secs: number, step: number) {
  const out = [];
  for (let o = 0; o <= secs; o += step) {
    const t = start + o;
    const { range, rr } = model(t);
    out.push({ t, az: 200, el: elevation(t), range_km: range, range_rate_km_s: rr });
  }
  return out;
}

const PASSES = {
  passes: [
    // 已经结束的、更晚的同一颗星的过境，以及另一颗星：都不该被选中。
    { name: "ISS (ZARYA)", norad: 25544, aos: T0 - 2000, los: T0 - 1400, max_elev: 40, max_elev_time: T0 - 1700, azimuth: 100 },
    { name: "ISS (ZARYA)", norad: 25544, aos: AOS, los: LOS, max_elev: 60, max_elev_time: TCA, azimuth: 200 },
    { name: "SO-50", norad: 27607, aos: T0 + 100, los: T0 + 700, max_elev: 30, max_elev_time: T0 + 400, azimuth: 10 },
    { name: "ISS (ZARYA)", norad: 25544, aos: T0 + 6000, los: T0 + 6600, max_elev: 20, max_elev_time: T0 + 6300, azimuth: 300 },
    // 线性转发器卫星：与 ISS 同一条过境时间线，只换 NORAD，线性转发器用例要加载它们的下一次过境。
    { name: "JAS-2 (FO-29)", norad: 24278, aos: AOS, los: LOS, max_elev: 60, max_elev_time: TCA, azimuth: 200 },
    { name: "OSCAR 7 (AO-7)", norad: 7530, aos: AOS, los: LOS, max_elev: 60, max_elev_time: TCA, azimuth: 200 },
  ],
};

type TrackAsk = { norad: string | null; lat: string | null; lon: string | null; start: string | null; secs: string | null; step: string | null };

async function stubDoppler(page: Page, asked: TrackAsk[] = []) {
  await page.clock.setFixedTime(NOW);
  await stubApi(page, {
    "/api/iss": { latitude: 0, longitude: 0, altitude: 420, velocity: 7.66, visibility: "daylight" },
    "/api/passes": PASSES,
    "/api/sat-track": (url: URL) => {
      const q = url.searchParams;
      asked.push({ norad: q.get("norad"), lat: q.get("lat"), lon: q.get("lon"), start: q.get("start"), secs: q.get("secs"), step: q.get("step") });
      const start = Number(q.get("start"));
      return { norad: Number(q.get("norad")), name: "ISS (ZARYA)", step: 5, samples: trackSamples(start, Number(q.get("secs")), Number(q.get("step"))) };
    },
  });
}

const at = (page: Page, unix: number) => page.clock.setFixedTime(new Date(unix * 1000));

function panel(page: Page) {
  return page.locator("section").filter({ has: page.getByRole("heading", { name: "多普勒修正与电台联动" }) });
}

// ---- 假电台（Web Serial 桩）：Kenwood 11 位 / Icom CI-V / 不回话 ----
type Kind = "kenwood" | "icom" | "silent";

async function installRig(page: Page, kind: Kind) {
  await page.addInitScript((k) => {
    const w = window as unknown as {
      __ascii: string[];
      __bytes: number[][];
      __setRigFreq: (digits: string) => void;
      __vfoB: () => number;
    };
    w.__ascii = [];
    w.__bytes = [];
    let freq = "00014074000"; // Kenwood 11 位
    let civHz = 14_074_000;
    // VFO B（上行）：Kenwood 的 FB / Icom 的 25 01 写到这里；VFO A 的 `freq` / `civHz` 绝不应被它们改动。
    let vfoB = 0;
    w.__vfoB = () => vfoB;
    const bcd = (hz: number) => {
      const out: number[] = [];
      let v = hz;
      for (let i = 0; i < 5; i++) {
        const p = v % 100;
        v = Math.floor(v / 100);
        out.push((Math.floor(p / 10) << 4) | (p % 10));
      }
      return out;
    };
    w.__setRigFreq = (digits) => {
      freq = digits;
      civHz = Number(digits);
    };
    const queue: Uint8Array[] = [];
    let wake: (() => void) | null = null;
    const push = (b: number[] | Uint8Array) => {
      queue.push(Uint8Array.from(b));
      wake?.();
    };
    const enc = new TextEncoder();
    const reader = {
      async read() {
        while (queue.length === 0) await new Promise<void>((r) => (wake = r));
        return { value: queue.shift(), done: false };
      },
      async cancel() {
        wake?.();
      },
      releaseLock() {},
    };
    const writer = {
      async write(b: Uint8Array) {
        const arr = Array.from(b);
        w.__bytes.push(arr);
        if (k === "silent") return;
        if (k === "kenwood") {
          const cmd = new TextDecoder().decode(b);
          w.__ascii.push(cmd);
          if (cmd === "FA;") push(enc.encode(`FA${freq};`));
          else if (cmd === "MD;") push(enc.encode("MD4;"));
          else {
            const m = /^FA(\d+);$/.exec(cmd);
            if (m) freq = m[1];
            const b = /^FB(\d+);$/.exec(cmd);
            if (b) vfoB = Number(b[1]);
          }
          return;
        }
        // Icom CI-V
        if (arr[0] === 0xfe && arr[1] === 0xfe && arr[2] === 0x94 && arr[3] === 0xe0) {
          const cmd = arr[4];
          if (cmd === 0x03) push([0xfe, 0xfe, 0xe0, 0x94, 0x03, ...bcd(civHz), 0xfd]);
          else if (cmd === 0x04) push([0xfe, 0xfe, 0xe0, 0x94, 0x04, 0x05, 0x01, 0xfd]);
          else if (cmd === 0x05) {
            civHz = arr.slice(5, 10).reduceRight((acc, byte) => acc * 100 + (byte >> 4) * 10 + (byte & 15), 0);
            push([0xfe, 0xfe, 0xe0, 0x94, 0xfb, 0xfd]); // 应答 OK
          } else if (cmd === 0x25 && arr[5] === 0x01) {
            vfoB = arr.slice(6, 11).reduceRight((acc, byte) => acc * 100 + (byte >> 4) * 10 + (byte & 15), 0);
            push([0xfe, 0xfe, 0xe0, 0x94, 0xfb, 0xfd]);
          }
        }
      },
      releaseLock() {},
    };
    const port = {
      async open() {},
      async close() {},
      readable: { getReader: () => reader },
      writable: { getWriter: () => writer },
    };
    Object.defineProperty(navigator, "serial", {
      configurable: true,
      value: { requestPort: async () => port },
    });
  }, kind);
}

const written = (page: Page) => page.evaluate(() => (window as unknown as { __ascii: string[] }).__ascii);
const writtenBytes = (page: Page) => page.evaluate(() => (window as unknown as { __bytes: number[][] }).__bytes);
const setRigFreq = (page: Page, digits: string) =>
  page.evaluate((d) => (window as unknown as { __setRigFreq: (d: string) => void }).__setRigFreq(d), digits);
const vfoB = (page: Page) => page.evaluate(() => (window as unknown as { __vfoB: () => number }).__vfoB());
/** 写「另一个 VFO」的命令（`FB` 后带数字）。 */
const uplinkCommands = (cmds: string[]) => cmds.filter((c) => /^FB\d+;$/.test(c));
const UPLINK = "同时把上行（发射）频率写进 VFO B";
/** 写频率的命令（`FA` 后带数字），不含查询用的 `FA;`。 */
const setCommands = (cmds: string[]) => cmds.filter((c) => /^FA\d+;$/.test(c));

const AUTO = "自动调谐：把下行修正频率写入电台";

test("卫星多普勒：选中下一次过境、倒计时、逐秒给出收发修正频率与地平线判断", async ({ page }) => {
  const asked: TrackAsk[] = [];
  await stubDoppler(page, asked);
  await page.goto("/satellites");
  const p = panel(page);

  // 还没加载：没有读数
  await expect(p.getByRole("status")).toHaveCount(0);
  await p.getByRole("button", { name: "加载下一次过境" }).click();

  // 选的是「正在或将要进行」的那次（不是已结束的、也不是更晚的），窗口前后各留 300 s
  await expect(p.getByText(/本地时间 .* 至 .*（最大仰角 60°）/)).toBeVisible();
  expect(asked).toHaveLength(1);
  expect(asked[0]).toEqual({ norad: "25544", lat: "39.9", lon: "116.4", start: String(TRACK_START), secs: String(TRACK_SECS), step: "5" });
  await expect(p.getByRole("status")).toContainText("距过境开始 10:00");
  // 还在轨迹窗口之外：没有读数行
  await expect(p.getByText(/^接收（下行）调到/)).toHaveCount(0);

  // 过顶前 100 s：径向速度 ≈ −6.73 km/s（靠近）→ 下行偏高、上行要发低一点
  await at(page, TCA - 100);
  const { rr } = model(TCA - 100);
  const rx = rxHz(437_800_000, rr);
  const tx = txHz(145_990_000, rr);
  await expect(p.getByText(`接收（下行）调到 ${(rx / 1e6).toFixed(6)} MHz`)).toBeVisible();
  await expect(p.getByText(`发射（上行）调到 ${(tx / 1e6).toFixed(6)} MHz`)).toBeVisible();
  await expect(p.getByText(`（比标称 +${Math.round(rx - 437_800_000)} Hz）`)).toBeVisible();
  await expect(p.getByText(`（比标称 ${Math.round(tx - 145_990_000)} Hz）`)).toBeVisible();
  expect(rx).toBeGreaterThan(437_800_000 + 9_000); // 量级：ISS 在 437.8 MHz 约 ±10 kHz
  await expect(p.getByRole("status")).toContainText("过境中");
  await expect(p.getByText(/方位 200°　仰角 40\.0°/)).toBeVisible();

  // 正好过顶：径向速度为 0，修正为 0
  await at(page, TCA);
  await expect(p.getByText("接收（下行）调到 437.800000 MHz（比标称 +0 Hz）")).toBeVisible();
  await expect(p.getByText("发射（上行）调到 145.990000 MHz（比标称 +0 Hz）")).toBeVisible();

  // 过顶后：远离 → 方向反过来
  await at(page, TCA + 100);
  const away = rxHz(437_800_000, model(TCA + 100).rr);
  expect(away).toBeLessThan(437_800_000);
  await expect(p.getByText(`接收（下行）调到 ${(away / 1e6).toFixed(6)} MHz`)).toBeVisible();

  // 过境结束但还在轨迹覆盖内：卫星在地平线下
  await at(page, LOS + 200);
  await expect(p.getByRole("status")).toContainText("这次过境已结束");
  await expect(p.getByText("卫星在地平线以下，不会向电台写入频率。")).toBeVisible();

  // 超出轨迹覆盖：不再有读数
  await at(page, LOS + 400);
  await expect(p.getByText(/^接收（下行）调到/)).toHaveCount(0);
});

test("卫星多普勒：换卫星带出预置频率；没有过境、取不到过境或轨迹时分别给出说明", async ({ page }) => {
  await stubDoppler(page);
  await page.goto("/satellites");
  const p = panel(page);

  // 预置：SO-50 的 NORAD 与上下行频率
  await p.getByRole("combobox", { name: "卫星" }).click();
  await page.getByRole("option", { name: "SO-50 · 27607" }).click();
  await expect(p.getByLabel("NORAD 编号")).toHaveValue("27607");
  await expect(p.getByLabel("下行频率（MHz）")).toHaveValue("436.795");
  await expect(p.getByLabel("上行频率（MHz）")).toHaveValue("145.85");

  // 手动改任何一个数：回到「自定义」，并且已加载的过境作废
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/最大仰角 30°/)).toBeVisible();
  await p.getByLabel("下行频率（MHz）").fill("436.8");
  await expect(p.getByRole("combobox", { name: "卫星" })).toContainText("自定义");
  await expect(p.getByText(/最大仰角 30°/)).toHaveCount(0);

  // 这颗星未来 24 小时没有过境
  await p.getByLabel("NORAD 编号").fill("43017");
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/未来 24 小时内没有这颗卫星的过境/)).toBeVisible();

  // 不是整数的 NORAD：不能点加载
  await p.getByLabel("NORAD 编号").fill("25544.5");
  await expect(p.getByRole("button", { name: "加载下一次过境" })).toBeDisabled();
});

test("卫星多普勒：过境接口或轨迹接口失败时降级，不留旧读数", async ({ page }) => {
  // 过境接口没桩 → 503
  await page.clock.setFixedTime(NOW);
  await stubApi(page, { "/api/iss": { latitude: 0, longitude: 0, altitude: 420, velocity: 7.66, visibility: "daylight" } });
  await page.goto("/satellites");
  const p = panel(page);
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByRole("alert").filter({ hasText: "过境数据暂不可用" })).toBeVisible();

  // 过境有、轨迹接口没桩 → 提示取不到轨迹
  const page2 = await page.context().newPage();
  await page2.clock.setFixedTime(NOW);
  await stubApi(page2, { "/api/iss": { latitude: 0, longitude: 0, altitude: 420, velocity: 7.66, visibility: "daylight" }, "/api/passes": PASSES });
  await page2.goto("/satellites");
  const p2 = panel(page2);
  await p2.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p2.getByRole("alert").filter({ hasText: "取不到这次过境的轨迹" })).toBeVisible();
  await expect(p2.getByRole("status")).toHaveCount(0);

  // 同一页先有成功读数，再让重新取轨迹失败，才能验证旧数据被撤下。
  await stubDoppler(page);
  await at(page, TCA);
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
  await page.route("**/api/sat-track?*", route => route.fulfill({ status: 503, body: "unavailable" }));
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByRole("alert").filter({ hasText: "取不到这次过境的轨迹" })).toBeVisible();
  await expect(p.getByText(/^接收（下行）调到/)).toHaveCount(0);
});

test("卫星多普勒：浏览器不支持 Web Serial 时只给提示，不出现连接按钮", async ({ page }) => {
  await stubDoppler(page);
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "serial", { configurable: true, value: undefined });
  });
  await page.goto("/satellites");
  const p = panel(page);
  await expect(p.getByText(/连接电台需要 Chromium 内核的桌面浏览器/)).toBeVisible();
  await expect(p.getByRole("button", { name: "连接电台" })).toHaveCount(0);
});

test("卫星多普勒 + Kenwood 电台：连接后只读；勾选才写；节流；手动改频即停；地平线下不写", async ({ page }) => {
  await stubDoppler(page);
  await installRig(page, "kenwood");
  await at(page, TCA - 100);
  await page.goto("/satellites");
  const p = panel(page);
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();

  // 1) 连接后只读：没勾选前，一条写频率的命令都不会出现
  await p.getByRole("button", { name: "连接电台" }).click();
  await expect(p.getByText("电台当前频率 14.074 MHz")).toBeVisible();
  const auto = p.getByRole("checkbox", { name: AUTO });
  await expect(auto).toHaveAttribute("aria-checked", "false");
  await expect.poll(async () => (await written(page)).filter((c) => c === "FA;").length).toBeGreaterThanOrEqual(3);
  expect(setCommands(await written(page))).toEqual([]);

  // 2) 勾选：照电台自己回的 11 位写一次下行修正频率（取整到 10 Hz）
  const first = roundTo10(rxHz(437_800_000, model(TCA - 100).rr));
  await auto.click();
  await expect.poll(async () => setCommands(await written(page))).toEqual([`FA${String(first).padStart(11, "0")};`]);
  await expect(p.getByText(`已写入 1 次，最近一次 ${(first / 1e6).toFixed(6).replace(/0+$/, "")} MHz`)).toBeVisible();
  await expect(p.getByText(`电台当前频率 ${(first / 1e6).toFixed(6).replace(/0+$/, "")} MHz`)).toBeVisible();

  // 3) 节流：1 s 后目标只变约 21 Hz（< 100 Hz），不再写；再过 10 s 变了约 240 Hz，写第二次
  await at(page, TCA - 99);
  await expect.poll(async () => (await written(page)).filter((c) => c === "FA;").length).toBeGreaterThanOrEqual(6);
  expect(setCommands(await written(page))).toHaveLength(1);
  await at(page, TCA - 90);
  const second = roundTo10(rxHz(437_800_000, model(TCA - 90).rr));
  expect(Math.abs(second - first)).toBeGreaterThanOrEqual(100);
  await expect.poll(async () => setCommands(await written(page)).length).toBe(2);
  expect(setCommands(await written(page))[1]).toBe(`FA${String(second).padStart(11, "0")};`);

  // 4) 有人转了旋钮：电台回读的频率连续几次偏离我们写的值 → 立刻停写，并提示
  await setRigFreq(page, "00007074000");
  await expect(p.getByRole("alert").filter({ hasText: "已暂停：检测到电台频率被手动改动" })).toBeVisible({ timeout: 15_000 });
  await expect(auto).toHaveAttribute("aria-checked", "false");
  const afterPause = setCommands(await written(page)).length;
  await at(page, TCA - 10); // 目标早已变了很多，若还在写，这里就会冒出新命令
  await page.waitForTimeout(2500);
  expect(setCommands(await written(page))).toHaveLength(afterPause); // 暂停后没有再写，也就没有「抢回去」

  // 5) 地平线下：就算勾选了也不写
  await at(page, LOS + 200);
  await expect(p.getByText("卫星在地平线以下，不会向电台写入频率。")).toBeVisible();
  await auto.click(); // 重新开启（同时清掉暂停提示）
  await expect(p.getByRole("alert").filter({ hasText: "已暂停" })).toHaveCount(0);
  await page.waitForTimeout(3000);
  expect(setCommands(await written(page))).toHaveLength(afterPause);

  // 6) 断开：自动调谐被关掉
  await p.getByRole("button", { name: "断开" }).click();
  await expect(p.getByRole("button", { name: "连接电台" })).toBeVisible();
  await expect(p.getByRole("checkbox", { name: AUTO })).toHaveCount(0);
});

test("卫星多普勒 + Icom 电台：写入的是命令 05 + 5 字节小端 BCD", async ({ page }) => {
  await stubDoppler(page);
  await installRig(page, "icom");
  await at(page, TCA - 100);
  await page.goto("/satellites");
  const p = panel(page);
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();

  await p.getByRole("combobox", { name: "CAT 协议" }).click();
  await page.getByRole("option", { name: "Icom CI-V" }).click();
  await p.getByRole("button", { name: "连接电台" }).click();
  await expect(p.getByText("电台当前频率 14.074 MHz")).toBeVisible();

  const hz = roundTo10(rxHz(437_800_000, model(TCA - 100).rr));
  const bcd = (v: number) => {
    const out: number[] = [];
    for (let i = 0; i < 5; i++) {
      const pair = v % 100;
      v = Math.floor(v / 100);
      out.push((Math.floor(pair / 10) << 4) | (pair % 10));
    }
    return out;
  };
  await p.getByRole("checkbox", { name: AUTO }).click();
  const expected = [0xfe, 0xfe, 0x94, 0xe0, 0x05, ...bcd(hz), 0xfd];
  await expect
    .poll(async () => (await writtenBytes(page)).filter((b) => b[4] === 0x05))
    .toEqual([expected]);
  // Icom 没有「位数」问题，不会出现等待位数的提示
  await expect(p.getByText(/还没读到电台回报的频率格式/)).toHaveCount(0);
});

test("卫星多普勒：没读到电台回报的频率位数之前，即使勾选了也不写", async ({ page }) => {
  await stubDoppler(page);
  await installRig(page, "silent"); // 电台不回任何话
  await at(page, TCA - 100);
  await page.goto("/satellites");
  const p = panel(page);
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
  await p.getByRole("button", { name: "连接电台" }).click();
  await expect(p.getByText("等待电台报告频率…")).toBeVisible();

  await p.getByRole("checkbox", { name: AUTO }).click();
  await expect(p.getByText(/还没读到电台回报的频率格式/)).toBeVisible();
  // 等几个轮询周期：串口里只有 FA; / MD; 查询，一个字节的设频命令都没有
  await expect.poll(async () => (await writtenBytes(page)).length).toBeGreaterThanOrEqual(6);
  const all = (await writtenBytes(page)).map((b) => String.fromCharCode(...b));
  expect(all.every((c) => c === "FA;" || c === "MD;")).toBe(true);
});

test("审查修复：动态多普勒中手动改频暂停，改参数后重新确认才写", async ({ page }) => {
  await stubDoppler(page);
  await installRig(page, "kenwood");
  await at(page, TCA - 10);
  await page.goto("/satellites");
  const p = panel(page);
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
  await p.getByRole("button", { name: "连接电台" }).click();
  await expect(p.getByText("电台当前频率 14.074 MHz")).toBeVisible();
  const auto = p.getByRole("checkbox", { name: AUTO });
  await auto.click();
  await expect.poll(async () => setCommands(await written(page)).length).toBe(1);
  await expect.poll(async () => (await written(page)).filter(c => c === "FA;").length).toBeGreaterThanOrEqual(3);
  const before = setCommands(await written(page)).length;
  await setRigFreq(page, "00007074000");
  await at(page, TCA - 5);
  await expect(auto).toHaveAttribute("aria-checked", "false", { timeout: 15_000 });
  expect(setCommands(await written(page))).toHaveLength(before);
  await auto.click();
  await expect.poll(async () => setCommands(await written(page)).length).toBeGreaterThan(before);
  await p.getByLabel("下行频率（MHz）").fill("437.9");
  await expect(auto).toHaveAttribute("aria-checked", "false");
  const after = setCommands(await written(page)).length;
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
  await expect.poll(async () => (await written(page)).filter(c => c === "FA;").length).toBeGreaterThanOrEqual(10);
  expect(setCommands(await written(page))).toHaveLength(after);
});

test("审查修复：服务器降级轨迹不进入电台自动调谐", async ({ page }) => {
  await stubDoppler(page);
  await at(page, TCA);
  await page.route("**/api/sat-track?*", route => {
    const q = new URL(route.request().url()).searchParams;
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({
      stale: true, samples: trackSamples(Number(q.get("start")), Number(q.get("secs")), Number(q.get("step"))),
    }) });
  });
  await page.goto("/satellites");
  const p = panel(page);
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByRole("alert").filter({ hasText: "取不到这次过境的轨迹" })).toBeVisible();
  await expect(p.getByText(/^接收（下行）调到/)).toHaveCount(0);
});

// ───────────────────────── 上行（发射）频率与线性转发器 ─────────────────────────
// 数值来自独立脚本（Python，不复用页面 / 内核任何逻辑）：
//   TCA-100 s：ṙ = −6.725378 km/s → 下行 FA 目标 437 809 820、上行 FB 目标 145 986 730（ISS 上行 145.990）。
//   TCA-90 s：上行目标 145 986 810，只比 TCA-100 s 高 80 Hz（< 100 Hz 的节流阈值），下行则变化约 240 Hz。
//   FO-29（反相，下行调谐点 435.850）：卫星系上行 145.950 → 地面 rx 435 859 780、tx 145 946 730；
//   调谐点 +1 kHz → 卫星系上行 −1 kHz：rx 435 860 780、tx 145 945 730。

/** 没有任何 PTT / 分频 / 切 VFO 的字节：Kenwood 的 TX; RX; FT; FR; 与 Icom 的 1C（PTT）、0F（分频）、07（切 VFO）。 */
function expectNoTransmitOrSplit(cmds: string[], bytes: number[][]) {
  expect(cmds.filter((c) => /^(TX|RX|FT|FR|ST|SP)/.test(c))).toEqual([]);
  expect(bytes.filter((b) => b[4] === 0x1c || b[4] === 0x0f || b[4] === 0x07)).toEqual([]);
}

async function connectedAt(page: Page, secs: number, kind: Kind = "kenwood") {
  await stubDoppler(page);
  await installRig(page, kind);
  await at(page, TCA - secs);
  await page.goto("/satellites");
  const p = panel(page);
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
  if (kind === "icom") {
    await p.getByRole("combobox", { name: "CAT 协议" }).click();
    await page.getByRole("option", { name: "Icom CI-V" }).click();
  }
  await p.getByRole("button", { name: "连接电台" }).click();
  await expect(p.getByText("电台当前频率 14.074 MHz")).toBeVisible();
  return p;
}

test("上行：默认关；必须先开下行自动调谐；写的是 FB，不动 VFO A，也没有任何发射 / 分频命令", async ({ page }) => {
  const p = await connectedAt(page, 100);
  const up = p.getByRole("checkbox", { name: UPLINK });
  const auto = p.getByRole("checkbox", { name: AUTO });

  // 1) 连接后：上行复选框存在、未勾选；下行没开时它是禁用的，点不了。
  await expect(up).toHaveAttribute("aria-checked", "false");
  await expect(up).toBeDisabled();
  // 风险说明必须看得见（分频、不发射命令、已知机型缺陷）。
  await expect(p.getByText(/只写 VFO B 的频率：不开分频、不切 VFO、没有任何发射命令/)).toBeVisible();
  await expect(p.getByText(/TS-590S/)).toBeVisible();
  await expect(p.getByText(/Icom IC-7600/)).toBeVisible();

  // 2) 开下行：只写 FA，一条 FB 都没有（上行是单独的开关）。
  await auto.click();
  await expect.poll(async () => setCommands(await written(page)).length).toBe(1);
  await expect(up).toBeEnabled();
  await expect(up).toHaveAttribute("aria-checked", "false");
  await expect.poll(async () => (await written(page)).filter((c) => c === "FA;").length).toBeGreaterThanOrEqual(3);
  expect(uplinkCommands(await written(page))).toEqual([]);
  expect(await vfoB(page)).toBe(0);

  // 3) 勾上行：照电台自己回的 11 位写一次 FB（ISS 上行 145.990 做多普勒修正，取整到 10 Hz）。
  const vfoABefore = await page.evaluate(() => (window as unknown as { __ascii: string[] }).__ascii.filter((c) => /^FA\d+;$/.test(c)).length);
  await up.click();
  await expect.poll(async () => uplinkCommands(await written(page))).toEqual(["FB00145986730;"]);
  expect(await vfoB(page)).toBe(145_986_730);
  await expect(p.getByText("上行已写入 1 次，最近一次 145.98673 MHz")).toBeVisible();
  // 写上行没有动下行：FA 的写入次数一次没多；电台的 VFO A 还是下行修正值，不是 145.98673。
  expect(setCommands(await written(page)).length).toBe(vfoABefore);
  await expect(p.getByText("电台当前频率 437.80982 MHz")).toBeVisible();

  // 4) 节流：TCA-90 s 时下行变了约 240 Hz（写第二次），上行只变了 80 Hz（< 100 Hz，不写）——两者的节流各算各的。
  await at(page, TCA - 90);
  await expect.poll(async () => setCommands(await written(page)).length).toBe(2);
  await expect.poll(async () => (await written(page)).filter((c) => c === "FA;").length).toBeGreaterThanOrEqual(8);
  expect(uplinkCommands(await written(page))).toEqual(["FB00145986730;"]);

  // 5) 再往后走，上行累计偏移超过 100 Hz 时才写第二次。
  await at(page, TCA - 60);
  await expect.poll(async () => uplinkCommands(await written(page)).length).toBe(2);

  // 6) 全程没有任何发射 / 分频 / 切 VFO 的命令。
  expectNoTransmitOrSplit(await written(page), await writtenBytes(page));
});

test("上行护栏：上行填错（离标称太远 / 不在卫星业务段 / 不在业余波段）时一个字节都不写，并说清原因", async ({ page }) => {
  const p = await connectedAt(page, 100);
  const up = p.getByRole("checkbox", { name: UPLINK });
  const auto = p.getByRole("checkbox", { name: AUTO });
  const arm = async () => {
    // 改参数会让联动停下、已加载的过境作废（既有行为）：每次改完，重新加载过境，再开下行、再明确开上行。
    await expect(auto).toHaveAttribute("aria-checked", "false");
    await p.getByRole("button", { name: "加载下一次过境" }).click();
    await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
    await auto.click();
    await up.click();
  };
  const uplink = p.getByLabel("上行频率（MHz）");

  // 1) ISS 的上行 145.990 敲成 145.099：落在卫星业务段内，只有「离标称太远」能抓到。
  //    标称取自预置表（NORAD 25544），不取输入框的值；偏离 891 kHz，远超 40 ppm 的多普勒上限（约 5.8 kHz）。
  await uplink.fill("145.099");
  await arm();
  await expect(p.getByRole("alert").filter({ hasText: "离这颗卫星的标称上行太远" })).toBeVisible();
  await page.waitForTimeout(2200);
  expect(uplinkCommands(await written(page))).toEqual([]);

  // 2) 146.990：在 2 m 业余波段内（144–148），但不在卫星业余业务段（144–146）。
  //    它离 ISS 标称也很远，所以先被「卫星业务段」挡下（顺序：波段 → 业务段 → 距离）。
  await uplink.fill("146.99");
  await arm();
  await expect(p.getByRole("alert").filter({ hasText: "不在卫星业余业务可用的频段内" })).toBeVisible();

  // 3) 14.599：根本不在业余波段（最先被挡）。
  await uplink.fill("14.599");
  await arm();
  await expect(p.getByRole("alert").filter({ hasText: "不在任何业余波段内" })).toBeVisible();

  // 三种填错都没有写出任何 FB，VFO B 始终是 0。
  await page.waitForTimeout(2200);
  expect(uplinkCommands(await written(page))).toEqual([]);
  expect(await vfoB(page)).toBe(0);
  expectNoTransmitOrSplit(await written(page), await writtenBytes(page));

  // 4) 改回正确的 145.99：护栏放行，自动恢复写入（不需要别的操作）。
  await uplink.fill("145.99");
  await arm();
  await expect.poll(async () => uplinkCommands(await written(page))).toEqual(["FB00145986730;"]);
  await expect(p.getByRole("alert").filter({ hasText: "没有写入" })).toHaveCount(0);
});

test("上行：下行因任何原因停下，上行同时停下；不会比下行活得更久", async ({ page }) => {
  const p = await connectedAt(page, 100);
  const up = p.getByRole("checkbox", { name: UPLINK });
  const auto = p.getByRole("checkbox", { name: AUTO });
  await auto.click();
  await up.click();
  await expect.poll(async () => uplinkCommands(await written(page)).length).toBe(1);

  // 有人转了旋钮：下行暂停 → 上行复选框也回到未勾选，且之后再不写 FB。
  await setRigFreq(page, "00007074000");
  await expect(p.getByRole("alert").filter({ hasText: "已暂停：检测到电台频率被手动改动" })).toBeVisible({ timeout: 15_000 });
  await expect(auto).toHaveAttribute("aria-checked", "false");
  await expect(up).toHaveAttribute("aria-checked", "false");
  await expect(up).toBeDisabled();
  const after = uplinkCommands(await written(page)).length;
  await at(page, TCA - 10);
  await page.waitForTimeout(2500);
  expect(uplinkCommands(await written(page))).toHaveLength(after);

  // 重新开下行：上行不会自动跟着打开（必须再次明确勾选）。
  await auto.click();
  await expect(up).toHaveAttribute("aria-checked", "false");
  await page.waitForTimeout(2500);
  expect(uplinkCommands(await written(page))).toHaveLength(after);

  // 再开上行，然后断开：全部关掉。
  await up.click();
  await expect.poll(async () => uplinkCommands(await written(page)).length).toBeGreaterThan(after);
  await p.getByRole("button", { name: "断开" }).click();
  await expect(p.getByRole("checkbox", { name: UPLINK })).toHaveCount(0);
});

test("上行 + Icom：写的是命令 25 01 + 5 字节小端 BCD，且不动 VFO A", async ({ page }) => {
  const p = await connectedAt(page, 100, "icom");
  await p.getByRole("checkbox", { name: AUTO }).click();
  await expect.poll(async () => (await writtenBytes(page)).filter((b) => b[4] === 0x05).length).toBe(1);
  await p.getByRole("checkbox", { name: UPLINK }).click();

  const bcd = (v: number) => {
    const out: number[] = [];
    for (let i = 0; i < 5; i++) {
      const pair = v % 100;
      v = Math.floor(v / 100);
      out.push((Math.floor(pair / 10) << 4) | (pair % 10));
    }
    return out;
  };
  const expected = [0xfe, 0xfe, 0x94, 0xe0, 0x25, 0x01, ...bcd(145_986_730), 0xfd];
  await expect.poll(async () => (await writtenBytes(page)).filter((b) => b[4] === 0x25)).toEqual([expected]);
  expect(await vfoB(page)).toBe(145_986_730);
  // 下行（命令 05）仍然只写了一次，写上行没有碰它。
  expect((await writtenBytes(page)).filter((b) => b[4] === 0x05)).toHaveLength(1);
  expectNoTransmitOrSplit(await written(page), await writtenBytes(page));
});

test("线性转发器：选转发器必须连带换卫星（NORAD），并作废已加载的过境，不会拿别的卫星的轨迹算多普勒", async ({ page }) => {
  const asked: TrackAsk[] = [];
  await stubDoppler(page, asked);
  await installRig(page, "kenwood");
  await at(page, TCA - 100);
  await page.goto("/satellites");
  const p = panel(page);
  const norad = p.getByLabel("NORAD 编号");

  // 先在固定频点模式加载 ISS 的过境，再切线性：旧过境作废，NORAD 变成所选转发器的。
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
  await p.getByRole("radio", { name: "线性转发器（上下行联动）" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toHaveCount(0);
  await expect(norad).toHaveValue("7530"); // 第一个转发器 AO-7 · A

  // 换成 FO-29：NORAD 变成 24278，请求的轨迹也是它的。
  await p.getByLabel("转发器", { exact: true }).click();
  await page.getByRole("option", { name: /FO-29/ }).click();
  await expect(norad).toHaveValue("24278");
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
  expect(asked.at(-1)?.norad).toBe("24278");

  // 再换 AO-7 · B：回到 7530，已加载的 FO-29 过境作废（不能拿它的轨迹给 AO-7 算）。
  await p.getByLabel("转发器", { exact: true }).click();
  await page.getByRole("option", { name: /AO-7 · B/ }).click();
  await expect(norad).toHaveValue("7530");
  await expect(p.getByText(/^接收（下行）调到/)).toHaveCount(0);
});

test("线性转发器 FO-29：调谐点一动上下行一起动（反相），收发各做多普勒修正，写 VFO A / B", async ({ page }) => {
  const p = await connectedAt(page, 100);
  await p.getByRole("radio", { name: "线性转发器（上下行联动）" }).click();
  await p.getByLabel("转发器", { exact: true }).click();
  await page.getByRole("option", { name: /FO-29/ }).click();

  // 通带与映射：上行 145.9–146.0、下行 435.8–435.9、反相；默认调谐点在通带中心。
  await expect(p.getByText(/通带：上行 145\.900000 – 146\.000000 MHz，下行 435\.800000 – 435\.900000 MHz（反相）/)).toBeVisible();
  await expect(p.getByText("下行 435.850000 MHz ↔ 上行 145.950000 MHz")).toBeVisible();

  // 加载过境（NORAD 已按 FO-29 带出），然后连接、开下行、开上行。
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await p.getByRole("checkbox", { name: AUTO }).click();
  await p.getByRole("checkbox", { name: UPLINK }).click();
  await expect.poll(async () => setCommands(await written(page))).toEqual(["FA00435859780;"]);
  await expect.poll(async () => uplinkCommands(await written(page))).toEqual(["FB00145946730;"]);

  // 调谐点 +1 kHz：下行 +1 kHz，卫星系上行 −1 kHz（反相），两端都重写。
  await p.getByRole("button", { name: "+1 kHz" }).click();
  await expect(p.getByText("下行 435.851000 MHz ↔ 上行 145.949000 MHz")).toBeVisible();
  await expect.poll(async () => setCommands(await written(page)).at(-1)).toBe("FA00435860780;");
  await expect.poll(async () => uplinkCommands(await written(page)).at(-1)).toBe("FB00145945730;");
  expectNoTransmitOrSplit(await written(page), await writtenBytes(page));
});

test("线性转发器：调谐点夹在通带内；AO-7 B 的 432 MHz 上行不在卫星业余业务段，被护栏拒写并说明原因", async ({ page }) => {
  const p = await connectedAt(page, 100);
  await p.getByRole("radio", { name: "线性转发器（上下行联动）" }).click();
  await p.getByLabel("转发器", { exact: true }).click();
  await page.getByRole("option", { name: /FO-29/ }).click();

  // 步进推不出通带：连点 +1 kHz 到顶，停在 435.900000；手填越界的值被拉回通带。
  for (let i = 0; i < 70; i++) await p.getByRole("button", { name: "+1 kHz" }).click({ delay: 0 });
  await expect(p.getByText("下行 435.900000 MHz ↔ 上行 145.900000 MHz")).toBeVisible();
  await p.getByLabel("下行调谐点（MHz，卫星坐标系）").fill("500");
  await expect(p.getByText("下行 435.900000 MHz ↔ 上行 145.900000 MHz")).toBeVisible();
  await p.getByRole("button", { name: "回到通带中心" }).click();
  await expect(p.getByText("下行 435.850000 MHz ↔ 上行 145.950000 MHz")).toBeVisible();

  // 换成 AO-7 B：反相、上行 432.125–432.175。
  await p.getByLabel("转发器", { exact: true }).click();
  await page.getByRole("option", { name: /AO-7 · B/ }).click();
  await expect(p.getByText("下行 145.950000 MHz ↔ 上行 432.150000 MHz")).toBeVisible();
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await p.getByRole("checkbox", { name: AUTO }).click();
  await p.getByRole("checkbox", { name: UPLINK }).click();
  // 下行照常写；上行一个字节都不写，并说明是「业余波段内但不在卫星业余业务段」。
  await expect.poll(async () => setCommands(await written(page)).length).toBe(1);
  await expect(p.getByRole("alert").filter({ hasText: "不在卫星业余业务可用的频段内" })).toBeVisible();
  await page.waitForTimeout(2500);
  expect(uplinkCommands(await written(page))).toEqual([]);
  expect(await vfoB(page)).toBe(0);
});

test("线性转发器：非反相的 AO-7 A 上下行同向（下行 +1 kHz，上行也 +1 kHz）", async ({ page }) => {
  const p = await connectedAt(page, 100);
  await p.getByRole("radio", { name: "线性转发器（上下行联动）" }).click();
  await p.getByLabel("转发器", { exact: true }).click();
  await page.getByRole("option", { name: /AO-7 · A/ }).click();
  await expect(p.getByText(/（非反相）/)).toBeVisible();
  await expect(p.getByText("下行 29.450000 MHz ↔ 上行 145.900000 MHz")).toBeVisible();
  await p.getByRole("button", { name: "+1 kHz" }).click();
  // 非反相：下行 +1 kHz，上行也 +1 kHz。
  await expect(p.getByText("下行 29.451000 MHz ↔ 上行 145.901000 MHz")).toBeVisible();
});

test("线性转发器：手填调谐点保持原文，越界拉回并提示，外部改值同步文本", async ({ page }) => {
  await stubDoppler(page);
  await page.goto("/satellites");
  const p = panel(page);
  await p.getByRole("radio", { name: "线性转发器（上下行联动）" }).click();
  await p.getByLabel("转发器", { exact: true }).click();
  await page.getByRole("option", { name: /FO-29/ }).click();

  const dl = p.getByLabel("下行调谐点（MHz，卫星坐标系）");

  // 合法手填：输入框保持用户原文（不补尾零重排），显示区按 6 位小数做上下行映射。
  await dl.fill("435.851");
  await expect(dl).toHaveValue("435.851");
  await expect(p.getByText("下行 435.851000 MHz ↔ 上行 145.949000 MHz")).toBeVisible();
  await expect(p.getByRole("alert").filter({ hasText: "已拉回" })).toHaveCount(0);

  // 越界手填：调谐点被拉回通带上沿，但输入框保持用户原文，并提示「已拉回」。
  await dl.fill("500");
  await expect(dl).toHaveValue("500");
  await expect(p.getByText("下行 435.900000 MHz ↔ 上行 145.900000 MHz")).toBeVisible();
  await expect(p.getByRole("alert").filter({ hasText: "已拉回" })).toBeVisible();

  // 外部改值（回到中心）：输入框同步成夹紧后的值，提示清除。
  await p.getByRole("button", { name: "回到通带中心" }).click();
  await expect(dl).toHaveValue("435.850000");
  await expect(p.getByText("下行 435.850000 MHz ↔ 上行 145.950000 MHz")).toBeVisible();
  await expect(p.getByRole("alert").filter({ hasText: "已拉回" })).toHaveCount(0);
});

// ───────────────────────── 线性转发器：自定义通带 ─────────────────────────
// 独立脚本（TCA−100 s，ṙ = −6.725378 km/s）：自定义上行 145.100–145.200、下行 435.300–435.400（反相）。
//   中心：卫星系 下行 435.350 / 上行 145.150 → 地面 rx FA00435359770、tx FB00145146740；
//   调谐 +1 kHz（反相）：下行 435.351 / 上行 145.149 → rx FA00435360770、tx FB00145145740。
//   中心点上反相与非反相的上行都是 145.150，所以「反相开关是否生效」要在偏离中心的点上验证。

async function openCustom(page: Page) {
  const p = panel(page);
  await p.getByRole("radio", { name: "线性转发器（上下行联动）" }).click();
  await p.getByLabel("转发器", { exact: true }).click();
  await page.getByRole("option", { name: "自定义通带" }).click();
  return p;
}
const customFields = (p: ReturnType<typeof panel>) => ({
  ulLo: p.getByLabel("上行下限（MHz）"),
  ulHi: p.getByLabel("上行上限（MHz）"),
  dlLo: p.getByLabel("下行下限（MHz）"),
  dlHi: p.getByLabel("下行上限（MHz）"),
  inverting: p.getByRole("checkbox", { name: "反相（上行向高，下行向低）" }),
});

test("自定义通带：没填全不报错也不生效；填全且合法后按它映射；反相开关在偏离中心处真的改变上行", async ({ page }) => {
  await stubDoppler(page);
  await page.goto("/satellites");
  const p = await openCustom(page);
  const f = customFields(p);
  await expect(p.getByText(/预置里没有的转发器/)).toBeVisible();
  await expect(p.getByText(/自定义转发器没有 NORAD/)).toBeVisible();

  // 边填边不报错：只填了两个端点时没有任何告警；通带摘要不显示（旧的 AO-7 A 不再展示，自定义的还没生效）。
  await f.ulLo.fill("145.1");
  await f.ulHi.fill("145.2");
  await expect(p.getByRole("alert")).toHaveCount(0);
  await expect(p.getByText(/通带：上行/)).toHaveCount(0);

  // 填全：通带换成自定义的，调谐点落到下行中心 435.350。
  await f.dlLo.fill("435.3");
  await f.dlHi.fill("435.4");
  await expect(p.getByText("通带：上行 145.100000 – 145.200000 MHz，下行 435.300000 – 435.400000 MHz（反相）")).toBeVisible();
  await expect(p.getByText("下行 435.350000 MHz ↔ 上行 145.150000 MHz")).toBeVisible();

  // 偏离中心 +1 kHz：反相 → 上行 −1 kHz。
  await p.getByRole("button", { name: "+1 kHz" }).click();
  await expect(p.getByText("下行 435.351000 MHz ↔ 上行 145.149000 MHz")).toBeVisible();

  // 取消反相：同一个调谐点上行变成 +1 kHz（以通带下限为基准：145.100 + 0.051 = 145.151）。
  await f.inverting.click();
  await expect(p.getByText(/（非反相）/)).toBeVisible();
  await expect(p.getByText("下行 435.350000 MHz ↔ 上行 145.150000 MHz")).toBeVisible(); // 通带换了，调谐点回中心
  await p.getByRole("button", { name: "+1 kHz" }).click();
  await expect(p.getByText("下行 435.351000 MHz ↔ 上行 145.151000 MHz")).toBeVisible();
});

test("自定义通带：四种不合法的输入各给具体原因，且不改当前通带（不会悄悄打断正在进行的联动）", async ({ page }) => {
  await stubDoppler(page);
  await page.goto("/satellites");
  const p = await openCustom(page);
  const f = customFields(p);
  const fill = async (a: string, b: string, c: string, d: string) => {
    await f.ulLo.fill(a);
    await f.ulHi.fill(b);
    await f.dlLo.fill(c);
    await f.dlHi.fill(d);
  };
  // 先填一个合法的，确认生效。
  await fill("145.1", "145.2", "435.3", "435.4");
  await expect(p.getByText("下行 435.350000 MHz ↔ 上行 145.150000 MHz")).toBeVisible();
  const good = "通带：上行 145.100000 – 145.200000 MHz，下行 435.300000 – 435.400000 MHz（反相）";

  // 1) 下限不小于上限
  await fill("145.2", "145.1", "435.3", "435.4");
  await expect(p.getByRole("alert").filter({ hasText: "下限必须小于上限" })).toBeVisible();
  await expect(p.getByText(good)).toBeVisible(); // 仍是上一个合法通带
  // 2) 上下行宽度不等
  await fill("145.1", "145.2", "435.3", "435.35");
  await expect(p.getByRole("alert").filter({ hasText: "通带宽度必须相等" })).toBeVisible();
  await expect(p.getByText(good)).toBeVisible();
  // 3) 宽度不在 1 kHz–1 MHz：把 kHz 当成 MHz 填（宽度 100 MHz）
  await fill("145", "245", "435", "535");
  await expect(p.getByRole("alert").filter({ hasText: "1 kHz 到 1 MHz" })).toBeVisible();
  await expect(p.getByText(good)).toBeVisible();
  // 4) 填成负数或 0 的端点解析不出，当作没填：不报错、不生效（与「没填全」同一行为）
  await fill("0", "145.2", "435.3", "435.4");
  await expect(p.getByRole("alert")).toHaveCount(0);
  await expect(p.getByText(good)).toBeVisible();
  // 改回合法：告警消失。
  await fill("145.1", "145.2", "435.3", "435.4");
  await expect(p.getByRole("alert")).toHaveCount(0);
});

test("自定义通带 + 电台：收发都按自定义映射并各做多普勒修正，写 VFO A / B；上行仍过护栏", async ({ page }) => {
  await stubDoppler(page);
  await installRig(page, "kenwood");
  await at(page, TCA - 100);
  await page.goto("/satellites");
  const p = await openCustom(page);
  const f = customFields(p);
  // 自定义转发器没有 NORAD：自己在上方填 25544（ISS 的过境桩就是这条），请求的轨迹也确实是它。
  await p.getByLabel("NORAD 编号").fill("25544");
  await f.ulLo.fill("145.1");
  await f.ulHi.fill("145.2");
  await f.dlLo.fill("435.3");
  await f.dlHi.fill("435.4");
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
  await p.getByRole("button", { name: "连接电台" }).click();
  await expect(p.getByText("电台当前频率 14.074 MHz")).toBeVisible();
  await p.getByRole("checkbox", { name: AUTO }).click();
  await p.getByRole("checkbox", { name: UPLINK }).click();
  await expect.poll(async () => setCommands(await written(page))).toEqual(["FA00435359770;"]);
  await expect.poll(async () => uplinkCommands(await written(page))).toEqual(["FB00145146740;"]);
  expectNoTransmitOrSplit(await written(page), await writtenBytes(page));
});

test("自定义通带不绕过上行护栏：通带整体挪到卫星业务段之外时，上行一个字节都不写并说明原因", async ({ page }) => {
  await stubDoppler(page);
  await installRig(page, "kenwood");
  await at(page, TCA - 100);
  await page.goto("/satellites");
  const p = await openCustom(page);
  const f = customFields(p);
  await p.getByLabel("NORAD 编号").fill("25544");
  // 146.2–146.3：在 2 m 业余波段内（144–148），但在卫星业余业务段（144–146）之外。
  await f.ulLo.fill("146.2");
  await f.ulHi.fill("146.3");
  await f.dlLo.fill("435.3");
  await f.dlHi.fill("435.4");
  await expect(p.getByText("通带：上行 146.200000 – 146.300000 MHz")).toBeVisible();
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
  await p.getByRole("button", { name: "连接电台" }).click();
  await expect(p.getByText("电台当前频率 14.074 MHz")).toBeVisible();
  await p.getByRole("checkbox", { name: AUTO }).click();
  await p.getByRole("checkbox", { name: UPLINK }).click();
  // 下行照常写；上行被护栏拒写。
  await expect.poll(async () => setCommands(await written(page)).length).toBe(1);
  await expect(p.getByRole("alert").filter({ hasText: "不在卫星业余业务可用的频段内" })).toBeVisible();
  await page.waitForTimeout(2500);
  expect(uplinkCommands(await written(page))).toEqual([]);
  expect(await vfoB(page)).toBe(0);
});

test("自定义通带：从预置转发器切到「自定义」时，正在进行的联动立刻停止写电台，不会继续按看不见的旧通带写", async ({ page }) => {
  await stubDoppler(page);
  await installRig(page, "kenwood");
  await at(page, TCA - 100);
  await page.goto("/satellites");
  const p = panel(page);
  // 先用预置的 FO-29 联动起来：下行 + 上行都在写。
  await p.getByRole("radio", { name: "线性转发器（上下行联动）" }).click();
  await p.getByLabel("转发器", { exact: true }).click();
  await page.getByRole("option", { name: /FO-29/ }).click();
  await p.getByLabel("NORAD 编号").fill("25544"); // 过境桩是 ISS 这条
  await p.getByRole("button", { name: "加载下一次过境" }).click();
  await expect(p.getByText(/^接收（下行）调到/)).toBeVisible();
  await p.getByRole("button", { name: "连接电台" }).click();
  await expect(p.getByText("电台当前频率 14.074 MHz")).toBeVisible();
  await p.getByRole("checkbox", { name: AUTO }).click();
  await p.getByRole("checkbox", { name: UPLINK }).click();
  await expect.poll(async () => setCommands(await written(page)).length).toBe(1);
  await expect.poll(async () => uplinkCommands(await written(page)).length).toBe(1);

  // 切到「自定义」：通带摘要消失（不再展示 FO-29），目标变空，联动停止写电台。
  await p.getByLabel("转发器", { exact: true }).click();
  await page.getByRole("option", { name: "自定义通带" }).click();
  await expect(p.getByText(/通带：上行 145\.900000/)).toHaveCount(0);
  const fa = setCommands(await written(page)).length;
  const fb = uplinkCommands(await written(page)).length;
  // 让时间往前走，旧通带若还在生效，目标早就变了（> 100 Hz 的节流阈值）。
  await at(page, TCA - 60);
  await page.waitForTimeout(3000);
  expect(setCommands(await written(page))).toHaveLength(fa);
  expect(uplinkCommands(await written(page))).toHaveLength(fb);

  // 填出合法的自定义通带后恢复：按自定义映射继续写。
  const f = customFields(p);
  await f.ulLo.fill("145.1");
  await f.ulHi.fill("145.2");
  await f.dlLo.fill("435.3");
  await f.dlHi.fill("435.4");
  await expect(p.getByText("下行 435.350000 MHz ↔ 上行 145.150000 MHz")).toBeVisible();
  await expect.poll(async () => setCommands(await written(page)).length).toBeGreaterThan(fa);
});

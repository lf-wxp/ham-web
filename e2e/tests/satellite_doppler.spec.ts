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
    };
    w.__ascii = [];
    w.__bytes = [];
    let freq = "00014074000"; // Kenwood 11 位
    let civHz = 14_074_000;
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

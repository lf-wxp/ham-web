import type { Page } from "@playwright/test";
import { expect, stubApi, test } from "./fixtures";

// ───────────────────────── 天线旋转器（GS-232）与自动跟踪 ─────────────────────────
// 协议字节对 Hamlib 的 gs232a.c / gs232b.c 核对过：命令以 `\r` 结尾、`W<方位> <仰角>` 同时给两个角、`C2` 查询、`S` 停止。
// 数值来自独立脚本（Python，不复用页面 / 内核任何逻辑）。
//
// 时间线与 satellite_doppler.spec.ts 相同：固定时钟 + 一次 10 分钟的过境，过顶在正中，方位恒为 200°，
// 仰角 = 60 − 0.2·|t − TCA|（过境两端恰为 0°）。TCA − 100 s 时仰角 40°，每 8–10 s 抬升 2°（死区 2°）。

const NOW = new Date("2026-10-14T10:00:00+08:00");
const T0 = Math.floor(NOW.getTime() / 1000);
const AOS = T0 + 600;
const LOS = T0 + 1200;
const TCA = T0 + 900;
const elevation = (t: number) => 60 - 0.2 * Math.abs(t - TCA);

function trackSamples(start: number, secs: number, step: number) {
  const out = [];
  for (let o = 0; o <= secs; o += step) {
    const t = start + o;
    const x = 7.6 * (t - TCA);
    const range = Math.sqrt(400 * 400 + x * x);
    out.push({ t, az: 200, el: elevation(t), range_km: range, range_rate_km_s: (7.6 * x) / range });
  }
  return out;
}

async function stubPass(page: Page) {
  await page.clock.setFixedTime(NOW);
  await stubApi(page, {
    "/api/iss": { latitude: 0, longitude: 0, altitude: 420, velocity: 7.66, visibility: "daylight" },
    "/api/passes": {
      passes: [{ name: "ISS (ZARYA)", norad: 25544, aos: AOS, los: LOS, max_elev: 60, max_elev_time: TCA, azimuth: 200 }],
    },
    "/api/sat-track": (url: URL) => {
      const q = url.searchParams;
      return { norad: 25544, name: "ISS (ZARYA)", step: 5, samples: trackSamples(Number(q.get("start")), Number(q.get("secs")), Number(q.get("step"))) };
    },
  });
}

const at = (page: Page, unix: number) => page.clock.setFixedTime(new Date(unix * 1000));

/**
 * 假旋转器（Web Serial 桩）。`style`：GS-232A 回 `+0aaa+0eee`，GS-232B 回 `AZ=aaa EL=eee`，`azonly` 只回 `AZ=aaa`。
 * 收到 `W<aaa> <eee>\r` 后位置每秒向目标走 `speed` 度（`speed = 0` 表示卡住不动）；记录收到的全部原始字节。
 */
type Style = "a" | "b" | "azonly";
async function installRotor(page: Page, style: Style, speed = 3) {
  await page.addInitScript(
    ([st, spd]) => {
      const w = window as unknown as {
        __rot: number[][];
        __rotPos: () => [number, number];
        __rotSet: (az: number, el: number) => void;
        __rotSpeed: (v: number) => void;
      };
      w.__rot = [];
      let az = 100;
      let el = 0;
      let tgtAz = 100;
      let tgtEl = 0;
      let speed = spd as number;
      w.__rotPos = () => [az, el];
      w.__rotSet = (a, e) => {
        az = tgtAz = a;
        el = tgtEl = e;
      };
      w.__rotSpeed = (v) => {
        speed = v;
      };
      const step = (cur: number, tgt: number) => (Math.abs(tgt - cur) <= speed ? tgt : cur + Math.sign(tgt - cur) * speed);
      setInterval(() => {
        if (speed > 0) {
          az = step(az, tgtAz);
          el = step(el, tgtEl);
        }
      }, 1000);
      const queue: Uint8Array[] = [];
      let wake: (() => void) | null = null;
      const enc = new TextEncoder();
      const push = (s: string) => {
        queue.push(enc.encode(s));
        wake?.();
      };
      const p3 = (n: number) => String(n).padStart(3, "0");
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
          w.__rot.push(arr);
          const s = new TextDecoder().decode(b);
          if (s === "C2\r") {
            if (st === "a") push(`+0${p3(az)}+0${p3(el)}\r\n`);
            else if (st === "b") push(`AZ=${p3(az)} EL=${p3(el)}\r\n`);
            else push(`AZ=${p3(az)}\r\n`);
          }
          const m = /^W(\d{3}) (\d{3})\r$/.exec(s);
          if (m) {
            tgtAz = Number(m[1]);
            tgtEl = Number(m[2]);
          }
          if (s === "S\r") {
            tgtAz = az;
            tgtEl = el;
          }
        },
        async abort() {},
        releaseLock() {},
      };
      const port = {
        async open() {},
        async close() {},
        readable: { getReader: () => reader },
        writable: { getWriter: () => writer },
      };
      Object.defineProperty(navigator, "serial", { configurable: true, value: { requestPort: async () => port } });
    },
    [style, speed] as const,
  );
}

const sent = (page: Page) => page.evaluate(() => (window as unknown as { __rot: number[][] }).__rot.map((b) => String.fromCharCode(...b)));
/** 只要「转向」命令（`W…`），不含查询用的 `C2\r`。 */
const moves = (cmds: string[]) => cmds.filter((c) => c.startsWith("W"));
const rotorPos = (page: Page) => page.evaluate(() => (window as unknown as { __rotPos: () => [number, number] }).__rotPos());
const rotorSet = (page: Page, az: number, el: number) =>
  page.evaluate(([a, e]) => (window as unknown as { __rotSet: (a: number, e: number) => void }).__rotSet(a, e), [az, el] as const);
const rotorSpeed = (page: Page, v: number) =>
  page.evaluate((s) => (window as unknown as { __rotSpeed: (v: number) => void }).__rotSpeed(s), v);

const TRACK = "自动跟踪：让旋转器对准上方多普勒面板里的卫星";

function rotorPanel(page: Page) {
  return page.locator("section").filter({ has: page.getByRole("heading", { name: "天线旋转器" }) });
}
function dopplerPanel(page: Page) {
  return page.locator("section").filter({ has: page.getByRole("heading", { name: "多普勒修正与电台联动" }) });
}

async function connected(page: Page, style: Style, secsBeforeTca = 100, speed = 3) {
  await stubPass(page);
  await installRotor(page, style, speed);
  await at(page, TCA - secsBeforeTca);
  await page.goto("/satellites");
  await dopplerPanel(page).getByRole("button", { name: "加载下一次过境" }).click();
  await expect(dopplerPanel(page).getByText(/^接收（下行）调到/)).toBeVisible();
  const r = rotorPanel(page);
  await r.getByRole("button", { name: "连接旋转器" }).click();
  await expect(r.getByRole("button", { name: "断开" })).toBeVisible();
  return r;
}

test("旋转器：连接后只读（只发 C2 查询）；GS-232A 与 B 两种应答都能解析，只回方位的控制器也行", async ({ page }) => {
  for (const [style, text] of [
    ["a", "方位 100° · 仰角 0°"],
    ["b", "方位 100° · 仰角 0°"],
    ["azonly", "方位 100°"],
  ] as const) {
    const r = await connected(page, style);
    await expect(r.getByText(text, { exact: true })).toBeVisible();
    // 连接后：没有勾选前，一条转向命令都不会出现；查询是 `C2\r`（Hamlib 用的），不是旧的 `C` / `B`。
    await expect.poll(async () => (await sent(page)).filter((c) => c === "C2\r").length).toBeGreaterThanOrEqual(3);
    expect(moves(await sent(page))).toEqual([]);
    expect((await sent(page)).every((c) => c === "C2\r")).toBe(true);
    await expect(r.getByRole("checkbox", { name: TRACK })).toHaveAttribute("aria-checked", "false");
    await page.goto("about:blank");
  }
});

test("旋转器：手动转向写的是 `W<方位> <仰角>\\r`（两个角一起、带回车），方位回绕，保持当前仰角", async ({ page }) => {
  const r = await connected(page, "a");
  await expect(r.getByText("方位 100° · 仰角 0°", { exact: true })).toBeVisible();
  await rotorSet(page, 100, 40); // 把仰角摆到 40°，再看「只转方位」会不会保持它
  await expect(r.getByText("方位 100° · 仰角 40°", { exact: true })).toBeVisible({ timeout: 6000 });

  await r.getByLabel("目标方位（度）").fill("90");
  await r.getByRole("button", { name: "转向", exact: true }).click();
  await expect.poll(async () => moves(await sent(page))).toEqual(["W090 040\r"]);

  // 400° 回绕成 40°：写 `W040 040`，不是 `W400`，更不是 `W360`。
  await r.getByLabel("目标方位（度）").fill("400");
  await r.getByRole("button", { name: "转向", exact: true }).click();
  await expect.poll(async () => moves(await sent(page))).toEqual(["W090 040\r", "W040 040\r"]);

  // 负数：-10° → 350°。
  await r.getByLabel("目标方位（度）").fill("-10");
  await r.getByRole("button", { name: "转向", exact: true }).click();
  await expect.poll(async () => moves(await sent(page)).at(-1)).toBe("W350 040\r");

  // 不是数字：什么都不写。
  const n = moves(await sent(page)).length;
  await r.getByLabel("目标方位（度）").fill("abc");
  await r.getByRole("button", { name: "转向", exact: true }).click();
  await page.waitForTimeout(1500);
  expect(moves(await sent(page))).toHaveLength(n);
});

test("旋转器：按「停止」发 `S\\r`，并关掉自动跟踪", async ({ page }) => {
  const r = await connected(page, "a");
  await r.getByRole("checkbox", { name: TRACK }).click();
  await expect.poll(async () => moves(await sent(page)).length).toBeGreaterThanOrEqual(1);
  await r.getByRole("button", { name: "停止", exact: true }).click();
  await expect.poll(async () => (await sent(page)).includes("S\r")).toBe(true);
  await expect(r.getByRole("checkbox", { name: TRACK })).toHaveAttribute("aria-checked", "false");
  const after = moves(await sent(page)).length;
  await at(page, TCA - 30);
  await page.waitForTimeout(2500);
  expect(moves(await sent(page))).toHaveLength(after);
});

test("旋转器自动跟踪：默认关；勾选才转；写的是 `W200 040\\r`，抬升 2° 才再写（死区）；仰角限制在 0–90°", async ({ page }) => {
  const r = await connected(page, "a", 100, 60); // 转得快，免得「没在往目标走」干扰这条用例
  const track = r.getByRole("checkbox", { name: TRACK });
  await expect(track).toHaveAttribute("aria-checked", "false");
  // 风险说明必须看得见。
  await expect(r.getByText(/会驱动真实的机械运动/)).toBeVisible();
  await expect(r.getByText(/不知道你的旋转器是 360° 还是 450°/)).toBeVisible();

  // 没勾选前，即使卫星在地平线上，也一个转向命令都没有。
  await page.waitForTimeout(2500);
  expect(moves(await sent(page))).toEqual([]);

  // 勾选：TCA−100 s 时仰角 40°、方位 200° → `W200 040\r`（独立脚本）。
  await track.click();
  await expect.poll(async () => moves(await sent(page))).toEqual(["W200 040\r"]);
  await expect(r.getByText("已写入 1 次，最近一次 方位 200° · 仰角 40°")).toBeVisible();

  // 死区：再过 1 s 仰角只升 0.2°，不再写；到 TCA−92 s 升到 41.6°→取整 42°，差 2°，写第二次。
  await at(page, TCA - 99);
  await page.waitForTimeout(2200);
  expect(moves(await sent(page))).toHaveLength(1);
  await at(page, TCA - 92);
  await expect.poll(async () => moves(await sent(page))).toEqual(["W200 040\r", "W200 042\r"]);
});

test("旋转器自动跟踪：卫星在地平线下一个字节都不转，等到升起再转", async ({ page }) => {
  const r = await connected(page, "a", 100, 60);
  await at(page, LOS + 200); // 过境结束后：仰角为负
  await dopplerPanel(page).getByText("卫星在地平线以下，不会向电台写入频率。").waitFor();
  await r.getByRole("checkbox", { name: TRACK }).click();
  await expect(r.getByText(/等待卫星升到地平线以上/)).toBeVisible();
  await page.waitForTimeout(3000);
  expect(moves(await sent(page))).toEqual([]);
});

test("旋转器自动跟踪：断开、重新连接都会关掉它，重新勾选前不再转", async ({ page }) => {
  const r = await connected(page, "a", 100, 60);
  await r.getByRole("checkbox", { name: TRACK }).click();
  await expect.poll(async () => moves(await sent(page)).length).toBeGreaterThanOrEqual(1);
  await r.getByRole("button", { name: "断开" }).click();
  await expect(r.getByRole("checkbox", { name: TRACK })).toHaveCount(0);
  await r.getByRole("button", { name: "连接旋转器" }).click();
  await expect(r.getByRole("checkbox", { name: TRACK })).toHaveAttribute("aria-checked", "false");
  const n = moves(await sent(page)).length;
  await at(page, TCA - 40);
  await page.waitForTimeout(2500);
  expect(moves(await sent(page))).toHaveLength(n);
});

test("旋转器自动跟踪：转得慢（正常追目标）不会被误判；被人手动转走 / 卡住约 8 秒后立刻停止并提示", async ({ page }) => {
  // 慢速旋转器（3°/s）：写入 W200 040 后位置从 (100, 0) 慢慢走过去，离目标几十度。这是正常转动，不能报异常。
  const r = await connected(page, "a", 100, 3);
  await r.getByRole("checkbox", { name: TRACK }).click();
  await expect.poll(async () => moves(await sent(page)).length).toBeGreaterThanOrEqual(1);
  // 观察窗口必须长于误判的触发时间：判据数满 8 拍（约 8 秒）才报，这里等 12 秒。
  // （曾经只等 6 秒，比误判的触发时间还短，把判据改坏也看不出来。）
  const [az0] = await rotorPos(page);
  await page.waitForTimeout(12_000);
  const [az1] = await rotorPos(page);
  expect(az1 - az0).toBeGreaterThanOrEqual(24); // 3°/s × 12 s：旋转器确实一路在往目标走，离目标仍然很远
  expect(200 - az1).toBeGreaterThan(10); // 还没到位——这正是「离得远 ≠ 异常」要证明的情形
  await expect(r.getByRole("alert").filter({ hasText: "已停止自动跟踪" })).toHaveCount(0);
  await expect(r.getByRole("checkbox", { name: TRACK })).toHaveAttribute("aria-checked", "true");

  // 现在有人在控制器上按了停止：旋转器卡住不动（速度 0），离目标还很远。约 8 个读数后判定并停止。
  await rotorSpeed(page, 0);
  await expect(r.getByRole("alert").filter({ hasText: "已停止自动跟踪" })).toBeVisible({ timeout: 20_000 });
  await expect(r.getByRole("checkbox", { name: TRACK })).toHaveAttribute("aria-checked", "false");
  const n = moves(await sent(page)).length;
  await at(page, TCA - 20);
  await page.waitForTimeout(2500);
  expect(moves(await sent(page))).toHaveLength(n); // 停止之后不再「抢回去」

  // 重新勾选：提示清掉，再次从当前目标写起。
  await rotorSpeed(page, 60);
  await r.getByRole("checkbox", { name: TRACK }).click();
  await expect(r.getByRole("alert").filter({ hasText: "已停止自动跟踪" })).toHaveCount(0);
  await expect.poll(async () => moves(await sent(page)).length).toBeGreaterThan(n);
});

test("旋转器：全程不会发出 PTT / 分频 / 电台命令，只有 C2 / W / S", async ({ page }) => {
  const r = await connected(page, "a", 100, 60);
  await r.getByRole("checkbox", { name: TRACK }).click();
  await expect.poll(async () => moves(await sent(page)).length).toBeGreaterThanOrEqual(1);
  await r.getByLabel("目标方位（度）").fill("10");
  await r.getByRole("button", { name: "转向", exact: true }).click();
  await r.getByRole("button", { name: "停止", exact: true }).click();
  await page.waitForTimeout(1500);
  const all = await sent(page);
  expect(all.length).toBeGreaterThan(3);
  expect(all.every((c) => c === "C2\r" || c === "S\r" || /^W\d{3} \d{3}\r$/.test(c))).toBe(true);
  // 每一条都以回车结尾（没有回车多数控制器不会执行）。
  expect(all.every((c) => c.endsWith("\r"))).toBe(true);
});

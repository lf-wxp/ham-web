import { expect, test, type Page } from "./fixtures";

/**
 * 独立 JS 实现：AX.25 地址编码 + KISS 封装（KISS 载荷不带 FCS，由 TNC 的 HDLC 层追加）。
 * 与 crates/core/src/kiss.rs 分属两套实现；另有一条手算的字节序列（GOLDEN_HEX）
 * 同时钉住两者 —— 两套实现一致不等于正确，手算值才是外部事实。
 */
function ax25Address(call: string, last: boolean): number[] {
  // 与 Rust 侧一致：拆分 -SSID 后缀（SSID ≤ 15）。
  const [base, ssidText] = call.toUpperCase().split("-");
  const ssid = ssidText ? Number(ssidText) : 0;
  const out = Array.from({ length: 6 }, () => 0x20 << 1);
  base
    .split("")
    .forEach((c, i) => {
      out[i] = c.charCodeAt(0) << 1;
    });
  out.push(0x60 | (ssid << 1) | (last ? 1 : 0));
  return out;
}

function ax25UiFrame(dest: string, src: string, info: string): number[] {
  return [
    ...ax25Address(dest, false),
    ...ax25Address(src, true),
    0x03,
    0xf0,
    ...Array.from(info, (c) => c.charCodeAt(0)),
  ];
}

function kissWrap(payload: number[]): string {
  const out: number[] = [0xc0, 0x00];
  for (const b of payload) {
    if (b === 0xc0) out.push(0xdb, 0xdc);
    else if (b === 0xdb) out.push(0xdb, 0xdd);
    else out.push(b);
  }
  out.push(0xc0);
  return out.map((b) => b.toString(16).padStart(2, "0").toUpperCase()).join("");
}

// 手算：目的 "APRS  " = 82 A0 A4 A6 40 40 + SSID 字节 60；源 "BG1AAA-7" = 84 8E 62 82 82 82 +
// 6F（SSID 7、末地址 E 位）；控制 03、PID F0；信息 ">hello" = 3E 68 65 6C 6C 6F；外层 C0 00 … C0。
const GOLDEN_HEX = "C000" + "82A0A4A64040" + "60" + "848E62828282" + "6F" + "03F0" + "3E68656C6C6F" + "C0";

test("KISS TNC：离线帧预览与独立 JS 实现逐字节一致", async ({ page }) => {
  await page.goto("/kiss-tnc");
  await expect(page.getByRole("heading", { name: "KISS TNC" })).toBeVisible();

  await page.getByLabel("源呼号").fill("BG1AAA-7");
  await page.getByLabel("目的呼号").fill("APRS");
  await page.getByLabel("信息").fill(">hello");

  // 页面给出的 KISS hex 必须与独立实现、手算值三者完全一致。
  const expected = kissWrap(ax25UiFrame("APRS", "BG1AAA-7", ">hello"));
  expect(expected).toBe(GOLDEN_HEX);
  await expect(page.locator("code").filter({ hasText: expected })).toBeVisible();

  // 未连接时发送不可用（串口未打开）。
  await expect(page.getByRole("button", { name: "发送" })).toBeDisabled();
});

test("KISS TNC：非法呼号时不给帧预览", async ({ page }) => {
  await page.goto("/kiss-tnc");
  await page.getByLabel("源呼号").fill("BAD-CALL!");
  await expect(page.getByText(/帧预览/)).toHaveCount(0);
});

test("KISS TNC：串口参数与发送帧正确（stub navigator.serial）", async ({ page }) => {
  // Web Serial 在 CI 里没有真机：注入一个假端口，钉住三件事 ——
  // `open` 收到的是含 baudRate 的对象（不是被包成数组）、`write` 收到完整
  // KISS 帧、以及主动断开不该报「连接已丢失」。
  await page.addInitScript(() => {
    const state = {
      openOptions: null as { baudRate?: number } | null,
      written: [] as number[][],
      resolveRead: null as (() => void) | null,
      // 断开收尾的完成信号：反面断言要等它，不能靠 sleep。
      closed: 0,
    };
    (window as unknown as { __kiss: typeof state }).__kiss = state;
    const reader = {
      // 挂起等待数据；cancel（断开）时以 done 结束，与真实 reader 一致。
      read: () =>
        new Promise<{ done: boolean; value?: undefined }>((resolve) => {
          state.resolveRead = () => resolve({ done: true, value: undefined });
        }),
      cancel: () => {
        state.resolveRead?.();
        return Promise.resolve();
      },
      releaseLock: () => {},
    };
    const writer = {
      write: (data: Uint8Array) => {
        state.written.push(Array.from(data));
        return Promise.resolve();
      },
      releaseLock: () => {},
    };
    const port = {
      open: (options: { baudRate?: number }) => {
        state.openOptions = options;
        return Promise.resolve();
      },
      close: () => {
        state.closed += 1;
        return Promise.resolve();
      },
      readable: { getReader: () => reader },
      writable: { getWriter: () => writer },
    };
    Object.defineProperty(navigator, "serial", {
      configurable: true,
      value: { requestPort: () => Promise.resolve(port) },
    });
  });

  await page.goto("/kiss-tnc");
  // `SimpleSelect` 是自定义 combobox（不是原生 <select>）：点开再选。
  await page.getByRole("combobox", { name: "波特率", exact: true }).click();
  await page.getByRole("option", { name: "19200", exact: true }).click();
  await page.getByLabel("源呼号").fill("BG1AAA-7");
  await page.getByLabel("目的呼号").fill("APRS");
  await page.getByLabel("信息").fill(">hello");
  await page.getByRole("button", { name: "连接 TNC" }).click();

  const openBaud = () =>
    page.evaluate(
      () =>
        (
          window as unknown as {
            __kiss: { openOptions: { baudRate?: number } | null };
          }
        ).__kiss.openOptions?.baudRate,
    );
  await expect.poll(openBaud).toBe(19200);

  await page.getByRole("button", { name: "发送" }).click();
  const expected = (kissWrap(ax25UiFrame("APRS", "BG1AAA-7", ">hello")).match(/../g) ?? []).map((h) =>
    parseInt(h, 16),
  );
  const written = () =>
    page.evaluate(
      () => (window as unknown as { __kiss: { written: number[][] } }).__kiss.written[0] ?? [],
    );
  await expect.poll(written).toEqual(expected);

  await page.getByRole("button", { name: "断开" }).click();
  // 等收尾（close → cancel → read 以 done 结束）真的跑完再看反面断言：用 stub 的
  // close 计数轮询，而不是睡 300ms —— 慢机上那会让反面断言恒真（假绿）。
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as unknown as { __kiss: { closed: number } }).__kiss.closed,
      ),
    )
    .toBeGreaterThan(0);
  await expect(page.getByText("串口连接已断开")).toHaveCount(0);
});

// ---------------------------------------------------------------------------
// 收到的 APRS 位置上地图（S12）
// ---------------------------------------------------------------------------

/** 装一个能「喂数据」的假串口：`window.__kissFeed(hex)` 把一段 KISS 字节交给页面的读循环。 */
async function installFeedableSerial(page: Page) {
  await page.addInitScript(() => {
    const queue: Uint8Array[] = [];
    let wake: (() => void) | null = null;
    const reader = {
      read: () =>
        new Promise<{ done: boolean; value?: Uint8Array }>((resolve) => {
          const next = () => {
            const v = queue.shift();
            if (v) resolve({ done: false, value: v });
            else wake = next;
          };
          next();
        }),
      cancel: () => Promise.resolve(),
      releaseLock: () => {},
    };
    const port = {
      open: () => Promise.resolve(),
      close: () => Promise.resolve(),
      readable: { getReader: () => reader },
      writable: { getWriter: () => ({ write: () => Promise.resolve(), releaseLock: () => {} }) },
    };
    Object.defineProperty(navigator, "serial", {
      configurable: true,
      value: { requestPort: () => Promise.resolve(port) },
    });
    (window as unknown as { __kissFeed: (hex: string) => void }).__kissFeed = (hex: string) => {
      const bytes = new Uint8Array((hex.match(/../g) ?? []).map((h) => parseInt(h, 16)));
      queue.push(bytes);
      const w = wake;
      wake = null;
      w?.();
    };
  });
}

/** 喂一帧：独立 JS 实现编码，经页面的读循环、KISS 解码、AX.25 解析、位置分类一路走到地图。 */
async function feedFrame(page: Page, src: string, info: string) {
  const hex = kissWrap(ax25UiFrame("APRS", src, info));
  await page.evaluate((h) => (window as unknown as { __kissFeed: (x: string) => void }).__kissFeed(h), hex);
}

/** 等距圆柱投影的 SVG 坐标：`cx = (lon+180)·2`、`cy = (90−lat)·2`（与 `pages/map/projection.rs` 同一约定，这里独立写）。 */
const svgXY = (lat: number, lon: number) => ({ cx: (lon + 180) * 2, cy: (90 - lat) * 2 });

const dots = (page: Page, station: string) =>
  page.locator(`[data-testid="aprs-position-map"] circle[data-station="${station}"]`);
const stationRow = (page: Page, station: string) =>
  page.locator(`[data-station-row="${station}"]`);

test("KISS TNC：收到的位置按站上地图，压缩 / Mic-E / 坏字段只计数不画点", async ({ page }) => {
  await installFeedableSerial(page);
  await page.goto("/kiss-tnc");
  await expect(page.getByTestId("aprs-position-map")).toContainText("还没有收到带位置的报文");
  await page.getByRole("button", { name: "连接 TNC" }).click();
  await expect(page.getByRole("button", { name: "断开" })).toBeVisible();

  // 49°03.50′N 72°01.75′W 是 APRS 协议规范 1.0.1 示例报文里的坐标：49 + 3.5/60、−(72 + 1.75/60)。
  await feedFrame(page, "BG1AAA-7", "!4903.50N/07201.75W-Test 001234");
  await expect(stationRow(page, "BG1AAA-7")).toContainText("49.0583°N 72.0292°W");
  const first = svgXY(49 + 3.5 / 60, -(72 + 1.75 / 60));
  await expect(dots(page, "BG1AAA-7").first()).toBeAttached();
  const at = await dots(page, "BG1AAA-7").evaluateAll((els) =>
    els.map((e) => [Number(e.getAttribute("cx")), Number(e.getAttribute("cy"))]),
  );
  expect(at.length).toBeGreaterThan(0);
  for (const [cx, cy] of at) {
    expect(cx).toBeCloseTo(first.cx, 3);
    expect(cy).toBeCloseTo(first.cy, 3);
  }

  // 备用符号表 `\` 与南纬：`decode_position` 只认 `/`，页面必须能读。
  await feedFrame(page, "BG1BBB", "=3114.50S\\12128.50E#");
  await expect(stationRow(page, "BG1BBB")).toContainText("31.2417°S 121.4750°E");
  // 同一呼号的不同 SSID 是两个站。
  await feedFrame(page, "BG1AAA-9", "!3000.00N/12000.00E>");
  await expect(stationRow(page, "BG1AAA-9")).toContainText("30.0000°N 120.0000°E");
  // 带时间戳的类型。
  await feedFrame(page, "BG1TTT", "/092345z4903.50N/07201.75W>");
  await expect(stationRow(page, "BG1TTT")).toContainText("49.0583°N 72.0292°W");

  // 不该画点的四类：压缩（规范示例）、Mic-E（规范示例）、数值越界、没有位置的消息。
  await feedFrame(page, "BG1CCC", "=/5L!!<*e7>7P[");
  await feedFrame(page, "BG1DDD", "`(_fn\"Oj/");
  await feedFrame(page, "BG1EEE", "!9959.99N/07201.75W-");
  await feedFrame(page, "BG1FFF", ":BLN3     :hello");
  // 反面断言要等「它们已被处理」：计数是确定的信号，不靠 sleep。
  await expect(page.getByTestId("aprs-tally")).toHaveText(
    "已解出位置 4 帧 · 压缩位置 1 帧（未解码）· Mic-E 1 帧（未解码）· 格式不合 1 帧",
  );
  for (const none of ["BG1CCC", "BG1DDD", "BG1EEE", "BG1FFF"]) {
    await expect(dots(page, none), `${none} 不该有点`).toHaveCount(0);
    await expect(stationRow(page, none)).toHaveCount(0);
  }
  await expect(page.locator("[data-station-row]")).toHaveCount(4);

  // 同一个站再报一次：只留最新位置，旧点消失、报数 +1，台站数不变。
  await feedFrame(page, "BG1AAA-7", "!3114.50N/12128.50E-");
  await expect(stationRow(page, "BG1AAA-7")).toContainText("31.2417°N 121.4750°E");
  await expect(stationRow(page, "BG1AAA-7")).toContainText("2 帧");
  await expect(page.locator("[data-station-row]")).toHaveCount(4);
  const moved = svgXY(31 + 14.5 / 60, 121 + 28.5 / 60);
  const xs = await dots(page, "BG1AAA-7").evaluateAll((els) =>
    els.map((e) => [Number(e.getAttribute("cx")), Number(e.getAttribute("cy"))]),
  );
  expect(xs.length, "世界地图平铺的每一份里都有这个点").toBeGreaterThan(0);
  for (const [cx, cy] of xs) {
    expect(cx).toBeCloseTo(moved.cx, 3);
    expect(cy).toBeCloseTo(moved.cy, 3);
  }
});

test("KISS TNC：收帧时地图不被整个重建（缩放与平移要保住）", async ({ page }) => {
  await installFeedableSerial(page);
  await page.goto("/kiss-tnc");
  await page.getByRole("button", { name: "连接 TNC" }).click();
  await expect(page.getByRole("button", { name: "断开" })).toBeVisible();
  await feedFrame(page, "BG1AAA", "!4903.50N/07201.75W-");
  await expect(dots(page, "BG1AAA").first()).toBeVisible();
  // 给地图 <svg> 打个标记：若后面一帧让它整个重建，标记会随旧节点一起消失。
  await page.evaluate(() => {
    document.querySelector('[data-testid="aprs-position-map"] svg')?.setAttribute("data-probe", "1");
  });
  await feedFrame(page, "BG1BBB", "!3114.50N/12128.50E-");
  await expect(stationRow(page, "BG1BBB")).toBeVisible();
  await expect(page.locator('[data-testid="aprs-position-map"] svg[data-probe="1"]')).toHaveCount(1);
});

test("KISS TNC：只收到压缩位置时也显示计数而不是「还没收到」", async ({ page }) => {
  await installFeedableSerial(page);
  await page.goto("/kiss-tnc");
  await page.getByRole("button", { name: "连接 TNC" }).click();
  await feedFrame(page, "BG1CCC", "=/5L!!<*e7>7P[");
  await expect(page.getByTestId("aprs-tally")).toHaveText(
    "已解出位置 0 帧 · 压缩位置 1 帧（未解码）· Mic-E 0 帧（未解码）· 格式不合 0 帧",
  );
  await expect(page.locator("[data-station-row]")).toHaveCount(0);
});

/** 整页与位置地图卡片各自的横向溢出（像素，≤ 1 视为没有溢出），以及伸出视口的非 SVG 元素。 */
async function positionOverflow(page: Page) {
  // 量稳态：现代风点按钮会放一圈 560 ms 的按压涟漪（`.md-ping`），动画放大时短暂伸出视口，
  // 之后自清理（见 docs/ui/interaction.md）。并发跑时测量落在这 560 ms 里就会量到假溢出。
  await expect(page.locator(".md-ping")).toHaveCount(0);
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="aprs-position-map"]') as HTMLElement;
    const offenders = (Array.from(document.querySelectorAll("body *")) as HTMLElement[])
      .filter((e) => !(e instanceof SVGElement))
      .filter((e) => e.getBoundingClientRect().width > 0 && e.getBoundingClientRect().right > window.innerWidth + 1)
      .slice(0, 6)
      .map(
        (e) =>
          `${e.tagName.toLowerCase()}[${(e.getAttribute("class") ?? "").split(" ").slice(0, 3).join(".")}] right=${Math.round(e.getBoundingClientRect().right)}`,
      );
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      card: el.scrollWidth - el.clientWidth,
      offenders: offenders.join(" | "),
    };
  });
}

async function feedTwoStations(page: Page) {
  await page.getByRole("button", { name: /Connect TNC|连接 TNC|Conectar TNC/ }).click();
  await feedFrame(page, "BG1AAA-15", "!4903.50N/07201.75W-");
  await feedFrame(page, "BG1CCC", "=/5L!!<*e7>7P[");
  await expect(page.locator("[data-station-row]")).toHaveCount(1);
}

test("KISS TNC：位置地图溢出检测器自测——塞进 900px 宽的元素时必须量得出溢出", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await installFeedableSerial(page);
  await page.goto("/kiss-tnc");
  await feedTwoStations(page);
  // 先确认基线没溢出，再注入：否则「测得出」可能只是基线就是坏的。
  const before = await positionOverflow(page);
  expect(before.page).toBeLessThanOrEqual(1);
  expect(before.card).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const wide = document.createElement("div");
    wide.style.cssText = "width:900px;height:4px";
    document.querySelector('[data-testid="aprs-position-map"] ul')!.appendChild(wide);
  });
  const after = await positionOverflow(page);
  expect(after.card, "卡片里放 900px 宽的元素，量到的溢出应远大于 1").toBeGreaterThan(100);
  expect(after.page, "整页同理").toBeGreaterThan(100);
});

// 三语文案 × 两套皮肤 × 320 px：台站名带 SSID、坐标串与计数句都不能把整页撑出横向滚动。
for (const lang of ["zh", "en", "es"]) {
  for (const style of ["pixel", "modern"]) {
    test(`KISS TNC：位置地图 320px 下不横向溢出（${lang} · ${style}）`, async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 800 });
      await page.addInitScript(
        ([code, ui]) => {
          localStorage.setItem("locale", code);
          localStorage.setItem("ui:uiStyle", ui);
          if (ui === "modern") localStorage.setItem("theme", "dark");
        },
        [lang, style],
      );
      await installFeedableSerial(page);
      await page.goto("/kiss-tnc");
      await feedTwoStations(page);
      const over = await positionOverflow(page);
      expect(over.page, `整页横向溢出：${over.offenders}`).toBeLessThanOrEqual(1);
      expect(over.card, "位置地图卡片自身横向溢出").toBeLessThanOrEqual(1);
    });
  }
}

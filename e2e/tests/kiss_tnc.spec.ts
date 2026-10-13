import { expect, test } from "./fixtures";

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
  // `NativeSelect` 是自定义 combobox（不是原生 <select>）：点开再选。
  await page.getByLabel("波特率").click();
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

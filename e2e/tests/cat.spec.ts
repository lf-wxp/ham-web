import { expect, test } from "./fixtures";

test("电台 CAT：模拟 Kenwood 串口，回填频率与模式", async ({ page }) => {
  // 用桩替换 Web Serial：收到 FA; / MD; 查询后按 Kenwood 格式应答，并故意把应答切成两半
  await page.addInitScript(() => {
    const queue: Uint8Array[] = [];
    let wake: (() => void) | null = null;
    let freq = "00014074000";
    const enc = new TextEncoder();
    const push = (s: string) => {
      const b = enc.encode(s);
      queue.push(b.slice(0, 3), b.slice(3));
      wake?.();
    };
    const w = window as unknown as { __setRigFreq: (f: string) => void; __written: string[] };
    w.__written = [];
    w.__setRigFreq = (f) => {
      freq = f;
    };
    const port = {
      async open() {},
      async close() {},
      readable: {
        getReader: () => ({
          async read() {
            while (queue.length === 0) await new Promise<void>((r) => (wake = r));
            return { value: queue.shift(), done: false };
          },
          async cancel() {
            wake?.();
          },
          releaseLock() {},
        }),
      },
      writable: {
        getWriter: () => ({
          async write(b: Uint8Array) {
            const cmd = new TextDecoder().decode(b);
            w.__written.push(cmd);
            if (cmd === "FA;") push(`FA${freq};`);
            if (cmd === "MD;") push("MD3;");
          },
          releaseLock() {},
        }),
      },
    };
    Object.defineProperty(navigator, "serial", {
      value: { requestPort: async () => port },
      configurable: true,
    });
  });

  await page.goto("/log");
  await expect(page.getByRole("heading", { level: 1, name: "通联日志" })).toBeVisible();
  await page.getByRole("button", { name: "连接电台" }).first().click();

  await expect(page.getByText("已连接 · 14.074 MHz · CW").first()).toBeVisible();
  const freq = page.getByLabel("频率（MHz）").first();
  await expect(freq).toHaveValue("14.074");

  // 电台换频后表单跟着变
  await page.evaluate(() => (window as unknown as { __setRigFreq: (f: string) => void }).__setRigFreq("00007025500"));
  await expect(freq).toHaveValue("7.0255");

  await page.getByRole("button", { name: "断开" }).first().click();
  await expect(page.getByRole("button", { name: "连接电台" }).first()).toBeVisible();
});

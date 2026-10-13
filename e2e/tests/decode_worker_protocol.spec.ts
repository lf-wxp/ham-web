import { expect, test } from "./fixtures";

/**
 * 统一解码 Worker（`/decode-worker.js`）协议冒烟：不经页面 UI，直接对 Worker 发信封。
 *
 * 为什么单列：八个解码器都走 `CODECS` 表里的路径，路径写错（胶水 / wasm 文件名、目录）时
 * 只有对应页面的端到端用例才会红，而 SSTV / WSPR 目前没有专属 spec。这里对每个 codec 发一个
 * 空 WAV，断言回的是**协议错误码**（`ok:false` + 英文码），而不是 `unknown codec`、
 * NetworkError 或 Worker 自己崩掉 —— 能证明 8 条路径都加载、初始化并走进了 `decode`。
 *
 * 另外钉住 `docs/decode-worker-protocol.md` 里的约束：一个 Worker 只服务一个 codec。
 */

const CODECS = ["apt", "sstv", "wspr", "rtty", "olivia", "dominoex", "navtex", "lrpt"];

type Reply = { ok: boolean; error?: string; result?: unknown };

/** 在页面里起一个 Worker，依次发送若干信封，返回每条的回复。 */
async function converse(
  page: import("@playwright/test").Page,
  messages: Array<Record<string, unknown>>,
): Promise<Reply[]> {
  return page.evaluate(async (msgs) => {
    const worker = new Worker("/decode-worker.js");
    const replies: Reply[] = [];
    for (const m of msgs) {
      const reply = await new Promise<Reply>((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error("worker 回复超时")), 30000);
        worker.onmessage = (ev) => {
          clearTimeout(timer);
          resolve(ev.data as Reply);
        };
        worker.onerror = (ev) => {
          clearTimeout(timer);
          reject(new Error("worker error: " + ev.message));
        };
        // `bytes: "RAW"` 之外的情形原样传；空 WAV = 0 字节 ArrayBuffer。
        worker.postMessage({ ...m, bytes: m.bytes === undefined ? undefined : new ArrayBuffer(0) });
      });
      replies.push(reply);
    }
    worker.terminate();
    return replies;
  }, messages);
}

test("解码 Worker：八个 codec 的加载路径都有效，空输入回协议错误码", async ({ page }) => {
  await page.goto("/");
  for (const codec of CODECS) {
    const [reply] = await converse(page, [{ codec, bytes: true, params: {} }]);
    expect(reply.ok, codec).toBe(false);
    // 英文错误码（`码` 或 `码:参数`），而不是路径 404 / 未知 codec / wasm 崩溃文案。
    expect(reply.error, codec).toMatch(/^[a-z][a-z0-9-]*(:.*)?$/);
    expect(reply.error, codec).not.toMatch(/^unknown codec/);
    expect(reply.error, codec).not.toMatch(/NetworkError|Failed to execute|importScripts/i);
  }
});

test("解码 Worker：未知 codec、原型链键与缺 bytes 都回明确错误", async ({ page }) => {
  await page.goto("/");
  const replies = await converse(page, [{ codec: "nope", bytes: true }]);
  expect(replies[0]).toMatchObject({ ok: false });
  expect(replies[0].error).toMatch(/^unknown codec/);

  for (const codec of ["constructor", "__proto__", "toString"]) {
    const [reply] = await converse(page, [{ codec, bytes: true }]);
    expect(reply.ok, codec).toBe(false);
    expect(reply.error, codec).toMatch(/^unknown codec/);
  }

  const [missing] = await converse(page, [{ codec: "rtty" }]);
  expect(missing).toMatchObject({ ok: false, error: "bad-request:bytes" });
});

test("解码 Worker：同 codec 可复用，换 codec 回 worker-codec-mismatch", async ({ page }) => {
  await page.goto("/");
  const replies = await converse(page, [
    { codec: "rtty", bytes: true, params: {} },
    { codec: "rtty", bytes: true, params: {} },
    { codec: "olivia", bytes: true, params: {} },
  ]);
  // 第二条同 codec：不得因胶水重复声明而崩（旧实现会抛 `wasm_bindgen has already been declared`）。
  expect(replies[1].ok).toBe(false);
  expect(replies[1].error).not.toMatch(/already been declared/);
  expect(replies[2]).toMatchObject({ ok: false, error: "worker-codec-mismatch:rtty" });
});

test("解码 Worker：入口脚本加载失败时页面收到 worker-load-failed 而不是一直转圈", async ({
  page,
}) => {
  await page.route("**/decode-worker.js", (route) => route.abort());
  await page.goto("/rtty-decode");
  await expect(page.getByRole("heading", { name: "RTTY 解码器" })).toBeVisible();
  // 页面建 Worker 的时刻就会触发 `error` 事件；上传一个空 WAV 走一遍完整路径。
  await page.locator('input[type="file"]').setInputFiles({
    name: "empty.wav",
    mimeType: "audio/wav",
    buffer: Buffer.alloc(0),
  });
  await expect(page.getByText(/解码模块加载失败/)).toBeVisible({ timeout: 15000 });
});

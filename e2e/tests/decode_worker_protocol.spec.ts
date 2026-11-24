import { expect, test } from "./fixtures";

/**
 * 统一解码 Worker（`/decode-worker.js`）协议冒烟：不经页面 UI，直接对 Worker 发信封。
 *
 * 为什么单列：所有解码器都走 `CODECS` 表里的路径，路径写错（胶水 / wasm 文件名、目录）时
 * 只有对应页面的端到端用例才会红，而 SSTV / WSPR 目前没有专属 spec。这里对每个 codec 发一个
 * 空 WAV，断言回的是**协议错误码**（`ok:false` + 英文码），而不是 `unknown codec`、
 * NetworkError 或 Worker 自己崩掉 —— 能证明表里每条路径都加载、初始化并走进了 `decode`。
 *
 * 另外钉住 `docs/domain/decode-worker-protocol.md` 里的约束：一个 Worker 只服务一个 codec。
 */

// 与 `public/decode-worker.js` 的 `CODECS` 表、`docs/domain/decode-worker-protocol.md` 的
// 「已接入的 codec」表保持一一对应：加解码器时三处一起加。
const CODECS = ["apt", "sstv", "wspr", "rtty", "olivia", "dominoex", "mfsk", "thor", "navtex", "lrpt", "ft8", "hell", "wefax", "afsk1200"];

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

test("解码 Worker：全部 codec 的加载路径都有效，空输入回协议错误码", async ({ page }) => {
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

/** 编码回复的摘要：样本是几十万个浮点，不能整个经 `evaluate` 序列化回来。 */
type EncodeSummary = {
  ok: boolean;
  error?: string;
  kind?: string;
  length?: number;
  sampleRate?: number;
  peak?: number;
  finite?: boolean;
};

/** 对一个新 Worker 依次发编码信封（不带 bytes），返回每条回复的摘要。 */
async function converseEncode(
  page: import("@playwright/test").Page,
  messages: Array<Record<string, unknown>>,
): Promise<EncodeSummary[]> {
  return page.evaluate(async (msgs) => {
    const worker = new Worker("/decode-worker.js");
    const out: EncodeSummary[] = [];
    for (const m of msgs) {
      const reply = await new Promise<{ ok: boolean; error?: string; result?: any }>(
        (resolve, reject) => {
          const timer = setTimeout(() => reject(new Error("worker 回复超时")), 30000);
          worker.onmessage = (ev) => {
            clearTimeout(timer);
            resolve(ev.data);
          };
          worker.onerror = (ev) => {
            clearTimeout(timer);
            reject(new Error("worker error: " + ev.message));
          };
          // 图片的像素（几十万字节）不经 `evaluate` 序列化：在页面里按 `fill` 现场生成；
          // `rgbLength` 可以故意与尺寸对不上，`rgb: null` 表示缺字段。
          const pic = (m as { params?: { picture?: Record<string, unknown> } }).params?.picture;
          if (pic && typeof pic.fill === "number") {
            const { fill, rgbLength, ...rest } = pic as { fill: number; rgbLength?: number };
            const len = rgbLength ?? Number(rest.width) * Number(rest.height) * 3;
            (m as { params: { picture: unknown } }).params.picture = {
              ...rest,
              rgb: new Uint8Array(len).fill(fill),
            };
          }
          worker.postMessage(m);
        },
      );
      const samples = reply.result?.samples;
      out.push({
        ok: reply.ok,
        error: reply.error,
        kind: samples?.constructor?.name,
        length: samples?.length,
        sampleRate: reply.result?.sampleRate,
        peak: samples ? samples.reduce((a: number, v: number) => Math.max(a, Math.abs(v)), 0) : undefined,
        finite: samples ? samples.every((v: number) => Number.isFinite(v)) : undefined,
      });
    }
    worker.terminate();
    return out;
  }, messages);
}

const NAVTEX_PARAMS = {
  station: "F",
  category: "A",
  serial: 79,
  text: "GALE WARNING IN SOUTHERN ADRIATIC",
  phasingSecs: 2,
};

test("解码 Worker：编码请求——navtex 回基带样本，别的 codec 回 encode-unsupported", async ({
  page,
}) => {
  await page.goto("/");
  const [good] = await converseEncode(page, [{ codec: "navtex", op: "encode", params: NAVTEX_PARAMS }]);
  expect(good.ok).toBe(true);
  // 样本是 Float32Array、采样率 8000、幅度不越过 ±1、没有 NaN：写 WAV 的一端直接依赖这几条。
  expect(good.kind).toBe("Float32Array");
  expect(good.sampleRate).toBe(8000);
  // 相位段 2 秒 + 33 个字符的正文与报头，远多于 1 秒的样本。
  expect(good.length).toBeGreaterThan(8000 * 5);
  expect(good.peak).toBeGreaterThan(0.5);
  expect(good.peak).toBeLessThanOrEqual(1);
  expect(good.finite).toBe(true);

  // 没有 encode 导出的 codec：明确说不支持，而不是 `wasm_bindgen.encode is not a function`。
  for (const codec of ["rtty", "olivia", "wspr"]) {
    const [reply] = await converseEncode(page, [{ codec, op: "encode", params: {} }]);
    expect(reply, codec).toMatchObject({ ok: false, error: `encode-unsupported:${codec}` });
  }
});

// 三个 GPL 编码器的全部子模式与它们的标称采样率（fldigi 里的值）。名单与页面的
// `crates/app/src/codec_modes.rs` 一一对应（该文件的单测钉死条数 11 / 15 / 6）：页面名单里
// 有、核心表里没有的名字，Worker 会回 `unsupported-submode`，这里逐个送进去就会红。
const MFSK_RATES: Record<string, number> = {
  "16": 8000, "8": 8000, "4": 8000, "11": 11025, "22": 11025, "31": 8000,
  "32": 8000, "64": 8000, "128": 8000, "64L": 8000, "128L": 8000,
};
const THOR_RATES: Record<string, number> = {
  "16": 8000, "8": 8000, "4": 8000, "micro": 8000, "5": 11025, "11": 11025, "22": 11025,
  "25": 8000, "32": 8000, "44": 11025, "56": 16000, "25x4": 8000, "50x1": 8000, "50x2": 8000,
  "100": 8000,
};
const DOMINOEX_MODES = ["8", "16", "11", "4", "22", "5"];

test("解码 Worker：编码请求——MFSK / THOR / DominoEX 的全部子模式都能合成，采样率是各模式的标称值", async ({
  page,
}) => {
  await page.goto("/");
  const text = "CQ CQ DE BG1AAA";
  for (const [codec, rates] of [["mfsk", MFSK_RATES], ["thor", THOR_RATES]] as const) {
    const modes = Object.keys(rates);
    const replies = await converseEncode(
      page,
      modes.map((mode) => ({ codec, op: "encode", params: { mode, text } })),
    );
    modes.forEach((mode, i) => {
      const label = `${codec} ${mode}`;
      expect(replies[i].ok, `${label}: ${replies[i].error}`).toBe(true);
      expect(replies[i].kind, label).toBe("Float32Array");
      expect(replies[i].sampleRate, label).toBe(rates[mode]);
      expect(replies[i].length, label).toBeGreaterThan(rates[mode]);
      expect(replies[i].finite, label).toBe(true);
      expect(replies[i].peak, label).toBeLessThanOrEqual(1);
    });
  }

  // DominoEX：每个子模式、FEC 开 / 关；采样率恒为 8000。FEC 的码率是 1/2，同样的报文更长。
  const cases = DOMINOEX_MODES.flatMap((mode) => [false, true].map((fec) => ({ mode, fec })));
  const dominoex = await converseEncode(
    page,
    cases.map(({ mode, fec }) => ({ codec: "dominoex", op: "encode", params: { mode, text, fec } })),
  );
  cases.forEach(({ mode, fec }, i) => {
    expect(dominoex[i].ok, `dominoex ${mode} fec=${fec}: ${dominoex[i].error}`).toBe(true);
    expect(dominoex[i].sampleRate).toBe(8000);
    expect(dominoex[i].finite).toBe(true);
  });
  for (const mode of DOMINOEX_MODES) {
    const plain = dominoex[cases.findIndex((c) => c.mode === mode && !c.fec)];
    const fec = dominoex[cases.findIndex((c) => c.mode === mode && c.fec)];
    expect(fec.length, `dominoex ${mode}`).toBeGreaterThan(plain.length!);
  }

  // 缺省 `mode` 与解码侧一致（MFSK / THOR 是 16，DominoEX 是 8）：长度与显式写缺省值相同。
  for (const [codec, mode] of [["mfsk", "16"], ["thor", "16"], ["dominoex", "8"]] as const) {
    const [implicit, explicit] = await converseEncode(page, [
      { codec, op: "encode", params: { text } },
      { codec, op: "encode", params: { text, mode } },
    ]);
    expect(implicit.ok, codec).toBe(true);
    expect(implicit.length, codec).toBe(explicit.length);
  }
});

test("解码 Worker：MFSK / THOR / DominoEX 编码入参不合法时回解码侧已有的码", async ({ page }) => {
  await page.goto("/");
  const text = "CQ CQ DE BG1AAA";
  const cases: Array<[string, Record<string, unknown>, string]> = [
    ["未知子模式", { codec: "mfsk", params: { mode: "99", text } }, "unsupported-submode:99"],
    ["子模式不是字符串", { codec: "thor", params: { mode: 16, text } }, "unsupported-submode:"],
    ["DominoEX 未知子模式", { codec: "dominoex", params: { mode: "7", text } }, "unsupported-submode:7"],
    ["中心频率越界", { codec: "mfsk", params: { mode: "16", text, centerHz: 100000 } }, "center-hz-out-of-range:100000"],
    ["中心频率放不下 THOR 100", { codec: "thor", params: { mode: "100", text, centerHz: 3500 } }, "center-hz-out-of-range:3500"],
    ["中心频率不是数字", { codec: "thor", params: { mode: "16", text, centerHz: "abc" } }, "center-hz-out-of-range:NaN"],
    ["报文只有中文", { codec: "mfsk", params: { text: "你好，世界" } }, "encode-params:text"],
    ["报文为空", { codec: "thor", params: { text: "" } }, "encode-params:text"],
    ["缺报文", { codec: "dominoex", params: {} }, "encode-params:text"],
    ["报文过长", { codec: "dominoex", params: { text: "A".repeat(1025) } }, "encode-params:text"],
  ];
  for (const [name, message, error] of cases) {
    const [reply] = await converseEncode(page, [{ ...message, op: "encode" }]);
    expect(reply, name).toMatchObject({ ok: false, error });
  }

  // 给了合法的中心频率就照它发（不悄悄挪回缺省）：THOR 16 放在 2000 Hz 与缺省 1500 Hz 样本数相同，
  // 但这条要通过 —— 证明显式中心被接受；越界值上面已钉死会被拒绝。
  const [custom, standard] = await converseEncode(page, [
    { codec: "thor", op: "encode", params: { mode: "16", text, centerHz: 2000 } },
    { codec: "thor", op: "encode", params: { mode: "16", text } },
  ]);
  expect(custom.ok).toBe(true);
  expect(custom.length).toBe(standard.length);
});

// 图片发端：有图片模式的子模式与 THOR 认得的尺寸。名单与页面的 `codec_modes.rs`
// （`MFSK_PICTURE_MODES` / `THOR_PICTURE_MODES` / `THOR_PICTURE_SIZES`）一一对应。
const MFSK_PICTURE_MODES = ["16", "32", "64", "128"];
const THOR_PICTURE_MODES = ["11", "16", "22", "25", "32", "44", "56"];
const THOR_PICTURE_SIZES: Array<[number, number]> = [
  [160, 120], [320, 240], [59, 74], [120, 150], [240, 300], [640, 480],
];

test("解码 Worker：编码请求——MFSK / THOR 的图片发端按子模式与尺寸校验", async ({ page }) => {
  await page.goto("/");
  const pic = (width: number, height: number, extra: Record<string, unknown> = {}) => ({
    width, height, color: true, fill: 128, ...extra,
  });

  // MFSK：只有 16 / 32 / 64 / 128 有图片模式；采样率是该模式的标称值；spp 只认 2 / 4 / 8。
  for (const mode of MFSK_PICTURE_MODES) {
    const [r] = await converseEncode(page, [
      { codec: "mfsk", op: "encode", params: { mode, text: "", picture: pic(16, 12, { spp: 8 }) } },
    ]);
    expect(r.ok, `MFSK ${mode}: ${r.error}`).toBe(true);
    expect(r.kind, mode).toBe("Float32Array");
    expect(r.sampleRate, mode).toBe(8000);
    expect(r.finite, mode).toBe(true);
  }
  for (const mode of ["4", "8", "11", "22", "31", "64L", "128L"]) {
    const [r] = await converseEncode(page, [
      { codec: "mfsk", op: "encode", params: { mode, text: "", picture: pic(16, 12) } },
    ]);
    expect(r, `MFSK ${mode}`).toMatchObject({ ok: false, error: "encode-params:picture" });
  }
  // 缺省 spp = 8：与显式写 8 同长；快档更短（图要够大，报头多两个字符的开销才不会盖过）。
  const [implicit, explicit, fast] = await converseEncode(page, [
    { codec: "mfsk", op: "encode", params: { text: "", picture: pic(64, 48) } },
    { codec: "mfsk", op: "encode", params: { text: "", picture: pic(64, 48, { spp: 8 }) } },
    { codec: "mfsk", op: "encode", params: { text: "", picture: pic(64, 48, { spp: 4 }) } },
  ]);
  expect(implicit.length).toBe(explicit.length);
  expect(fast.length!).toBeLessThan(explicit.length!);

  // 图片参数不合法一律回 `encode-params:picture`，不悄悄改成别的值。
  const bad: Array<[string, Record<string, unknown>]> = [
    ["spp 不在 2/4/8", pic(16, 12, { spp: 3 })],
    ["像素数据长度对不上", pic(16, 12, { rgbLength: 16 * 12 * 3 - 1 })],
    ["尺寸为零", pic(0, 12, { rgbLength: 0 })],
    ["宽度是字符串", pic("16" as unknown as number, 12)],
    ["缺 color", { width: 16, height: 12, fill: 128 }],
    ["缺 rgb", { width: 16, height: 12, color: true }],
    ["尺寸超过像素上限", pic(2000, 2000, { rgbLength: 3 })],
  ];
  for (const [name, picture] of bad) {
    const [r] = await converseEncode(page, [
      { codec: "mfsk", op: "encode", params: { text: "", picture } },
    ]);
    expect(r, name).toMatchObject({ ok: false, error: "encode-params:picture" });
  }

  // 合成后超过上限是 `encode-params:length`（不是图片参数错）：1024 × 1024 彩色在 MFSK16 下要一个多小时。
  const [long] = await converseEncode(page, [
    { codec: "mfsk", op: "encode", params: { text: "", picture: pic(1024, 1024) } },
  ]);
  expect(long).toMatchObject({ ok: false, error: "encode-params:length" });

  // THOR：只有 11 / 16 / 22 / 25 / 32 / 44 / 56 有图片模式，采样率是该模式的标称值。
  const thorRates: Record<string, number> = { "11": 11025, "16": 8000, "22": 11025, "25": 8000, "32": 8000, "44": 11025, "56": 16000 };
  for (const mode of THOR_PICTURE_MODES) {
    const [r] = await converseEncode(page, [
      { codec: "thor", op: "encode", params: { mode, text: "CQ ", picture: pic(59, 74) } },
    ]);
    expect(r.ok, `THOR ${mode}: ${r.error}`).toBe(true);
    expect(r.sampleRate, mode).toBe(thorRates[mode]);
  }
  for (const mode of ["micro", "4", "5", "8", "25x4", "50x1", "50x2", "100"]) {
    const [r] = await converseEncode(page, [
      { codec: "thor", op: "encode", params: { mode, text: "", picture: pic(59, 74) } },
    ]);
    expect(r, `THOR ${mode}`).toMatchObject({ ok: false, error: "encode-params:picture" });
  }
  // 六种尺寸 × 彩色 / 灰度：640 × 480 彩色在 8 kHz 下超过样本上限（`length`），其余都能发。
  for (const [w, h] of THOR_PICTURE_SIZES) {
    for (const color of [true, false]) {
      const [r] = await converseEncode(page, [
        { codec: "thor", op: "encode", params: { mode: "16", text: "", picture: pic(w, h, { color }) } },
      ]);
      if (w === 640 && color) {
        expect(r, `${w}×${h} 彩色`).toMatchObject({ ok: false, error: "encode-params:length" });
      } else {
        expect(r.ok, `${w}×${h} ${color ? "彩色" : "灰度"}: ${r.error}`).toBe(true);
      }
    }
  }
  // 不在名单里的尺寸（像素数对得上也不行）。
  const [odd] = await converseEncode(page, [
    { codec: "thor", op: "encode", params: { mode: "16", text: "", picture: pic(100, 100) } },
  ]);
  expect(odd).toMatchObject({ ok: false, error: "encode-params:picture" });

  // 中心频率：图片同样守着同步音不掉到 0 Hz 以下（THOR 56 的带宽约 1 kHz，中心 500 Hz 时同步音在 −96 Hz）。
  const [low, fine] = await converseEncode(page, [
    { codec: "thor", op: "encode", params: { mode: "56", text: "", centerHz: 500, picture: pic(59, 74) } },
    { codec: "thor", op: "encode", params: { mode: "56", text: "", centerHz: 1500, picture: pic(59, 74) } },
  ]);
  expect(low).toMatchObject({ ok: false, error: "center-hz-out-of-range:500" });
  expect(fine.ok).toBe(true);
});

test("解码 Worker：编码请求——THOR 的 ID 信标（二次信道）与中心频率", async ({ page }) => {
  await page.goto("/");
  const text = "CQ CQ TEST";
  const [plain, withId, longId, custom] = await converseEncode(page, [
    { codec: "thor", op: "encode", params: { mode: "16", text } },
    { codec: "thor", op: "encode", params: { mode: "16", text, id: "DE BG1AAA" } },
    { codec: "thor", op: "encode", params: { mode: "16", text, id: "A".repeat(65) } },
    { codec: "thor", op: "encode", params: { mode: "16", text, id: "DE BG1AAA", centerHz: 2000 } },
  ]);
  expect(plain.ok).toBe(true);
  // 有 ID 的音频更长（多发了 ID 字符的码字）；ID 过长回 `encode-params:id`。
  expect(withId.ok).toBe(true);
  expect(withId.length!).toBeGreaterThan(plain.length!);
  expect(longId).toMatchObject({ ok: false, error: "encode-params:id" });
  // 显式的中心频率与 ID 可以同时给；样本数只取决于内容，不取决于中心。
  expect(custom.ok).toBe(true);
  expect(custom.length).toBe(withId.length);

  // MFSK 没有二次信道：给了 `id` 也被忽略（长度与不给相同），不会报错。
  const [mfsk, mfskId] = await converseEncode(page, [
    { codec: "mfsk", op: "encode", params: { mode: "16", text } },
    { codec: "mfsk", op: "encode", params: { mode: "16", text, id: "DE BG1AAA" } },
  ]);
  expect(mfskId.length).toBe(mfsk.length);
});

test("解码 Worker：编码入参不合法时按字段回 encode-params，未知 op 回 bad-request:op", async ({
  page,
}) => {
  await page.goto("/");
  const cases: Array<[string, Record<string, unknown>, string]> = [
    ["缺 params", {}, "encode-params:station"],
    ["台站不是一个字母", { ...NAVTEX_PARAMS, station: "FF" }, "encode-params:station"],
    ["类型不是字母", { ...NAVTEX_PARAMS, category: "1" }, "encode-params:category"],
    ["序号越界", { ...NAVTEX_PARAMS, serial: 100 }, "encode-params:serial"],
    ["序号非整数", { ...NAVTEX_PARAMS, serial: 4.5 }, "encode-params:serial"],
    ["序号是字符串", { ...NAVTEX_PARAMS, serial: "42" }, "encode-params:serial"],
    ["正文为空", { ...NAVTEX_PARAMS, text: "  " }, "encode-params:text"],
    ["正文过长", { ...NAVTEX_PARAMS, text: "A".repeat(1025) }, "encode-params:text"],
    ["相位段过长", { ...NAVTEX_PARAMS, phasingSecs: 61 }, "encode-params:phasing"],
    ["相位段缺失", { ...NAVTEX_PARAMS, phasingSecs: undefined }, "encode-params:phasing"],
  ];
  for (const [name, params, error] of cases) {
    const [reply] = await converseEncode(page, [{ codec: "navtex", op: "encode", params }]);
    expect(reply, name).toMatchObject({ ok: false, error });
  }

  // 拼错的 op 不能被当成解码，再以一条误导的 `bad-request:bytes` 报错。
  const [typo] = await converseEncode(page, [{ codec: "navtex", op: "encod", params: NAVTEX_PARAMS }]);
  expect(typo).toMatchObject({ ok: false, error: "bad-request:op" });

  // 显式写 `op: "decode"` 与缺省等价：走解码，空输入回解码错误码，而不是 op 相关的错误。
  const [decode] = await converse(page, [{ codec: "rtty", op: "decode", bytes: true, params: {} }]);
  expect(decode.ok).toBe(false);
  expect(decode.error).not.toMatch(/^bad-request/);

  // 编码之后同一个 Worker 仍是同一个 codec：换 codec 照旧回 mismatch。
  const mixed = await converseEncode(page, [
    { codec: "navtex", op: "encode", params: NAVTEX_PARAMS },
    { codec: "rtty", op: "encode", params: {} },
  ]);
  expect(mixed[0].ok).toBe(true);
  expect(mixed[1]).toMatchObject({ ok: false, error: "worker-codec-mismatch:navtex" });
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

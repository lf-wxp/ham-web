// 统一解码 Worker 协议：APT / SSTV / WSPR / RTTY / Olivia / DominoEX / NAVTEX / LRPT 共用这一份入口。
//
// 消息信封：
//   入站：{ codec: "apt"|"sstv"|"wspr"|"rtty"|"olivia"|"dominoex"|"navtex"|"lrpt", bytes: ArrayBuffer, params?: Object }
//   出站：{ ok: true, result } 或 { ok: false, error }
//   error 是英文错误码（"no-signal:RTTY" / "not-wav" …），页面侧按码取各语言词条
//   （见 crates/app/src/decode_worker.rs 的 localize_error）；不认识的错误原样显示。
//
// 各 codec 的 wasm glue 导出名统一为 `decode(bytes, params)`；
// params 按解码器取字段（wspr.baseHz / rtty.markHz / olivia.mode / dominoex.mode，缺省回退各自默认值）。
//
// 一个 Worker 只服务一个 codec：各胶水脚本顶层都是 `let wasm_bindgen = ...`，同一个 Worker 里
// 再 `importScripts` 第二个胶水会抛 `Identifier 'wasm_bindgen' has already been declared`。
// 页面约定「每次选新文件先 terminate 再 spawn」天然满足这一点；这里再显式守卫：同 codec 复用
// 已初始化的 wasm，换 codec 直接回错误码，不让胶水重复声明的报错冒出来误导排查。

const CODECS = Object.assign(Object.create(null), {
  apt: ["/apt-worker/apt_worker.js", "/apt-worker/apt_worker_bg.wasm"],
  sstv: ["/sstv-worker/sstv_worker.js", "/sstv-worker/sstv_worker_bg.wasm"],
  wspr: ["/wspr-worker/wspr_worker.js", "/wspr-worker/wspr_worker_bg.wasm"],
  rtty: ["/rtty-worker/rtty_worker.js", "/rtty-worker/rtty_worker_bg.wasm"],
  olivia: ["/olivia-worker/olivia_worker.js", "/olivia-worker/olivia_worker_bg.wasm"],
  dominoex: ["/dominoex-worker/dominoex_worker.js", "/dominoex-worker/dominoex_worker_bg.wasm"],
  navtex: ["/navtex-worker/navtex_worker.js", "/navtex-worker/navtex_worker_bg.wasm"],
  lrpt: ["/lrpt-worker/lrpt_worker.js", "/lrpt-worker/lrpt_worker_bg.wasm"],
  ft8: ["/ft8-worker/ft8_worker.js", "/ft8-worker/ft8_worker_bg.wasm"],
});

// 已加载的 codec 与其 wasm 初始化 Promise（失败也保留：胶水已声明，重试只会得到重复声明错误）。
let loaded = null;

function load(codec, entry) {
  if (loaded) {
    if (loaded.codec !== codec) {
      throw new Error("worker-codec-mismatch:" + loaded.codec);
    }
    return loaded.ready;
  }
  importScripts(entry[0]);
  // 单对象参数：位置参数形式在新版 wasm-bindgen 里已弃用并会在控制台告警。
  const ready = wasm_bindgen({ module_or_path: entry[1] });
  loaded = { codec, ready };
  return ready;
}

// 结果里「独占整块 ArrayBuffer」的 Uint8Array（SSTV 的 rgba、LRPT 的 rgb 可达几十 MB）走
// transfer 而不是结构化克隆。只转移独占整块的：指向 wasm 线性内存的视图转移会让整个实例失效。
function transferables(result) {
  const out = [];
  if (result && typeof result === "object") {
    for (const v of Object.values(result)) {
      if (
        v instanceof Uint8Array &&
        v.byteOffset === 0 &&
        v.byteLength === v.buffer.byteLength &&
        v.buffer instanceof ArrayBuffer
      ) {
        out.push(v.buffer);
      }
    }
  }
  // 同一块 buffer 重复出现在 transfer 列表里会抛 DataCloneError。
  return [...new Set(out)];
}

// 本条消息是否已经回信。`unhandledrejection` 可能晚于正常回信（wasm 实例化失败后的
// 残余 rejection），没有这个标志会在 `ok:true` 之后再补一条 `ok:false`，页面侧会把
// 已渲染的结果覆盖成错误。
let replied = false;

self.onmessage = async (event) => {
  replied = false;
  try {
    const { codec, bytes, params } = event.data || {};
    // 用 Object.create(null) 的表：`"constructor"` / `"__proto__"` 之类不会命中原型链。
    const entry = typeof codec === "string" ? CODECS[codec] : undefined;
    if (!entry) {
      // 错误细节按惯例用英文：这条信息会经 { ok:false, error } 原样显示在
      // en/es 界面上，不能硬编码中文。
      throw new Error("unknown codec: " + codec);
    }
    if (!(bytes instanceof ArrayBuffer) && !ArrayBuffer.isView(bytes)) {
      throw new Error("bad-request:bytes");
    }
    await load(codec, entry);
    const input = bytes instanceof ArrayBuffer ? new Uint8Array(bytes) : new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const result = wasm_bindgen.decode(input, params || {});
    self.postMessage({ ok: true, result }, transferables(result));
    replied = true;
  } catch (err) {
    self.postMessage({ ok: false, error: String((err && err.message) || err) });
    replied = true;
  }
};

// 兜底：没被 try/catch 接住的异步错误（如 wasm 实例化失败后的未处理 rejection）也要回信，
// 否则页面一直转圈到超时。
self.addEventListener("unhandledrejection", (ev) => {
  ev.preventDefault();
  if (replied) {
    return;
  }
  self.postMessage({ ok: false, error: String((ev.reason && ev.reason.message) || ev.reason) });
});

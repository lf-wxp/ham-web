# 解码 Worker 协议（统一入口）

本文档是「文件 → 浏览器内解码」链路的**现行规范**：所有解码器页面（APT / SSTV /
WSPR / RTTY / Olivia / DominoEX / NAVTEX / LRPT）共用一份 JS 入口
`public/decode-worker.js`，各自的 wasm 胶水遵守同一份导出契约。新增解码器照
文末清单走一遍即可。

## 消息信封

```ts
// 入站（页面 → worker）
{ codec: string, bytes: ArrayBuffer, params?: Object }
// 出站（worker → 页面）
{ ok: true, result: Object } | { ok: false, error: string }
```

- `bytes`：用户文件的原始字节（WAV 等）；WAV 解析在各解码器 crate 内做
  （`ham_web_apt::parse_wav`）。
- `params`：按解码器取字段，缺省回退各自默认值（见下表）；无参数的解码器
  收到 `{}`。
- 出错一律 `{ ok: false, error }`。`error` 是**英文错误码**（`码` 或 `码:参数`，如
  `no-signal:RTTY`、`not-wav`、`need-sample-rate:44100`），Worker 在 wasm 里拿不到当前
  语言，所以不写界面文案；`parse_outcome` 按码取 `tools.decode-err-*` 词条（zh / en / es），
  不认识的错误（wasm trap、`unknown codec: …`）原样显示，空串时页面回退通用文案。
  新增错误码要同时：在 worker 里返回码、在 `localize_error` 加一支、补三份词条。
  WAV 解析错误用 `ham_web_apt::Error::code()`（`not-wav` / `unsupported-format` /
  `truncated` / `no-audio` / `too-short`），不要用 `to_string()`（那是中文 `Display`）。

现行错误码（wasm 侧的都实现了 `localize_error` 分支，词条在 `tools.decode-err-*`；
最后一行是 JS 入口 `decode-worker.js` 自己的协议错误，没有词条，原样显示英文）：

| 码 | 来源 | 含义 |
| --- | --- | --- |
| `not-wav` / `unsupported-format` / `truncated` / `no-audio` / `too-short` | 各 worker 的 WAV 解析 | 见 `ham_web_apt::Error::code()` |
| `no-signal:<名>` | RTTY / Olivia / DominoEX / NAVTEX / WSPR | 没有可解调的信号（门限见下节） |
| `no-vis` | SSTV | 未检测到 VIS 头 |
| `unsupported-submode:<模式>` / `olivia-params:<音调数>,<带宽>` | Olivia / DominoEX | 子模式不在标准表内 |
| `mark-hz-out-of-range:<Hz>` / `base-hz-out-of-range:<Hz>` | RTTY / WSPR | 频率越界（须在 0 与采样率一半之间） |
| `rate-too-low:<最低 Hz>` | Olivia / DominoEX | 采样率低于音调带上沿的 Nyquist 要求 |
| `need-sample-rate:<Hz>` / `need-stereo:<声道数>` | LRPT | 需要 288 kHz 立体声 I/Q WAV |
| `image-incomplete:<帧数>` | LRPT | 帧层有输出但图像层不完整（不等于「帧层都通过了」） |
| `input-too-large:<字节>` | LRPT | 超过 worker 的输入上限（页面已限 128 MiB，这里是兜底） |
| `worker-load-failed` | **页面侧合成** | `/decode-worker.js` 或胶水加载失败（404 / 被拦截 / 语法错误） |
| `bad-request:bytes` / `worker-codec-mismatch:<已加载 codec>` | `decode-worker.js` | 信封缺 `bytes`；同一个 Worker 收到第二种 codec |

## 已接入的 codec

| codec | params 字段（缺省值） | `result` 形态 | wasm 目录 |
| --- | --- | --- | --- |
| `apt` | — | APT 图像元数据与各通道 / 合成图像素（字段见 `crates/apt-worker/src/lib.rs`） | `/apt-worker/` |
| `sstv` | — | `{ width, height, mode, source_sample_rate, duration_seconds, rgba: Uint8Array }` | `/sstv-worker/` |
| `wspr` | `baseHz`（1500） | 解码文本 / 元数据 | `/wspr-worker/` |
| `rtty` | `markHz`（2125） | `{ text }` | `/rtty-worker/` |
| `olivia` | `mode`（`"32/1000@1500"`） | `{ text }` | `/olivia-worker/` |
| `dominoex` | `mode`（`"8"`）、`fec`（`false`；`true` = MultiPSK 兼容 FEC） | `{ text, id }`（`id` = 二次信道） | `/dominoex-worker/` |
| `navtex` | — | `{ messages: [{ transmitter, category, serial, text, corrected, errors, chars, accepted }] }` | `/navtex-worker/` |
| `lrpt` | — | `{ width, height, rgb: Uint8Array }`（RGB 三平面交错） | `/lrpt-worker/` |

## 质量门限与能力边界

- **噪声 / 静音不能当解码结果**：RTTY 要求锁定点之后有足够多且占多数的合法字符
  （停止位 / 起始位结构），DominoEX 要求锁定后的「无音调变化」nibble（16/17）
  比例 ≤ 5%，NAVTEX 另有 4% 残余错误率门限。达不到一律 `no-signal:<codec>`，
  页面显示「没有可解调的信号」而不是一段乱码。
- **门限是概率性的**：门限之下仍可能出现零星疑似字符，输出不等于校验过的数据。
  有内部校验的模式才有硬证据（NAVTEX 的 4-mark 权重、LRPT 的 RS(255,223)）。
- **LRPT 的容错是尽力而为**：RS 纠不了的帧跳过、帧计数允许空洞、JPEG 段解不开只丢
  该段（不完整行整行丢弃），采样相位与字节网格一起搜索。真实弱信号仍可能只得到
  `image-incomplete`（它不代表「帧层都通过了」）。
- **软判决、时钟漂移、载波恢复不在范围内**：各解码器都是硬判决 + 固定速率假设，
  真实弱录音的完整正文不由合成测试保证（逐条说明见各 crate 的模块文档，尚未验证的部分列在仓库根 `ROADMAP.md`「未完成」）。
- **Olivia 在噪声下可能给出乱码而不是 `no-signal`**：WHT 门限是概率性的（实测随机噪声里
  偶尔会解出几个可打印字符），页面只对「完全空」的结果报 `no-signal`。

## Worker 生命周期约束

- **一个 Worker 只服务一个 codec。** 各 wasm-bindgen 胶水顶层都是 `let wasm_bindgen = …`，
  同一个 Worker 里再 `importScripts` 第二个胶水会抛 `Identifier 'wasm_bindgen' has already
  been declared`。`decode-worker.js` 的守卫：同 codec 复用已初始化的 wasm，换 codec 回
  `worker-codec-mismatch:<已加载 codec>`。页面「每次选新文件先 terminate 再 spawn」天然满足它。
- **加载失败要有出口。** 入口脚本或胶水 404 / 被拦截 / 语法错误时浏览器只触发 Worker 的
  `error` 事件、没有任何回信。`spawn_worker` 因此同时挂 `onerror`（`decode_worker::error_handler`
  合成的转发闭包），把它转成一条合成的 `{ ok:false, error:"worker-load-failed" }` 交给页面原有的
  `onmessage`，走同一套代际 / 超时 / 本地化路径。**注意事件类型**：加载失败派发的是普通
  `Event`（不是 `ErrorEvent`，也没有 `message` 属性），所以转发闭包按 `Event` 接收、按属性读
  `message`；声明成 `ErrorEvent` 会让 wasm-bindgen 转换时抛错，消息永远到不了页面。
  `message` 非空说明是运行期异常（wasm trap），码变成 `worker-run-failed:<message>`。
  Worker 端另有 `unhandledrejection` 兜底。
- **结果里独占整块 buffer 的 `Uint8Array`**（SSTV 的 `rgba`、LRPT 的 `rgb`）由
  `decode-worker.js` 自动放进 transfer 列表，避免几十 MB 的结构化克隆多拷一份；指向 wasm
  线性内存的视图不会被转移。

## wasm 胶水契约

每个 `*-worker` crate 的导出函数**固定**为：

```rust
#[wasm_bindgen]
pub fn decode(bytes: &[u8], params: &JsValue) -> Result<JsValue, JsValue>
```

- 构建方式：`wasm-bindgen --target no-modules --out-name <codec>_worker`，
  由 `[tasks.build-<codec>-worker]`（`Makefile.toml`）产出到
  `public/<codec>-worker/<codec>_worker.js` + `_bg.wasm`。
- 返回对象可被 `postMessage` 结构化克隆：普通字段 + `js_sys::Uint8Array`
  （用 `copy_from` 拷贝，不要传借用的视图）。

## 前端侧桥（`crates/app/src/decode_worker.rs`）

**优先用 `crate::decode_session::DecodeSession`**（`crates/app/src/decode_session.rs`）：它把下面这些函数与
`generation` / 超时 / 取消 / 重置的编排收成一份实现，页面只需给 `DecodeConfig`（codec、体积上限、
是否转移 buffer、`parse`、三条文案回调：文件过大 / 超时 / 协议损坏）并在 `start` 里构造 `params`。八个解码页全部已迁移，
APT 页同样已迁移（上传区 `AptUploader` 保留整块拖放，经 `Callback<File>` 接入会话）。结果解析用 `decode_worker::{get_u32, get_f64, get_str,
get_bool, get_u8s}`，缺字段按错误处理，不要再 `unwrap_or_default()`。

底层函数（`DecodeSession` 内部使用，手写页面仍可直接调）：

- `spawn_worker(&onmessage, &onerror)` —— 建 `/decode-worker.js` 的 Worker（同时挂 `onerror`，见上）；
- `shutdown_worker(&worker)` —— 先摘 `onmessage` / `onerror` 再 `terminate()`（队列里已排的消息不会再进已释放的 Rust 闭包）；
- `retain_message_handler(closure)` / `retain_error_handler(closure)` —— 把消息 / 错误闭包的所有权交给组件，
  卸载时释放，**必须在组件体内调用**；`error_handler(&onmessage)` 负责合成那条错误回信；
- `read_file_bytes(&File)` / `read_file_buffer(&File)` —— 文件 → 字节 / `ArrayBuffer`（异步）；
- `post_decode(&worker, codec, &bytes, &params)` / `post_decode_transfer(…)` —— 组信封发送（后者转移 buffer，大录音用）；
- `decode_timeout_slot(processing)` / `arm_decode_timeout_with(…)` —— 超时兜底槽位与定时器；
- `parse_outcome(&data)` —— 拆 `{ ok, result?, error? }`，页面再按自己字段解析
  `result`（每页一个 `parse_message` 小函数，见各 `pages/<codec>_decoder/`）。

页面的生命周期约定：每次选择新文件**先 terminate 旧 Worker 再 spawn 新的**
（Worker 无法取消在途解码），用代际计数（`generation`）丢弃过期回复。文件读取
`await` 后必须先用 `generation.try_get_value()` 检查过期或已卸载的批次，不可读取已释放值。

消息闭包通过 `retain_message_handler` 保留 Rust 所有权并在卸载时释放，不用 `forget()`。
有超时的页面必须在组件体调用 `decode_timeout_slot(processing)`，向 `arm_decode_timeout_with`
传入槽位；完成、取消、替换及卸载均清理旧句柄，超时回调也安全检查已释放值。LRPT
保留 `read_file_buffer` + `post_decode_transfer` 路径，避免大录音在主线程重复拷贝。

NAVTEX 的 `accepted` 是完整标准报头、CR/LF 与 NNNN 后的残余字符错误率门限（仓内
4% 质量政策）。硬判决实现按 LSB 位序、五字符 DX/RX 延迟处理；真实弱录音的时钟漂移、
软判决及完整正文恢复不由合成测试保证。

## 新增一个解码器的清单

1. **核心 crate**（纯 Rust + 单测）：解码函数签名从 WAV 字节或样本起。
2. **worker crate**：上述导出契约 + 目录 `crates/<codec>-worker/`。
3. **`Makefile.toml`**：复制 `[tasks.build-<既有>-worker]` 段落改三处
   （`-p ham-web-<codec>-worker` / `public/<codec>-worker` / `--out-name
   <codec>_worker`；APT 的任务名是历史遗留的 `build-worker`，请复制别的 codec 的段落），
   并同步以下**三份依赖列表与两个循环**：`dev`、`dev-full`、`build-web` 的 `dependencies`；
   `clippy` 与 `test` 任务里 `for codec in …` 的清单。
   **`Dockerfile`** 同样要改三处：`-p` 列表、构建 `for` 循环、末尾产物断言
   （`dist/<codec>-worker/<codec>_worker.js` 与 `_bg.wasm`）。
   **根 `Cargo.toml`** 的 `members` 加两个新 crate。构建脚本是共享的
`scripts/build-decode-worker.sh <codec>`，新任务只写一行 `script = "sh scripts/build-decode-worker.sh <codec>"`，
并把 codec 名加进脚本里的 `case` 白名单。
4. **`crates/app/index.html`**：加一行
   `<link data-trunk rel="copy-dir" href="../../public/<codec>-worker" />`。
5. **`public/decode-worker.js`**：`CODECS` 表加一项（两文件路径）。
6. **页面** `crates/app/src/pages/<codec>_decoder/`：`mod.rs` + `_page.rs` +
   `_worker.rs`（`parse_message`），照任一既有页面（如 `rtty_decoder`）复制结构。
7. **登记**：`pages/mod.rs`、`app/main_content.rs` 的路由、`core/registry.rs`
   的 `Module`（`GROUP_TOOL_CALC` 分组，导航自动收纳）。
8. **i18n**：zh/en/es 三份 `tools.json` 的键（`cargo make i18n-pack` 校验
   覆盖率 100%）。
9. **e2e**：`e2e/tests/<codec>_decoder.spec.ts` —— 用**独立 TS 实现**编码合成
   WAV（延续「三套实现互锁」的约定），上传断言解码结果；坏样本如实报错。
   登记到 `e2e/coverage-targets.json`。

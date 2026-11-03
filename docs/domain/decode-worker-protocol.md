# 解码 Worker 协议（统一入口）

本文档是「文件 → 浏览器内解码」链路的**现行规范**：十一个解码器页面（APT / SSTV /
WSPR / RTTY / Olivia / DominoEX / NAVTEX / LRPT / FT8 / Feld Hell / WEFAX），加上归在
软 TNC 小工具（`/tools#soft-tnc`）里的 AFSK1200，共十二个 codec 共用一份 JS 入口
`public/decode-worker.js`，各自的 wasm 胶水遵守同一份导出契约，页面侧统一经
`crate::decode_session::DecodeSession` 接入。新增解码器照文末清单走一遍即可。

不在这条链路上的三处，别照它们的写法接入新解码器——它们代表三种**不同的架构形态**，
不是同一件事的三种偷懒写法。Worker 协议存在的前提是「解码重、要懒加载、一次性文件解调」，
三处特例各自不满足：

- **PSK31**（`pages/psk_decode.rs`）仍在主线程解码整段 WAV，错误走自己的
  `radio.not-a-valid-wav` / `radio.no-text-decoded-make`。**为什么不搬**：解码器
  `core/psk31.rs` 住在主包 `ham-web-core` 里，而 PSK31 是 31.25 波特 BPSK + Varicode，
  解几秒音频是毫秒级——既不卡主线程、也没给主包增重多少；它还收发共用（`synthesize` 供
  `/tools#mode-encoder` 发端）。拆成独立 crate + worker 要走整套清单，换来「主线程少跑几
  毫秒」，不值得，也没迁移必要。
- **CW**（`pages/morse/cw_decoder.rs`）吃麦克风实时音频（`getUserMedia` + Web Audio 的
  `AudioContext` / `AnalyserNode` / `BiquadFilterNode`），是**连续流式解码**：音频持续进、
  文字增量出，带实时电平表与自适应门限。Worker 信封 `{codec, bytes, params} → {ok, result}`
  是**一次性请求/响应**，装不下流式输入与增量 UI；且 Web Audio 节点跑在主线程，后台化要换
  `AudioWorklet` + buffer transfer，是另一套架构。
- **离线瀑布**（`pages/sdr_waterfall/`）只做频谱分析（`core/spectrum::analyze_waterfall`
  FFT 成瀑布图），**不做解调、不提取报文**。Worker 协议是给「音频 → 报文/图像」解调器的，
  瀑布是「音频 → 可视化频谱」，且 `spectrum` 同时被实时 SDR 瀑布复用、住在 core 里。

## 消息信封

```ts
// 入站（页面 → worker）
{ codec: string, bytes: ArrayBuffer | ArrayBufferView, params?: Object }
// 出站（worker → 页面）
{ ok: true, result: Object } | { ok: false, error: string }
```

- `bytes`：用户文件的原始字节（WAV 等）；WAV 解析在各解码器 crate 内做
  （`ham_web_apt::parse_wav`）。`decode-worker.js` 同时接受 `ArrayBuffer` 与类型化数组 /
  `DataView`（后者按 `byteOffset` / `byteLength` 取视图），缺 `bytes` 才回
  `bad-request:bytes`。
- `params`：按解码器取字段，缺省回退各自默认值（见下表）。无参数的解码器在页面侧发
  `params: null`（APT 页即如此），`decode-worker.js` 归一成 `{}` 再交给 wasm。
- 出错一律 `{ ok: false, error }`。`error` 是**英文错误码**（`码` 或 `码:参数`，如
  `no-signal:RTTY`、`not-wav`、`need-sample-rate:44100`），Worker 在 wasm 里拿不到当前
  语言，所以不写界面文案；`parse_outcome` 按码取 `tools.decode-err-*` 词条（zh / en / es），
  不认识的错误（wasm trap、`unknown codec: …`）原样显示，空串时页面回退通用文案。
  新增错误码要同时：在 worker 里返回码、在 `localize_error` 加一支、补三份词条。
  WAV 解析错误用 `ham_web_apt::Error::code()`（`not-wav` / `unsupported-format` /
  `truncated` / `no-audio`），不要用 `to_string()`（那是中文 `Display`）；`too-short` 是
  APT / SSTV / WEFAX 解码器在样本不足时自己给的，不在 WAV 解析阶段。

页面侧还有几类**不是错误码**的文案，直接取词条、不经 `localize_error`：文件过大
（`DecodeConfig::too_large`）、超时（`DecodeConfig::timed_out`）、协议损坏兜底
（`DecodeConfig::failed`）、文件读取 / 发送失败（`tools.decode-io-failed`）、结果缺字段
（`common.field-is-missing` / `common.field-is-not-a`）。

现行错误码（wasm 侧的都实现了 `localize_error` 分支，词条在 `tools.decode-err-*`，只有
`not-wav` 复用 `radio.not-a-valid-wav`；页面侧合成的两条也有词条；最后一行的 JS 入口协议
错误没有词条，原样显示英文）：

| 码 | 来源 | 含义 |
| --- | --- | --- |
| `not-wav` / `unsupported-format` / `truncated` / `no-audio` | 各 worker 的 WAV 解析 | 见 `ham_web_apt::Error::code()` |
| `too-short` | APT / SSTV / WEFAX | 音频不足以解出一帧 / 一张图 |
| `no-signal:<名>` | RTTY / Olivia / DominoEX / NAVTEX / WSPR / HELL / WEFAX | 没有可解调的信号（门限见下节；`<名>` 是 `RTTY` / `Olivia` / `DominoEX` / `NAVTEX` / `WSPR` / `HELL` / `WEFAX`。WEFAX 的 `NoSync` 也归这一码，不再单独回 `no-sync`） |
| `no-vis` | SSTV | 未检测到 VIS 头 |
| `unsupported-submode:<模式>` | Olivia / DominoEX | 子模式不在标准表内（语法解析失败） |
| `olivia-params:<音调数>,<带宽>` | Olivia | 参数组合语法合法但语义不在标准表内（音调数 ≠ 31.25 波特 × 带宽） |
| `olivia-params:<音调数>,<带宽>@<中心频率>` | Olivia | 中心频率非有限，或中心 + 半带宽越过 Nyquist；`localize_error` 按 `@` 拆出第三个参数，走 `tools.decode-err-olivia-params-range` |
| `mark-hz-out-of-range:<Hz>` | RTTY | 传号频率越界（须 > 0 且传号 + 170 Hz 小于采样率一半） |
| `base-hz-out-of-range:<Hz>` | WSPR | 基带频率越界（须 > 0 且小于采样率一半） |
| `rate-too-low:<最低 Hz>` | DominoEX | 采样率低于音调带上沿的 Nyquist 要求（Olivia 的同类检查报带 `@` 的 `olivia-params`） |
| `need-sample-rate:<Hz>` / `need-stereo:<声道数>` | LRPT | 采样率低于 144 kHz，或不是双声道 I/Q；参数是录音的实际值 |
| `ft8-sample-rate:<Hz>` | FT8 / FT4 | 采样率不是 12000 Hz（时频分析按该采样率设计，别的采样率会静默解错，所以直接拒绝） |
| `image-incomplete:<帧数>` | LRPT | 帧层有输出但图像层不完整（不等于「帧层都通过了」） |
| `image-size-mismatch` | WEFAX（仅 `modulate` 的入参校验，不经 Worker） | 灰度图 `width × height` 与 `gray` 长度不符；刻意不复用 LRPT 的 `image-incomplete`（那个码带帧数占位符） |
| `input-too-large:<字节>` | LRPT | 超过 worker 的 192 MiB 输入上限（页面已限 128 MiB，这里是兜底） |
| `worker-load-failed` / `worker-run-failed:<message>` | **页面侧合成**（`decode_worker::error_handler`） | 入口脚本或胶水加载失败（404 / 被拦截 / 语法错误）；运行期异常（wasm trap，带原始英文 message） |
| `bad-request:bytes` / `worker-codec-mismatch:<已加载 codec>` / `unknown codec: <名>` | `decode-worker.js` | 信封缺 `bytes`；同一个 Worker 收到第二种 codec；codec 不在 `CODECS` 表里 |

## 已接入的 codec

| codec | params 字段（缺省值） | `result` 形态 | wasm 目录 |
| --- | --- | --- | --- |
| `apt` | —（页面发 `null`） | `{ width, height, lines, source_sample_rate, duration_seconds, channel_a, channel_b, false_color: Uint8Array }`（三张图都是一字节一像素） | `/apt-worker/` |
| `sstv` | —（页面发 `null`） | `{ width, height, mode, source_sample_rate, duration_seconds, rgba: Uint8Array }` | `/sstv-worker/` |
| `wspr` | `baseHz`（1500） | `{ callsign, locator, power, text }`（`text` 是 `"调用 网格 功率"` 的展示串） | `/wspr-worker/` |
| `rtty` | `markHz`（2125；空号 = 传号 + 170 Hz） | `{ text }` | `/rtty-worker/` |
| `olivia` | `mode`（`"32/1000@1500"`） | `{ text }` | `/olivia-worker/` |
| `dominoex` | `mode`（`"8"`）、`fec`（`false`；`true` = MultiPSK 兼容 FEC） | `{ text, id }`（`id` = 二次信道，可能为空串） | `/dominoex-worker/` |
| `navtex` | —（页面发 `null`） | `{ messages: [{ transmitter, category, serial, text, corrected, errors, chars, accepted }] }` | `/navtex-worker/` |
| `lrpt` | —（页面发 `null`） | `{ width, height, rgb: Uint8Array }`（RGB 三平面交错） | `/lrpt-worker/` |
| `ft8` | `protocol`（`"ft8"`；`"ft4"`） | `{ messages: [{ text, freq_hz, time_sec, sync_db }], unsupported, slots }`（`time_sec` 距录音开头；`sync_db` 是同步强度而非 SNR；`unsupported` 是 CRC 通过但类型无字段解析的条数；`slots` 是逐个解码的时隙数） | `/ft8-worker/` |
| `hell` | —（页面发 `null`） | `{ text }` | `/hell-worker/` |
| `wefax` | —（页面发 `null`） | `{ width, height, gray: Uint8Array }`（灰度图一字节一像素） | `/wefax-worker/` |
| `afsk1200` | —（页面发 `null`） | `{ frames: [{ kind: "ui", source, destination, info } \| { kind: "raw", hex }], truncated }`；录音超过 60 秒时 `truncated: true`，只解前一段 | `/afsk1200-worker/` |

AFSK1200 **没有对应的 `/afsk1200/` 核心 crate**：它的解调/调制逻辑住在 `ham-web-core`
（`core/afsk1200.rs` + `core/kiss.rs`）。理由是它和其余十一个 codec 形态不同——它是
**软 TNC**（`帧 ↔ 基带音频` 收发双向），发端 `modulate_frame_for_tx` 还要供 `/tools#soft-tnc`
下载 `afsk1200.wav` 复用，不是只有「音频 → 报文/图像」的收端；于是核心按「不依赖浏览器的
逻辑放 `ham-web-core`」的规则从第一天就留在 core（与 PSK31 同因：`synthesize` 供发端复用）。
`afsk1200-worker` 是**后来补的薄胶水**：软 TNC 的解调原先在主线程跑（8 kHz 录音重采样到
48 kHz 后约 2500 万样本 × 40 位相位 × 2 个 NRZI 初值穷举，页面会冻结数十秒），因此只把
RX 下沉到 Worker、核心没搬，worker 直接 `use ham_web_core::afsk1200`。协议约束的是 worker
的导出契约，不要求核心独立成 crate，所以这不构成例外。

页面体积上限（`DecodeConfig::max_bytes`，超出直接报错、不发 Worker）：8 MiB（RTTY /
Olivia / DominoEX / NAVTEX / FT8 / Feld Hell / WEFAX / AFSK1200）、32 MiB（SSTV / WSPR）、96 MiB（APT）、128 MiB（LRPT，
也是唯一 `transfer: true` 的页面）。8 MiB 这个值统一取自 `decode_session::MAX_UPLOAD_BYTES`，
各页不要再各写一份。

## 质量门限与能力边界

- **噪声 / 静音不能当解码结果**：RTTY 要求锁定点之后有足够多且占多数的合法字符
  （停止位 / 起始位结构），DominoEX 要求锁定后的「无音调变化」nibble（16/17）
  比例 ≤ 5%，NAVTEX 另有 4% 残余错误率门限。达不到一律 `no-signal:<名>`，
  页面显示「没有可解调的信号」而不是一段乱码。
- **门限是概率性的**：门限之下仍可能出现零星疑似字符，输出不等于校验过的数据。
  有内部校验的模式才有硬证据（NAVTEX 的 4-mark 权重、LRPT 的 RS(255,223)）。
- **LRPT 的容错是尽力而为**：RS 纠不了的帧跳过、帧计数允许空洞、JPEG 段解不开只丢
  该段（不完整行整行丢弃），采样相位与字节网格一起搜索。真实弱信号仍可能只得到
  `image-incomplete`（它不代表「帧层都通过了」）。解调有两个前端：288 kHz 走整数 4 倍采样
  的 OQPSK，其余采样率（≥ 144 kHz）走 QPSK + 定时 / 载波环路；先跑与采样率匹配的那条，
  解不出图再试另一条（288 kHz 的真实录音两条都可能命中）。
- **软判决、时钟漂移、载波恢复不在范围内**：各解码器都是硬判决 + 固定速率假设，
  真实弱录音的完整正文不由合成测试保证（逐条说明见各 crate 的模块文档，尚未验证的部分列在仓库根 `ROADMAP.md`「未完成」）。
- **Olivia 在噪声下可能给出乱码而不是 `no-signal`**：WHT 门限是概率性的（实测随机噪声里
  偶尔会解出几个可打印字符），页面只对「完全空」的结果报 `no-signal`。
- **FT8 / FT4 的「零结果」不是错误**：`messages` 为空仍回 `ok:true`（`slots` /
  `unsupported` 仍有意义），由页面提示 `tools.ft8-no-messages`。其余解码器空结果一律
  `no-signal:<名>`。

## Worker 生命周期约束

- **一个 Worker 只服务一个 codec。** 各 wasm-bindgen 胶水顶层都是 `let wasm_bindgen = …`，
  同一个 Worker 里再 `importScripts` 第二个胶水会抛 `Identifier 'wasm_bindgen' has already
  been declared`。`decode-worker.js` 的守卫：同 codec 复用已初始化的 wasm（连失败的
  初始化 Promise 也缓存 —— 胶水已经声明过，重试只会得到重复声明错误），换 codec 回
  `worker-codec-mismatch:<已加载 codec>`。页面「每次选新文件先 terminate 再 spawn」天然满足它。
- **Worker 惰性创建**：`DecodeSession` 进页面不建 Worker，第一次选文件才 spawn。挂载即建会
  让每次进页面白加载 1–2 MB wasm，随后被首个文件触发的 terminate 回收。
- **加载失败要有出口。** 入口脚本或胶水 404 / 被拦截 / 语法错误时浏览器只触发 Worker 的
  `error` 事件、没有任何回信。`spawn_worker` 因此同时挂 `onerror`（`decode_worker::error_handler`
  合成的转发闭包），把它转成一条合成的 `{ ok:false, error:"worker-load-failed" }` 交给页面原有的
  `onmessage`，走同一套代际 / 超时 / 本地化路径。**注意事件类型**：加载失败派发的是普通
  `Event`（不是 `ErrorEvent`，也没有 `message` 属性），所以转发闭包按 `Event` 接收、按属性读
  `message`；声明成 `ErrorEvent` 会让 wasm-bindgen 转换时抛错，消息永远到不了页面。
  `message` 非空说明是运行期异常（wasm trap），码变成 `worker-run-failed:<message>`。
  Worker 端另有 `unhandledrejection` 兜底。
- **结果里独占整块 buffer 的 `Uint8Array`**（SSTV 的 `rgba`、LRPT 的 `rgb`、WEFAX 的 `gray`）
  由 `decode-worker.js` 自动放进 transfer 列表，避免几十 MB 的结构化克隆多拷一份；指向 wasm
  线性内存的视图不会被转移。判据是「`byteOffset == 0` 且 `byteLength == buffer.byteLength`」，
  所以给某个 codec 加「读后复用这块 Uint8Array」的优化时，它就不再满足判据（否则转移后 worker
  手里的 buffer 会被 detach）。

## wasm 胶水契约

每个 `*-worker` crate 的导出函数**固定**为：

```rust
#[wasm_bindgen]
pub fn decode(bytes: &[u8], params: &JsValue) -> Result<JsValue, JsValue>
```

- 构建方式：`sh scripts/build-decode-worker.sh <codec>`（`wasm-bindgen --target no-modules
  --no-typescript --out-name <codec>_worker`），产出 `public/<codec>-worker/<codec>_worker.js`
  + `_bg.wasm`；`[tasks.build-<codec>-worker]` 只是它的薄包装，供本地单独重建一个 codec。
- **codec 清单只有一处来源：`crates/*-worker/` 目录本身。** `scripts/decode-workers.sh`
  扫描这些目录输出 codec 名（同时按目录名校验包名 `ham-web-<codec>-worker`，这条链断了
  wasm 产物名就对不上）；`build-decode-worker.sh` 无参时构建它列出的全部，
  `[tasks.build-workers]`、`[tasks.clippy]` / `[tasks.test]` 的循环与 `Dockerfile` 都走这一份。
  新增 Worker 只需加 crate 目录，**不必再改 Dockerfile 或 Makefile 的名单**。
- 返回对象可被 `postMessage` 结构化克隆：普通字段 + `js_sys::Uint8Array`
  （用 `copy_from` 拷贝，不要传借用的视图）。

### Dockerfile 里的 worker 构建

镜像里**不跑 `cargo make`**（builder 阶段不装 cargo-make，且要复用 BuildKit 的
`--mount=type=cache`），worker 由同一条命令生成：

```dockerfile
sh scripts/build-decode-worker.sh
```

`public/*-worker/` 被 `.gitignore` 排除、`dist/` 又只由 `trunk build` 按
`crates/app/index.html` 的 `copy-dir` 拷入，所以镜像里的 worker 只能由这一步生成；
而 codec 名单是脚本扫 `crates/*-worker` 得来的，**Dockerfile 里没有硬编码的 codec 列表**。
前提与顺序：

1. **wasm-bindgen CLI 的版本从 `Cargo.lock` 取，并且在同一条 `RUN` 里硬校验**：
   `ARG WASM_BINDGEN_VERSION` 为空时用 `awk` 解析 `^name = "wasm-bindgen"$` 后面的
   `version`，`cargo install wasm-bindgen-cli --version "${wb}" --locked
   --target-dir /opt/wasm-bindgen-target`，装完断言 `wasm-bindgen --version` 等于 `${wb}`。
   胶水与 `_bg.wasm` 的版本不一致**不会编译失败**，只在运行时抛 `cannot read properties`，
   所以必须在构建期比对（构建脚本里也有同一份检查，两道都留）。
2. **先出工具二进制，再出 worker**：`cargo build --release -p ham-web-tools
   -p ham-web-server` 产出 `target/release/ham-web-tools`（`icons` / `i18n-pack` /
   `postbuild` 都用它），之后才调构建脚本。
3. **脚本内部一次 `cargo build --profile wasm-release --target wasm32-unknown-unknown`
   编全部 codec**（`-p` 由清单拼出），再逐个 `wasm-bindgen`；profile 必须是
   `wasm-release` 而不是 `--release`（wasm 产物路径
   `target/wasm32-unknown-unknown/wasm-release/…` 与体积都按它定）。脚本开头会 `cd` 到仓库根，
   所以从镜像里的 `/src` 直接调即可。
4. **必须排在 `trunk build` 之前**：`index.html` 的 `copy-dir` 在 `trunk build` 时把
   `public/<codec>-worker/` 拷进 `dist/`，晚一步拷到的就是空目录（页面
   `worker-load-failed`），而 `docker build` 不经过 `cargo make check` 的门禁。
5. **末尾用 `test -s` 断言 `dist/<codec>-worker/<codec>_worker.js` 与 `_bg.wasm`**，
   断言也按同一份清单循环。记得**先校验清单非空**（`workers="$(sh scripts/decode-workers.sh)"`
   后 `test -n`）：脚本若因目录移位返回空清单，空的 for 循环会让断言静默通过，镜像构建成功
   但一个 worker 都没有。

新增 / 删除 codec 时 **Dockerfile 不用改**；仍需手工加的是 `crates/app/index.html` 的
`copy-dir` 与 `public/decode-worker.js` 的 `CODECS` —— 两处都是显式路径，脚本扫描替代不了。

## 前端侧桥（`crates/app/src/decode_session.rs` + `decode_worker.rs`）

**页面只用 `crate::decode_session::DecodeSession`**：十一个解码页（APT / SSTV / WSPR / RTTY /
Olivia / DominoEX / NAVTEX / LRPT / FT8 / Feld Hell / WEFAX）已全部迁移，`crates/app` 里不再有手写
`generation` / 超时 / terminate 的解码页。页面只需要三件事：

1. 在**组件体**（有 Owner）里 `DecodeSession::new(DecodeConfig { … })` —— 内部的
   `onmessage` / `onerror` 闭包、超时槽位与卸载清理都挂在当前 Owner 上；出了组件体调用，
   这些闭包会立即释放、`Function` 悬空。上传区用 `ui::FileDropZone` 的 `on_file`
   （`Callback<File>`）接到 `session.start`，`AptUploader` 已删除，不要再造一份。
2. 给 `DecodeConfig`：`codec`、`max_bytes`、`transfer`、`parse`（各页的 `parse_message`，
   在 `pages/<codec>_decoder/<codec>_worker.rs`）、三条文案回调 `too_large` / `timed_out`
   / `failed`（词条字面量留在各页调用点，`check-i18n` 只采集直接实参）。
3. 在 `session.start(file, move || …)` 的回调里构造 `params` 并 `Ok(params.into())`；
   回调在**文件读完之后**才求值（读取期间改的输入框也算），返回 `Err(文案)` 就直接报错、
   不发 Worker。结果字段用 `decode_worker::{get_u32, get_f64, get_str, get_bool, get_u8s}`
   读，缺字段按错误处理，不要再 `unwrap_or_default()`。

`DecodeSession` 已经包住的编排：每次选新文件先 `terminate` 旧 Worker 再 spawn 新的
（Worker 无法取消在途解码）、代际计数丢弃过期回复、文件读取 `await` 之后先查代际 / 是否
已卸载、超时槽位与 300 秒兜底（`DECODE_TIMEOUT_SECS`）、`processing` / `reading` /
`error` / `file_name` / `result` 五个信号、`cancel` / `reset`。LRPT 走
`transfer: true`（读成 `ArrayBuffer` 直接转移，避免 128 MB 级录音在主线程多拷一份），
其余八页 `transfer: false`。

底层函数（`DecodeSession` 内部使用；只有确实不走会话的场合才直接调）：

- `spawn_worker(&onmessage, &onerror)` —— 建 `/decode-worker.js` 的 Worker（同时挂 `onerror`，见上）；
- `shutdown_worker(&worker)` —— 先摘 `onmessage` / `onerror` 再 `terminate()`（队列里已排的消息不会再进已释放的 Rust 闭包）；
- `retain_message_handler(closure)` / `retain_error_handler(closure)` —— 把消息 / 错误闭包的所有权交给组件，
  卸载时释放，**必须在组件体内调用**；`error_handler(&onmessage)` 负责合成那条错误回信；
- `read_file_bytes(&File)` / `read_file_buffer(&File)` —— 文件 → 字节 / `ArrayBuffer`（异步）；
- `post_decode(&worker, codec, &bytes, &params)` / `post_decode_transfer(…)` —— 组信封发送（后者转移 buffer，大录音用）；
- `decode_timeout_slot(processing)` / `arm_decode_timeout_with(…)` —— 超时兜底槽位与定时器；
- `parse_outcome(&data)` —— 拆 `{ ok, result?, error? }` 并按码本地化，页面再按自己字段解析
  `result`（每页一个 `parse_message` 小函数，见各 `pages/<codec>_decoder/`）。

生命周期铁律：`DecodeSession` 必须在组件体内创建；每个批次结束（完成 / 取消 / 替换 /
卸载）都要让超时句柄失效；文件读取 `await` 之后不得碰已释放的 `StoredValue`（代际检查
已经在内置路径里，绕过去手写就会踩）。

NAVTEX 的 `accepted` 是完整标准报头、CR/LF 与 NNNN 后的残余字符错误率门限（仓内
4% 质量政策）。硬判决实现按 LSB 位序、五字符 DX/RX 延迟处理；真实弱录音的时钟漂移、
软判决及完整正文恢复不由合成测试保证。

## 新增一个解码器的清单

1. **核心 crate**（纯 Rust + 单测）：解码函数签名从 WAV 字节或样本起；错误类型给一个
   `code()` 返回英文码（照 `ham_web_apt::Error`），供 worker 直接回传。
2. **worker crate**：上述导出契约 + 目录 `crates/<codec>-worker/`。
3. **构建 / 检查链路（大部分自动）** —— codec 清单来自 `crates/*-worker` 目录本身，
   `scripts/decode-workers.sh` 是唯一的读取方，所以下列位置**都不必逐个加 codec**：
   - worker 构建：`[tasks.build-workers]` 无参调用 `scripts/build-decode-worker.sh`，
     `dev` / `dev-full` / `build-web` 都只依赖它；`clippy` / `test` 的循环也走清单脚本
     （clippy 同时检查 `ham-web-<codec>` 与 wasm 目标下的 `ham-web-<codec>-worker`）；
   - `Dockerfile`：同样调无参脚本，镜像里没有名单；
   - **仍需手工**：`[tasks.build-<codec>-worker]`（只有想本地单独重建某个 codec 时才加，
     `description` 写清 codec，`script = "sh scripts/build-decode-worker.sh <codec>"`；
     APT 的历史任务名是 `[tasks.build-worker]`，别照它起名）；
     需要命令行自检时加 `[tasks.<codec>-sample]` / `[tasks.<codec>-decode]` /
     `[tasks.<codec>-test]`（`<codec>-test` 依赖前两者），并把 `"<codec>-sample"` 加进
     `[tasks.samples]` 的 `dependencies` 与它的 description；
   - **根 `Cargo.toml`** 的 `members` 加两个新 crate（需要被 `workspace.dependencies`
     引用时一并加）。
   验一遍：`sh scripts/decode-workers.sh` 能列出新 codec（列不出就是目录 / 包名不合约定，
   脚本会直接报错），`cargo make build-workers` 能过，
   `ls public/<codec>-worker/` 有 `*.js` 与 `*_bg.wasm`。
4. **`crates/app/index.html`**：加一行
   `<link data-trunk rel="copy-dir" href="../../public/<codec>-worker" />`（漏了 → `trunk`
   不把 wasm 拷进产物，页面 `worker-load-failed`）。
5. **`public/decode-worker.js`**：`CODECS` 表加一项（两文件路径），同时把文件头注释里的
   codec 清单补上。
6. **页面** `crates/app/src/pages/<codec>_decoder/`：`mod.rs` + `_page.rs` +
   `_worker.rs`（`parse_message`），照任一既有页面（如 `rtty_decoder`）复制结构；
   页面主体是 `DecodeSession::new(...)` + `ui::FileDropZone`，不手写 Worker 生命周期。
7. **登记**：`pages/mod.rs`、`app/main_content.rs` 的路由、`core/registry.rs`
   的 `Module`（`GROUP_TOOL_CALC` 分组，导航自动收纳）。
8. **i18n**：zh/en/es 三份 `tools.json` 的键（`cargo make i18n-pack` 校验
   覆盖率 100%）；新错误码要同时进 `localize_error` 与三份词条。
9. **e2e**：`e2e/tests/<codec>_decoder.spec.ts` —— 用**独立 TS 实现**编码合成
   WAV（延续「三套实现互锁」的约定），上传断言解码结果；坏样本如实报错。另外两处同步：
   `e2e/tests/decode_worker_protocol.spec.ts` 的 `CODECS` 数组加新 codec（它逐 codec 发空
   WAV，断言回的是协议错误码，能证明 `CODECS` 表里的路径都加载得起来；该数组与
   `decode-worker.js` 的 `CODECS` 表、本文件的 codec 表一一对应）；如果用例数变了，
   `e2e/coverage-targets.json` 的 `journeys` 里那条 `grep` 也要跟着改（只是标题登记，
   不是 spec 路径）。
10. **许可证**（解码器若转写自第三方代码或带第三方录音样本）：许可全文放 `LICENSES/`
    （首段写明适用范围，格式照 `MIT-ft8_lib.txt`）；在 `license_notice.rs` 加链接与「组件来源」
    条目，补三份 `shell.software-license-*` 词条；`Dockerfile` 末尾产物断言加上
    `dist/licenses/<文件名>`；`e2e/tests/navigation.spec.ts` 的许可页用例把新链接加进路径列表。
    MIT / BSD 都要求再分发时保留版权与许可声明，漏掉就是合规缺口（ft8 一批就漏过一次）。

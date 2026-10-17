# 审查记录：FT8 解码器 / LoRa / Radiogram / 国内专题一批

这一批改动（`crates/ft8`、`crates/ft8-worker`、`crates/core` 的 lora / radiogram /
china_ham / modern_digital_modes、对应页面与 e2e）在提交前的审查结论与修复进度。

- 审查范围：本地未提交的全部变更（72 个文件，+6821 行）。
- 审查基线：`cargo clippy`（app / core / ft8 / ft8-worker）、`cargo fmt --check`、
  `cargo test -p ham-web-ft8`（19 条）、`check-i18n`、`check-ui`、
  `knowledge-i18n-check` 全部通过；未发现语法错误、注入类安全问题或 UI 规范硬违规。
- 状态标记：`待修` / `已修` / `保留`（评估后不改，附原因）。

## 一、高：功能缺陷（FT8 消息层）

| # | 问题 | 位置 | 状态 |
|---|---|---|---|
| 1 | type-4（非标准呼号）消息的 12 位哈希呼号永远解不出：`encode_nonstd` 只把 `hash >> 10` 当数字用，从未按 12 位键入表；`decode_nonstd` 的 `lookup_callsign(n12)` 必然 miss，该字段恒为 `<...>` | `crates/ft8/src/message.rs`（`HashTable` / `encode_nonstd` / `decode_nonstd`） | 已修 |
| 2 | 带尖括号的呼号无法参与 12 位哈希路径：`callsign_hash` 不跳过 `<`/`>`，`pack58` 只 strip 前导 `<`。`iflip == 1` 分支必然 `Err(Callsign1)`，整体回退自由文本后报 `Err(Type)`，即「解码结果复制再编码」失败 | `crates/ft8/src/message.rs` | 已修 |

## 二、中：正确性 / 健壮性

| # | 问题 | 位置 | 状态 |
|---|---|---|---|
| 3 | 解不出文本的消息仍进入结果列表，页面出现只有频率/SNR 的空白行（`ArrlFd` / `Contesting` / `Wwrof` / `Unknown` 均返回 `Err(Type)`） | `crates/ft8-worker/src/lib.rs` | 已修 |
| 4 | `normalize_logl` 在零方差（静默音频）下 `norm = inf`、`0 * inf = NaN`，白跑 25 轮 BP 迭代 | `crates/ft8/src/decode.rs` | 已修 |
| 5 | NTS Radiogram 生成器无必填校验，空表单也产出一份以空格开头的错格式报文 | `crates/app/src/pages/tools/radiogram_generator.rs` | 已修 |
| 6 | wasm 热路径每帧堆分配（`timedata`/`freqdata`/`packed`/`tmp`），一个 FT8 时隙约 158 次 FFT | `crates/ft8/src/monitor.rs`、`crates/ft8/src/fft.rs` | 已修 |
| 7 | 候选选取缺少同域抑制，同一信号的相邻 `freq_sub`/`time_sub` 会吃满候选名额，弱信号被挤出 | `crates/ft8/src/decode.rs` | 已修 |

## 三、低：一致性 / 卫生 / 数据

| # | 问题 | 位置 | 状态 |
|---|---|---|---|
| 8 | `extract_likelihood` 里 `sym_skip` 计算后 `let _ = sym_skip;`（死变量） | `crates/ft8/src/decode.rs` | 已修 |
| 9 | 模块文档与实现不符：仍写「解码器随后补齐」，实际 `decode.rs` / `monitor.rs` 已完整实现 | `crates/ft8/src/lib.rs` | 已修 |
| 10 | `ham-web-apt` 只被 example 使用却放在 `[dependencies]`（wasm 目标也会链接）；`description` 写「无外部依赖」不符 | `crates/ft8/Cargo.toml` | 已修 |
| 11 | 若干 `pub` API 缺输入校验会 panic：`compute_crc` 的 `num_bits` 越界、`Monitor::process` 的 `frame.len()`、`synth_gfsk` 空 `symbols`、`Fft::new(0)`、`Monitor::new` 的 `f_max` 过高导致 bin 越界 | `crc.rs` / `monitor.rs` / `audio.rs` / `fft.rs` | 已修 |
| 12 | worker 去重顺序：`seen.insert` 在 `decode_message` 之前，首个同 payload 解码失败会屏蔽后续 | `crates/ft8-worker/src/lib.rs` | 已修 |
| 13 | 数据精度：409 MHz 公众对讲机写「409.75–409.99 MHz（12.5 kHz 间隔 20 个信道）」，20 个信道上界应为 409.9875 | `crates/core/src/china_ham.rs` | 已修 |
| 14 | 上传接受类型与解码器能力不符：`accept` 含 `audio/*`，但 worker 只走 `parse_wav` | `crates/app/src/pages/ft8_decoder/ft8_decoder_page.rs` | 已修 |
| 15 | 风格：`Ics213::format` 用 `push_str(&format!(...))`；Radiogram 选项表用 `StoredValue<Vec<_>>` + `get_value()` | `crates/core/src/radiogram.rs`、`radiogram_generator.rs` | 已修（选项表保留，见下） |
| 16 | e2e 用 `input[type="file"]` 硬编码选择器，未走 role/id 契约 | `e2e/tests/ft8_decoder.spec.ts` | 已修 |

### 第 15 条的两点说明

- `push_str(&format!(...))` 改为 `writeln!`，省掉每行一次中间 `String`。
- 选项表**保留** `StoredValue`：该闭包只在格式（NTS / ICS-213）切换时重跑，`get_value()`
  的 clone 成本低于每次重建 8 个 `SelectOption`（每个都要走一次 `t()`），改成闭包内构造是负收益。

## 四、各条的修复要点

| # | 改成了什么 |
|---|---|
| 1 | `HashTable` 拆成 22 位表 + 12 位表；`encode_nonstd` 按 12 位键存表，`decode_nonstd` 按 12 位键查。新增 `nonstd_type4_resolves_the_12bit_side` 单测。 |
| 2 | `callsign_hash` / `pack58` 统一跳过 `<` `>` 且不占 11 字符名额；新增 `strip_angle` 供存表用。新增 `angle_brackets_are_ignored_when_encoding` 单测。 |
| 3 | worker 过滤空文本、改为先解码再判重，并回传 `unsupported` 计数；页面在元信息下提示条数（新词条 `tools.ft8-unsupported`，三语）。 |
| 4 | `normalize_logl` 返回 `bool`，方差非正或非有限时让 `decode_candidate` 直接放弃该候选。新增 `silent_audio_decodes_nothing` 单测。 |
| 5 | 生成器按格式维护必填清单，缺字段时输出区改为提示（`tools.radiogram-missing-fields`）并禁用复制。新增 e2e 覆盖缺字段路径。 |
| 6 | `Monitor` 复用 `timedata` / `freqdata`，`RealFft` 持有 `packed` / `tmp`；逐子帧不再分配。 |
| 7 | `find_candidates` 排序后按「同 `freq_sub`/`time_sub` 且频/时偏移差 ≤ 1」贪心去重，名额分给更多不同信号。 |
| 8 | 删除 `sym_skip`（连同解构里无用的第二项）。 |
| 9 | `lib.rs` 模块文档改为描述现状（编解码 + waterfall + 音频合成）与范围限制。 |
| 10 | `ham-web-apt` 移到 `[dev-dependencies]`，`description` 说明修正。 |
| 11 | `compute_crc` → `Option`；`Monitor::process` → `bool`（长度不符返回 `false`）；`synth_gfsk` 空符号/退化采样返回空 `Vec`；`synthesize` 长度不足返回 `None`；`Fft::new(0)` / 实数 FFT 长度不符显式拒绝；`Monitor::new` 按实数 FFT 能力裁剪 `max_bin`；`decode_candidate` 增加 `candidate_in_bounds`。新增 `out_of_bounds_candidate_is_refused`、`short_frame_is_rejected` 单测。 |
| 12 | 见第 3 条（同一次改动）。 |
| 13 | 频段改为 `409.7500–409.9875 MHz`，同步 e2e 与 `data/knowledge-i18n/{en,es}/china_ham.json` 的 key。 |
| 14 | `accept` 收紧为 `.wav,audio/wav,audio/x-wav`。 |
| 15 | `Ics213::format` 改用 `writeln!`；选项表保留（见上）。 |
| 16 | 页面给 `FileDropZone` 固定 `id="ft8-file"`，e2e 改用该 id 定位。 |

## 五、验证结果

- `cargo make check`：**通过**（fmt-check / clippy / test / i18n-check / knowledge-i18n-check /
  explanations-check / check-ui / pixel-check）。
- `cargo test -p ham-web-ft8`：**24 条通过**（新增 5 条：type-4 往返、尖括号输入、静默音频、
  越界候选、短帧拒绝）。
- 真实录音回归：`cargo run --release -p ham-web-ft8 --example decode_sample --
  crates/ft8/samples/ft8_20m_busy.wav` → **解出 14 条**（README 记录的修复前为约 13 条），
  候选去重没有减少检出。
- e2e（dev 构建 / `--workers=1`）：`ft8_decoder.spec.ts`、`radiogram.spec.ts`（4 条）、
  `china_ham.spec.ts`、`lora.spec.ts` 共 13 条**全过**。
- 词条：`data/i18n/{zh,en,es}/tools.json` 各新增 2 条，`cargo make i18n-pack` 已重生成，
  `check-i18n` 三语 4237 条一致。

## 六、遗留（本次未改）

- FT4 的同步 / 解调仍没有真实信号验证（`ft8_lib` 无 FT4 录音），只有编码方向对拍。
- 竞赛 / ARRL Field Day / 遥测消息只识别类型、不解析字段（现在会以 `unsupported` 计数提示，
  不再产生空行）。

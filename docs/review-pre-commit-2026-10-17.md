# 审查记录：提交前审查（2026-10-17）

对本地未提交的全部变更做的一次提交前审查（代码层 + 构建 / 发布层 + 文案 / 许可层），
本文件追踪每一项的**状态与修复要点**。更早两轮见 [`review-ft8-batch.md`](review-ft8-batch.md)、
[`review-followup-2026-10-17.md`](review-followup-2026-10-17.md)。

状态标记：`已修` / `保留`（评估后不改，附原因）。

- **审查范围**：暂存区 73 个文件（+7288/−54）与 6 个未暂存文件（FT8 解码页、`message.rs`、
  `monitor.rs`、三语 `tools.json`），内容是 FT8/FT4 解码内核、LoRa 计算器、NTS Radiogram
  生成器与 3 个知识页。
- **审查基线**（修复前）：`cargo clippy --workspace --all-targets -D warnings` 通过；
  `cargo test -p ham-web-ft8 -p ham-web-core` 通过；`i18n-check` / `knowledge-i18n-check` 通过；
  安全方面（注入 / XSS / 路径 / 体积上限）未发现问题。
- **进度**：20 项原始问题 + 3 项修复中新发现的问题，**全部已修**（#20 按评估保留）。

## 一、严重

| # | 问题 | 位置 | 状态 |
|---|---|---|---|
| 1 | `Dockerfile` 没有构建 `ft8-worker`：`index.html` 已 `copy-dir public/ft8-worker`，而该目录被 `.gitignore` 排除、只能由构建生成，镜像构建会在 `trunk build` 失败；就算没失败，`/ft8-decode` 也缺 wasm，现有 `test -s` 断言又不含 ft8 | `Dockerfile` 的 `-p` 列表、`for w in` 循环、产物断言 | 已修 |
| 2 | 转写的第三方代码没有随发布物提供许可全文：`fft.rs` 来自 KISS FFT（BSD-3-Clause），消息 / CRC / LDPC / 同步 / waterfall 与样本录音来自 ft8_lib（MIT）。仓库对 lrpt / olivia 的既有做法是 `LICENSES/` + 许可页登记 + Dockerfile 断言，这次三者皆缺 | `LICENSES/`、`license_notice.rs`、`Dockerfile`、三语 `shell.json` | 已修 |

## 二、警告

| # | 问题 | 位置 | 状态 |
|---|---|---|---|
| 3 | LoRa「低速率优化」自动档按 `SF ≥ 11` 判断；Semtech 的规则是**符号时间 > 16 ms**，与带宽有关（SF12@500 kHz 只有 8.2 ms 不该开，SF10@62.5 kHz 有 16.4 ms 该开）。e2e 期望值沿用同一条简化规则，测不出 | `lora_calculator.rs`、`core/src/lora.rs` | 已修 |
| 4 | 最窄带宽「62 kHz」实为 62.5 kHz，符号时间 / 灵敏度偏约 0.8%；知识页本来就写 62.5–500 kHz，页面与内核不一致 | `core/src/lora.rs`、`lora_calculator.rs` | 已修 |
| 5 | 超过一个时隙的录音在第一个时隙收满后被静默丢弃（`process` 返回 `false` 后 `break`），FT4 的 15 s 文件只解前半段，没有任何提示 | `ft8-worker/src/lib.rs`、`ft8/src/monitor.rs` | 已修 |
| 6 | 界面把 `score × 0.5` 标成「SNR dB」；它是 Costas 同步峰高出邻频 / 邻符号的平均幅度，不是 WSJT-X 口径的 SNR | `ft8-worker/src/lib.rs`、`tools.ft8-message-meta` | 已修 |
| 7 | 暂存区与工作区分裂：6 个文件有未暂存的修复，直接 `git commit` 会漏掉 | `git status` 中的 `AM` / `MM` | 已修 |
| 8 | NTS 报文格式与 ARRL 约定不符：正文被加了行首 `X ` 前缀、签名被加了 `= ` 前缀；实际是正文前后各一个 `BT`（语音 `BREAK`）、句号写字母 `X`（算 1 组、不能是最后一组）。另外紧急优先级必须在报头拼写全称 `EMERGENCY`，不写 `E` | `core/src/radiogram.rs`、`radiogram_knowledge.rs`、知识库译文 | 已修 |

## 三、建议

| # | 问题 | 位置 | 状态 |
|---|---|---|---|
| 9 | 「不支持的消息类型」口径与实际不符：注释和文案写「竞赛 / Field Day / 遥测」，但遥测会解成十六进制文本显示；实际计入的是 DXpedition、EU VHF、Field Day、RTTY Roundup、WWROF 等 | `ft8-worker`、`ft8/src/lib.rs`、`tools.ft8-unsupported` | 已修 |
| 10 | 呼号哈希表按候选顺序边解边填，type 4 里的 `<...>` 能否还原取决于顺序 | `ft8-worker/src/lib.rs` | 已修 |
| 11 | `compute_crc(...).unwrap_or(0)` 把错误吞成 0，与「提取出的 CRC 恰为 0」撞上会把垃圾当有效消息 | `ft8/src/decode.rs` | 已修 |
| 12 | 同一信号 `get()` 两次再判空 / 取值 | `radiogram_generator.rs` | 已修 |
| 13 | 事件回调里用跟踪式 `output.get()` | `radiogram_generator.rs` | 已修 |
| 14 | `t(k)` 的实参是变量，`check-i18n` 采集不到 | `radiogram_generator.rs` | 已修 |
| 15 | 有效载荷 / 前导码输入 `12.5` 被 `round()` 后计算，输入框仍显示 `12.5`，显示与计算不一致 | `lora_calculator.rs` | 已修 |
| 16 | SF / 带宽 / 编码率选项在页面里又写了一遍，没复用内核常量 | `lora_calculator.rs` | 已修 |
| 17 | 链路预算闭包里的 `t` 遮蔽了 `i18n::t` | `lora_calculator.rs` | 已修 |
| 18 | 九个 `NumberField` 都没传 `invalid`，非法输入时控件本身不标红、没有 `aria-invalid` | `lora_calculator.rs` | 已修 |
| 19 | 「八个 Worker」的注释 / 文档漂移（实际九个）；协议文档缺 `ft8` codec 与 `ft8-sample-rate` 错误码 | `Dockerfile`、`.gitignore`、`Makefile.toml`、`Trunk.toml`、`decode_worker.rs`、`build-decode-worker.sh`、`docs/decode-worker-protocol.md`、`README.md` | 已修 |
| 20 | 过程性审查记录入库 | `docs/review-*.md` | 保留 |

## 四、修复过程中新发现的问题

| # | 问题 | 位置 | 状态 |
|---|---|---|---|
| N1 | **真实接收路径下 type 4 的 12 位呼号永远还原不出**：上一轮把 12 位与 22 位哈希表分开存，但 12 位表只在**编码**时填；解码标准消息只写 22 位表。原单测是「同一张表先编码再解码」，所以没发现。真实录音里 `LZ365BM <...> 73` 这类就是这个原因 | `ft8/src/message.rs`（`save_callsign`） | 已修 |
| N2 | LDRO 自动档的界面选项文案「自动（SF ≥ 11）」与知识页「SF11 / SF12 建议开启」沿用了旧口径 | 三语 `tools.json`、`lora_knowledge.rs`、知识库译文 | 已修 |
| N3 | 许可页新增条目后，新增解码器的清单里没有「许可证」这一步，容易再漏 | `docs/decode-worker-protocol.md` 清单第 10 条 | 已修 |

## 各条修复要点

| # | 改成了什么 |
|---|---|
| 1 | Dockerfile：`-p ham-web-ft8-worker`、`for w in … ft8`、产物断言加 `dist/ft8-worker/ft8_worker.js` 与 `_bg.wasm`。 |
| 2 | 新增 `LICENSES/MIT-ft8_lib.txt`、`LICENSES/BSD-3-Clause-kissfft.txt`（全文取自上游仓库，首段写明适用范围）；`license_notice.rs` 加两个许可链接与一条「组件来源」；三语 `shell.json` 加 `software-license-mit-ft8-lib` / `-bsd-kissfft` / `-ft8`；Dockerfile 断言加两个文件；README「许可」章节登记；`navigation.spec.ts` 许可页用例把两条新链接纳入「可访问且是许可全文」断言。 |
| 3 | 内核新增 `low_data_rate_optimize_auto(sf, bw_hz)`（符号时间 > 16 ms）；页面自动档改调它；单测覆盖 125 / 250 / 500 / 62.5 kHz 的分界。选项文案改为「自动（符号时间 > 16 ms）」。 |
| 4 | `BW_KHZ_CHOICES` 改为 `BW_HZ_CHOICES = [62_500, 125_000, 250_000, 500_000]`；页面带宽信号以 Hz 存，标签显示 `62.5 kHz`；单测钉住 62.5 kHz 的符号时间与灵敏度。 |
| 5 | 解码流程从 Worker 挪进内核 `ham_web_ft8::decode_recording`（可原生单测）：录音按时隙长度（FT8 15 s / FT4 7.5 s）切片，**每个时隙新建 `Monitor`**（滑窗里留着上一时隙的尾巴，不清会污染开头几个符号）；`time_sec` 加上时隙起点；去重只在**时隙内**做（同一台站下个时隙重复同样的话是两条真实接收）；结果带 `slots`，页面在 `slots > 1` 时显示「共 N 个时隙」。 |
| 6 | 字段 `snr` 改名 `sync_db`，文案改为「同步强度 {} dB」；代码注释写明它不是 WSJT-X 口径的 SNR；README 与协议文档同步。 |
| 7 | 提交前 `git add -u` 并补加新文件，使暂存区与工作区一致（只暂存，未提交）。 |
| 8 | 依据见下文「口径说明」。`format()` 改为：地址 → `BT` → 正文原样 → `BT` → 签名（无 `X ` / `= ` 前缀）；`Precedence::code()` 返回 `&'static str`，紧急为 `EMERGENCY`；知识页概念 / 字段表 / 要点与 en / es 译文同步（知识库译文按中文原文做 key，三处同步改 key）；`count_groups` 文档改写（`X` 与拼出的标点词各算 1 组，BT 不计）。 |
| 9 | 注释与三语文案改为「DXpedition / 欧洲 VHF / 竞赛 / Field Day 等」，`ft8/src/lib.rs` 的范围说明改为「标准呼号、非标准呼号、自由文本、遥测（十六进制）四类解析」。 |
| 10 | `decode_recording` 分三遍：逐时隙解出 payload → 全量过一遍只为填哈希表 → 再出文本，type 4 与标准消息的先后顺序不再影响结果；单测覆盖两种顺序。 |
| 11 | 改为 `compute_crc(...)?`，长度不符时放弃该候选。 |
| 12 | 先把 `time` / `phone` 各读一次到局部变量，再判空取值。 |
| 13 | 复制回调里改 `output.get_untracked()`。 |
| 14 | 缺失字段改用枚举 `Missing`，每个分支里 `t("…")` 的字面量写在调用点。 |
| 15 | 内核新增 `whole_u16`（只收 0–65535 的整数）；`12.5` 判为非法、结果区给出「请检查输入」。 |
| 16 | SF / 带宽 / 编码率选项分别取自 `SF_RANGE`、`BW_HZ_CHOICES`、`CR_RANGE` + `coding_rate()`；合法范围判断用 `SF_RANGE` / `PL_RANGE` / `PREAMBLE_RANGE`。 |
| 17 | 重命名为 `tx`。 |
| 18 | 有效载荷 / 前导码 / 频率 / 距离 / 噪声系数 / 功率 / 增益 / 附加损耗都传 `invalid`，非法时 `aria-invalid="true"` 并转 `destructive` 描边。 |
| 19 | 「八个」统一改「九个」；协议文档补 `ft8` codec 行（params 与 result 形态）与 `ft8-sample-rate:<Hz>` 错误码。 |
| 20 | 保留：按日期命名、随提交入库，是现有两份记录的延续；不放进 `AGENTS.md` 的入口索引（那里只放「改代码前要读的规范」）。 |
| N1 | `save_callsign` 同时写 22 位与 12 位两张表（12 位键 = 22 位哈希右移 10 位），删除只在编码路径用的 `save_callsign12`；新增单测：接收端哈希表里只有「解码标准消息」留下的条目，也能还原 type 4 的 12 位呼号。 |
| N2 | 三语词条与知识页文案改为「符号时间 > 16 ms 时建议开启（125 kHz 下即 SF11 / SF12）」。 |
| N3 | 清单第 10 条：许可全文、许可页登记、三语词条、Dockerfile 断言、e2e 许可页路径列表。 |

## 口径说明

- **#8 的依据**：ARRL《Public Service Communications Manual》对报文格式的约定，经
  Wikipedia「ARRL Radiogram」条目概括（2026-10-17 取得）：正文与前后的地址 / 签名用 `BT`
  （CW）或 `BREAK`（语音）分隔、不计入检查；`X` 代表句号，本身算一个组、永远不是正文最后一组；
  其它标点拼成单词并各算一组；紧急优先级在报头里永远拼写全称。**未取到 ARRL 官方 PDF 原文**，
  所以这条是「与二手概括一致」，不等于「与手册原文逐字核对」。
- **多时隙解码的前提**：录音须从时隙边界附近起录。候选搜索只容忍起点偏差约 −1.6 s～+3.2 s
  （FT8），偏得更多的信号会跨在两个时隙之间而解不出。
- **仍未解决**：FT4 的解调没有真实信号验证（`ft8_lib` 没有 FT4 真实录音样本）；
  同步强度只用于排序和粗判，不是 SNR 估计。

## 验证结果

- `cargo make check`：**通过**（fmt-check / clippy / test / i18n-check / knowledge-i18n-check /
  check-explanations / check-ui / pixel-check）。
- 单测：`ham-web-ft8` 32 条（修复前 24，新增 `recording` 7 条 + 12 位哈希 1 条）、
  `ham-web-core` 1173 条（含新增的 LoRa LDRO / 62.5 kHz / 整数校验，以及 Radiogram 的 `BT` 与
  `EMERGENCY` 用例），全部通过。
- 真实录音：`cargo run -p ham-web-ft8 --release --example decode_sample -- crates/ft8/samples/ft8_20m_busy.wav`
  仍解出 14 条，与改动前一致（单时隙录音走同一条路径）。
- e2e（`trunk serve` 的 dev 产物，`--workers=1`）：`ft8_decoder`（含新增的两时隙用例）、`lora`（含新增
  的 LDRO / 62.5 kHz 与整数校验用例）、`radiogram`（含新增的 `EMERGENCY` 用例）、`navigation` 的
  许可页用例，全部通过；`mobile_overflow`（175 个路由 × 中 / 英 / 西三语，320 / 390px）通过，
  覆盖 LoRa 带宽选项变长与许可页新增条目后的移动端布局。
- 没跑：`docker build`（Dockerfile 的改动按静态核对）、release 口径的整批 e2e（`cargo make e2e`）。

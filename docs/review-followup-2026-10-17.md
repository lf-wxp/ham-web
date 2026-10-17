# 审查记录：未提交变更复检（2026-10-17）

对 `review-ft8-batch.md` 那一轮修复后的代码再做一次复检，聚焦本轮新引入/遗留的少量问题。
状态标记：`已修` / `保留`（评估后不改，附原因）。

- 审查范围：本地未提交的全部变更（73 个文件，+7288/−54）。
- 整体结论：无崩溃、数据错误或安全问题；注释写「为什么」、单测对拍充分、越界保护到位。

## 问题清单与状态

| # | 严重度 | 问题 | 位置 | 状态 |
|---|---|---|---|---|
| 1 | 中 | FT8 解码页「文件过大」提示复用了降采样文案（建议「降采样到 8–16 kHz」），但 FT8 严格要求 12000 Hz，降采样必然触发 `ft8-sample-rate` 错误，提示自相矛盾 | `crates/app/src/pages/ft8_decoder/ft8_decoder_page.rs:31` | 已修 |
| 2 | 低 | LoRa 计算器的比特率是「有效载荷吞吐」（未扣前导码/报头/CRC 开销） | `crates/app/src/pages/tools/lora_calculator.rs:136` | 保留 |
| 3 | 低 | `message.rs` 字节长度与字符迭代混用，非 ASCII 呼号/文本可能错位 | `crates/ft8/src/message.rs` | 已修 |
| 4 | 提示 | `Monitor::new` 的 `num_bins > 0` 用 `debug_assert!`，与同函数其它参数校验强度不一致 | `crates/ft8/src/monitor.rs:142` | 已修 |

## 各条修复要点

| # | 改成了什么 |
|---|---|
| 1 | 新增词条 `tools.ft8-file-too-large`（zh/en/es 三语），文案改为「改用更短的 15 秒录音或压缩文件（须保持 12000 Hz 采样率）」；`too_large` 回调改引用该词条。 |
| 2 | 确认 `tools.lora-toa-result` 词条已是「…/ 有效数据速率 {} bps」，「有效数据速率」已准确限定为有效载荷吞吐，不改。 |
| 3 | `pack_basecall` 入口加 `!callsign.is_ascii()` 直接返回 `None`，并在文档注释声明 ASCII 约束；`encode_free` 的长度上限由 `text.len()`（字节）改为 `text.chars().count()`（字符），消除非 ASCII 下的提前误判。 |
| 4 | `num_bins > 0` 的 `debug_assert!` 改为 `assert!`，release 下同样生效，与上方 `sample_rate`/`time_osr`/`freq_osr` 的校验口径一致。 |

## 验证结果

- `cargo make check`：**通过**（fmt-check / clippy / test / i18n-check /
  knowledge-i18n-check / check-explanations / check-ui / pixel-check）。
- `ham-web-ft8` 单测：**24 条通过**（`message.rs` / `monitor.rs` 的修改未破坏既有对拍与往返用例）。
- 词条：`data/i18n/{zh,en,es}/tools.json` 各新增 1 条 `tools.ft8-file-too-large`，
  `cargo make i18n-pack` 重生成后三语各 4238 条、key 集合一致、无死条目。

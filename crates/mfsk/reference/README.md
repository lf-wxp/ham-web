# 参考实现与来源（互操作对拍用）

本 crate（`ham-web-mfsk`）转写自 **fldigi**（GPL-3.0，<https://github.com/w1hkj/fldigi>，转写时的版本是该镜像的 4.1.23），整个 crate 按 GPL-3.0 发布；
`ham-web-thor` 的转写与验证用的是同一份源码与同一套方法，来源与验证见本文与 `fldigi_web/README.md`。

## 转写了什么

| 本仓文件 | fldigi 出处 | 说明 |
| --- | --- | --- |
| `src/fec.rs` 的 `Encoder` / `Viterbi` | `src/filters/viterbi.cxx` | 卷积编码器与 Viterbi 译码器（改编自 gMFSK，© 2006 Dave Freese W1HKJ） |
| `src/fec.rs` 的 `Interleaver` | `src/mfsk/interleave.cxx` | 卷积交织器（改编自 gMFSK，© 2001–2003 Tomi Manninen OH2BNS） |
| `src/varicode.rs` | `src/mfsk/mfskvaricode.cxx` | IZ8BLY MFSK Varicode（生成物，由 `examples/extract_varicode.rs` 提取并与同文件的 `varidecode[]` 逐项比对） |
| `src/tx.rs` | `src/mfsk/mfsk.cxx` 的 `sendbit` / `sendchar` / `clearbits` / `flushtx` / `tx_process` | 发端帧结构 |
| `src/rx.rs` 的 `Receiver` | `src/mfsk/mfsk.cxx` 的 `harddecode` / `softdecode` / `decodesymbol` / `recvbit` | 判决、软位、两个 Viterbi 的投票 |

接收端的**前端**（中心频率检测、符号对齐、质量门限）不是转写，是离线文件专用的重写：fldigi 用滑动 FFT 逐样本跟踪，这里用 Goertzel 在整段录音上搜一次。

## 怎么验证的

1. **与上游代码逐位对拍**（`tests/upstream_xcheck.rs` + `reference/fldigi_ref.cpp`）：把 fldigi 的 `viterbi.cxx` / `interleave.cxx` / `mfskvaricode.cxx` / `thorvaricode.cxx` **原样编译**，驱动它们生成音调序列，Rust 版必须逐个音调相同（11 个子模式 × 4 段文本，含 64L / 128L 与它们「没有前导零位」的帧结构）；Viterbi 对随机软值逐步比较输出位与路径度量（K = 7 与 K = 15）。

   ```text
   git clone --depth 1 https://github.com/w1hkj/fldigi /tmp/fldigi
   FLDIGI_SRC=/tmp/fldigi cargo make mfsk-xcheck
   ```

   这组用例标了 `#[ignore]`：默认 `cargo test` 里显示为 ignored（不会悄悄「通过」），显式运行（上面的任务已带 `--include-ignored`）时没设 `FLDIGI_SRC` 会直接失败。

2. **真实录音**（`tests/real_recordings.rs`）：gMFSK 官方站点的 MFSK16 / MFSK8 样本与 G4WNC 的 MFSK16 / 32 / 64 样本，逐字解出标准测试报文（`the quick brown fox jumps over the lazy dog 1234567890`）。录音不进仓库，用 `MFSK_REAL_DIR` 指向存放目录；同样是 `#[ignore]` 用例，运行时加 `-- --include-ignored`，目录或文件缺失会失败而不是跳过（只想跑其中一部分就用用例名过滤，如 `-- --include-ignored g4wnc_mfsk16_sample`）。

3. **fldigi 自己的收发端**（`reference/fldigi_web/`）：fldigi 4.2.13 的原版调制解调器（WebAssembly 版）生成的 11 个 MFSK 子模式录音被本 crate 逐字解出，本 crate 生成的音频被 fldigi 的接收机还原 —— 双向都验过，细节与边界见 `fldigi_web/README.md`。

4. **合成往返**（`tests/roundtrip.rs`）：11 个子模式 × 多种采样率 / 中心频率 / 噪声 / 时钟误差。

## THOR 部分（`ham-web-thor`）

`ham-web-thor` 同样转写自 fldigi，整个 crate 按 GPL-3.0 发布。它复用本 crate 的 `fec`（卷积码 / Viterbi / 交织器）、`front`（接收前端）与 `lookup`（Varicode 反查），不再单独写一份。

| 本仓文件 | fldigi 出处 | 说明 |
| --- | --- | --- |
| `thor/src/tx.rs` | `src/thor/thor.cxx` 的 `sendsymbol` / `sendchar` / `Clearbits` / `flushtx` / `tx_process` | 发端帧结构、IFK+ 选音调、二次信道（ID 信标） |
| `thor/src/rx.rs` 的 `Receiver` | `src/thor/thor.cxx` 的 `decodesymbol`（硬位路径）/ `decodePairs` | 音调差还原 nibble、拆成码位软值、Viterbi、按 `001` 切字符 |
| `thor/src/varicode.rs` | `src/thor/thorvaricode.cxx`（© 2008 Dave Freese W1HKJ） | 二次信道 Varicode（生成物，由 `thor/examples/extract_varicode.rs` 提取并与同文件的 `thor_varidecode[]` 逐项比对）；主字符集就是本 crate 的 IZ8BLY MFSK Varicode |
| `mfsk/src/fec.rs` 的 `Code::IEEE_K15` | `src/filters/viterbi.cxx` 的 K = 15 码 | 25 / 25x4 / 50x1 / 50x2 / 100 这五个模式用 |

THOR 的设计 © Nino Porcino IZ8BLY / Murray Greenman ZL1BPU。验证方法与上面 MFSK 的四条一一对应：`thor/tests/upstream_xcheck.rs`（11 个子模式 × 5 段文本，含二次信道）、`thor/tests/real_recordings.rs`（fldigi 4.2.13 自己的发端录音，`THOR_REAL_DIR`）、`thor/tests/roundtrip.rs`（合成往返），以及 `fldigi_web/` 的双向验证。与 fldigi 接收机的差异（硬位判决、平滑曲线而非逐样本的漂移跟踪）见 `thor/src/lib.rs` 的模块文档。

## 一个与 fldigi 源码不一致的发现

fldigi 里 MFSK4 / MFSK8 的交织深度是 5（fldigi 4.2.13 的原版发端生成的录音确实要用 5 才能译出）；但 gMFSK（这个模式的原作者实现）的官方 MFSK8 样本要用 **10** 才能译出原文。所以这两个子模式两种深度都试、按平均路径度量择优（`rx.rs` 的 `depth_candidates` / `depth_wins`），结果里带回实际用的深度。

# 样本与真值（互操作对拍用）

来源：**lrpt-encoder**（MIT License，© 2026 Ashen Chathuranga，
https://github.com/ktauchathuranga/lrpt-encoder ）—— 把图像编码成 Meteor-M N2-4
LRPT 数字信号的独立 Rust 实现，其输出「可被 SatDump 和任何符合标准的 LRPT
解码器解码」（README 原文）。

| 文件 | 内容 | 大小 |
| --- | --- | --- |
| `lrpt_encoder_cadus.bin` | `a.png`（554×554 → 放大 1568×1568 + luma，R=G=B）的**全量** CADU 流（839 帧） | 859 KB |
| `lrpt_encoder_head.wav` | 同一运行的 288 kHz 立体声 I/Q WAV **前 4 秒** | 4.6 MB |
| `bands_cadus.bin` | `truth_bands.png`（1568 宽纯色带，`--rgb` 模式）的 18 帧 CADU 流 | 18 KB |
| `bands.wav` | 同一色带图的 288 kHz 立体声 I/Q WAV（约 2 秒），浏览器 e2e（`e2e/tests/lrpt_decoder.spec.ts`）上传用 | 2.3 MB |
| `truth_bands.png` / `truth_bands.bin` | 色带真值图 / R·G·B 三平面**逐像素真值**（高 64 像素；纯色 → resize 恒等、无 luma 歧义） | 2.8 KB + 294 KB |

以上全部是 lrpt-encoder 的**合成**输出，不是真实过境录音；MIT 版权与许可全文见仓库根
`LICENSES/MIT-lrpt-encoder.txt`（随发布物提供）。`a.png` 的原图出处未记录，其派生物
（`lrpt_encoder_*`）仅用于互操作测试。

`bands.wav` 与 `bands_cadus.bin` 由同一份色带图用 `--rgb` 模式生成，命令与下面的 `a.png`
示例相同，把输入换成 `truth_bands.png` 并去掉 `--dump-cadu-stream` 即得 WAV；
`--dump-cadu-stream` 得 CADU 流。

完整 WAV 约 110 MB（95.46 s），不进仓库 —— 重新生成：

```sh
git clone --depth 1 https://github.com/ktauchathuranga/lrpt-encoder /tmp/lrpt-encoder
cd /tmp/lrpt-encoder && cargo build --release
./target/release/lrpt-encoder a.png -o /tmp/lrpt-full.wav --dump-cadu-stream /tmp/cadus.bin
```

对拍断言（`tests/`）：
- `image_decode.rs`：CADU 流（去扰码）→ RS → VCDU/MPDU → JPEG → 图像 ——
  色带图**逐像素**一致；a.png 解出 1568×1568、R=G=B、内容非退化；
- `real_sample.rs`：288 kHz WAV → 完整接收链 → 与编码器真值逐字节一致。

# 真实录音样本（互操作回归用）

这两份是**真实**的 SSTV 录音（不是本仓库编码器合成的），用来钉住解码器对真实信号的行为。
此前解码器只在「自己的编码器生成的信号」上测试：编码器与解码器共用同一份错误假设
（VIS 起始位写成 300 ms、位极性写反、Robot / Scottie DX 时序错误），自洽测试一直是绿的，
对真实录音却一份也解不出（5 份开源录音全部 `no-vis`）。

| 文件 | 模式 | 来源 | 许可 | 处理 |
| --- | --- | --- | --- | --- |
| `robot36_wikipedia_fr.wav` | Robot 36 | Wikimedia Commons：[French Wikipedia logo in SSTV.flac](https://commons.wikimedia.org/wiki/File:French_Wikipedia_logo_in_SSTV.flac)（用户 Wizly-08，MMSSTV 生成） | **CC0**（公有领域献出，无署名义务） | FLAC → 8 kHz 单声道 8 位 PCM |
| `martin_m1_sunset.wav` | Martin M1 | Wikimedia Commons：[SSTV sunset audio.ogg](https://commons.wikimedia.org/wiki/File:SSTV_sunset_audio.ogg)（用户 **Mysid**） | **CC BY 2.5**（须署名，已在此注明；允许改编，本文件为其格式转换） | Ogg Vorbis → 8 kHz 单声道 8 位 PCM |

署名（CC BY 2.5）：*SSTV sunset audio* by Mysid, via Wikimedia Commons,
<https://creativecommons.org/licenses/by/2.5/>。本仓库对它的**唯一改动**是降采样与量化
（22.05 kHz / 16 位 → 8 kHz / 8 位，对解码结果的平均像素影响约 1 / 255，已实测）。

只选 CC0 与 CC BY：**CC BY-SA** 的样本（同样在 Commons 上有 Robot 36 与 Martin M1 的录音）
会让样本文件带上「相同方式共享」义务，没有收。`colaclanth/sstv` 的 `examples/m1.ogg`（含参考图
`m1.png`）仓库为 GPL-3.0、样本自身许可未单独声明，**只在本地用来标定与对照，没有入库**。

## 它们证明了什么、没证明什么

- 证明：VIS 检测（30 ms 起始位、`1100 Hz = 逻辑 1`、偶校验）、Martin M1 与 Robot 36 的行结构、
  Robot 36 的交替色度、同步脉冲定位与逐行直线拟合，在**真实信号**上成立。
- **没证明**：Martin M2、Scottie S1 / S2 / DX、Robot 72。这些模式**没有找到开源许可的真实录音**；
  它们的时序按规格文本实现，并用整图时长（114 / 58 / 110 / 71 / 269 / 36 / 72 s）与独立编码器
  做了对账，但**没有真实信号验证**。`Mode::edge_evidence` 里它们标为 `Inherited` /
  `Unverified`，解码结果可能有 1 ~ 3 像素的水平偏移。
- 同步下降沿的经验偏移（`Mode::edge_ms`）**只能用真实录音标定**：用本仓库的独立编码器合成
  信号去扫这个常数，Martin M1 在 −1.5 ~ 0 ms、Robot 36 在 +0.1 ~ +1.2 ms 的整段范围内表现都
  一样 —— 合成信号没有真实 FM 解调的边沿过渡，对它不敏感。

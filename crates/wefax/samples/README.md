# 真实信号样本（互操作测试用）

四段真实的短波气象传真录音，来自 **Signal Identification Wiki**（sigidwiki.com）的
[WEFAX](https://www.sigidwiki.com/wiki/WEFAX) 页面：

| 文件 | 来源 | 内容 |
| --- | --- | --- |
| `northwood_intro_8k.wav` | https://www.sigidwiki.com/images/0/0c/WFax_IntroNORTHWOOD.mp3 | 英国 Northwood 台发射开头：约 30 s 的**相位线**（黑底每行一个白脉冲），之后进入图像，120 LPM / IOC 576 |
| `northwood_data_8k.wav` | https://www.sigidwiki.com/images/e/e0/WFaxData.mp3 | Northwood 的一段图像（海岸线与一块黑色标注），没有相位线，120 LPM |
| `fax240_8k.wav` | https://www.sigidwiki.com/images/8/86/Fax240.mp3 | 页面上的 240 LPM 示例，没有相位线 |
| `dwd_chart_8k.wav` | https://www.sigidwiki.com/images/8/81/Weatherfax_%28dat_14_1%29.mp3 的前 50 s | 一幅德国气象局（DWD）地面预报图的开头：相位线 + 图像。**接收机带宽被削窄**：黑 ≈ 1725 Hz、白 ≈ 2095 Hz，不是标称的 1500 / 2300 Hz；发收时钟差约 0.18%（120.22 LPM） |

## 加工方式

原文件是 mp3（11.025 kHz 或 44.1 kHz），用 ffmpeg 重采样为 8000 Hz / 16-bit / 单声道 WAV，未做其他处理：

```text
ffmpeg -i 原文件.mp3 -ar 8000 -ac 1 -c:a pcm_s16le 目标.wav
ffmpeg -i Weatherfax_(dat_14_1).mp3 -t 50 -ar 8000 -ac 1 -c:a pcm_s16le dwd_chart_8k.wav   # 只取前 50 s
```

## 怎么验证的（以及验证不了什么）

页面只给了录音，**没有给出解码后的图像**（页面上的缩略图是瀑布图）。所以这些样本验证的是
**结构**，不是逐像素对错：

- 相位线被找到、条数合理，黑白电平与行周期的实测值落在预期里（`tests/real_samples.rs`）；
- 解出的图能**人眼读懂**：DWD 那段是一幅带「DEUTSCHER WETTERDIENST … STD PROG BODEN FUER」文字、
  等压线与锋面的地面预报图，竖线竖直（斜率修正到位）；Northwood 那段是地中海一带的海岸线。
  这条只在开发时人工看过，没有自动化。

## 许可

- SigidWiki 页面**未对样本标注单独的许可条款**。
- 样本仅用于本仓解码器的**互操作测试**，不随前端分发、不进入构建产物。若样本权利人提出要求，
  删除并换用合成样本即可——核心的合成测试自足，样本测试是独立的一个文件。

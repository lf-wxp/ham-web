# 真实信号样本（互操作测试用）

两段 SITOR-B / NAVTEX 真实录音，来自 **Signal Identification Wiki**（sigidwiki.com）：

| 文件 | 来源 | 内容（页面标注的解码输出） |
| --- | --- | --- |
| `sitorb_8k.wav` | https://www.sigidwiki.com/images/9/96/SITOR-B.mp3 | 测试字符（RY 测试、「the quick brown fox…」、数字序列）+ 一份 NAVTEX 海岸警告（**MONDOLFO RADIO**，COASTAL WARNING 012/12 JAN 12） |
| `sitorb_tfc_8k.wav` | https://www.sigidwiki.com/images/7/7f/SITOR-B_tfc.mp3 | 相位段 + 报文体片段 |

## 加工方式

原文件是 44.1 kHz mp3，用 ffmpeg 重采样为 8000 Hz / 16-bit / 单声道 WAV
（`ffmpeg -i 原文件.mp3 -ar 8000 -ac 1 -c:a pcm_s16le 目标.wav`），未做其他处理。

## 许可

- SigidWiki 页面**未对样本标注单独的许可条款**（页面只有站级隐私政策与免责声明）。
- 样本仅用于本仓解码器的**互操作测试**（测试断言页面文档中写明的解码内容），不随
  前端分发、不进入构建产物。若样本权利人提出要求，删除并换用合成样本即可 ——
  核心测试全部自足，样本测试是独立的一条。

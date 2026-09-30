//! SSTV 慢扫描电视：在话音带宽内传输静止图像。

/// 常见 SSTV 模式（模式名、分辨率与时长、说明）。
pub const SSTV_MODES: &[(&str, &str, &str)] = &[
  ("Robot 36", "320×240 · 约 36 秒", "快速彩色，入门首选。"),
  ("Robot 72", "320×240 · 约 72 秒", "比 Robot 36 更抗噪声。"),
  ("Martin 1", "320×256 · 约 114 秒", "欧洲常用彩色模式。"),
  ("Martin 2", "320×256 · 约 58 秒", "Martin 1 的快速版。"),
  (
    "Scottie 1",
    "320×256 · 约 110 秒",
    "经典彩色模式，色彩还原好。",
  ),
  ("Scottie 2", "320×256 · 约 71 秒", "Scottie 1 的快速版。"),
  (
    "Scottie DX",
    "320×256 · 约 269 秒",
    "高分辨率慢速，弱信号下更可靠。",
  ),
  ("PD120", "640×496 · 约 119 秒", "现代高分辨率模式。"),
];

/// 常用呼叫频率。
pub const SSTV_FREQS: &[(&str, &str)] = &[
  ("80m", "3.735 MHz（LSB）"),
  ("40m", "7.171 MHz（LSB）"),
  ("20m", "14.230 MHz（USB）"),
  ("15m", "21.340 MHz（USB）"),
  ("10m", "28.680 MHz（USB）"),
];

/// 核心概念。
pub const SSTV_CONCEPTS: &[(&str, &str)] = &[
  (
    "SSTV",
    "Slow-Scan Television，在话音带宽内逐行传送静止图像。",
  ),
  (
    "频率调制",
    "像素亮度用音频频率高低表示，经 SSB/FM 信道发射。",
  ),
  ("同步脉冲", "每行与每帧之间的同步信号，用于对齐图像位置。"),
  ("VIS 码", "图像开始前的数字标识，接收端据此自动识别模式。"),
];

/// 操作要点。
pub const SSTV_TIPS: &[&str] = &[
  "接收只需普通 SSB 接收机 + 电脑/手机声卡，配合 MMSSTV、QSSTV 或手机 SSTV 应用。",
  "把电台音频接到电脑麦克风或线路输入即可解码，无需特殊接口。",
  "国际空间站不定期开展 SSTV 活动，用 437.800MHz FM 接收。",
  "发射前确认频率未被占用，图像内容遵守业余无线电礼仪。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sstv_data_populated() {
    assert!(SSTV_MODES.len() >= 5);
    assert!(SSTV_FREQS.len() >= 4);
    assert!(!SSTV_CONCEPTS.is_empty());
    assert!(!SSTV_TIPS.is_empty());
  }
}

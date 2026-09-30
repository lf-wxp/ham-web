//! SDR 硬件与 GNU Radio：接收机选型与软件无线电流图。

/// 硬件选型。
pub const SDR_HARDWARE: &[(&str, &str, &str)] = &[
  ("RTL-SDR", "接收", "约 24MHz–1.7GHz，廉价入门首选。"),
  ("Airspy", "接收", "高性能接收机，R2 / Mini 覆盖宽频。"),
  ("SDRplay RSP", "接收", "宽频接收，覆盖 LW/MW/HF/VHF/UHF。"),
  ("HackRF One", "收发", "1MHz–6GHz 半双工收发，适合实验。"),
  ("USRP", "收发", "高端软件无线电平台，科研与基站。"),
  ("ICOM IC-705", "收发一体", "内置频谱瀑布，便携全频段收发。"),
];

/// GNU Radio 概念。
pub const GNU_RADIO_CONCEPTS: &[(&str, &str)] = &[
  ("流图 Flowgraph", "用模块（block）连接成信号处理流水线。"),
  (
    "信号源 / 汇",
    "Source 采集、Sink 输出，中间连接滤波器、解调器等模块。",
  ),
  ("GNU Radio Companion", "图形化拖拽搭建流图的工具（GRC）。"),
  (
    "采样率与带宽",
    "采样率决定可观察带宽，需满足奈奎斯特采样定理。",
  ),
  ("应用", "解码数传、分析信号、自研接收机原型。"),
];

/// 上手要点。
pub const GNU_RADIO_TIPS: &[&str] = &[
  "入门从 GRC 拖拽「信号源 → 低通 → 解调 → 音频输出」开始。",
  "先熟悉 FM 接收流图，再尝试 AM / SSB 与数字模式解码。",
  "注意驱动与采样率设置，避免丢样（underflow / overflow）。",
  "配合 RTL-SDR 驱动（rtl-sdr）与 SoapySDR 通用接口。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn gnuradio_data_populated() {
    assert!(SDR_HARDWARE.len() >= 4);
    assert!(GNU_RADIO_CONCEPTS.len() >= 4);
    assert!(!GNU_RADIO_TIPS.is_empty());
  }
}

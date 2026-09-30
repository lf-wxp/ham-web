//! 模拟通信模式：CW、SSB、AM、FM 等传统话音与电报模式速查。

/// 一种模拟模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalogMode {
  pub name: &'static str,
  pub abbr: &'static str,
  /// ITU 发射类别标识。
  pub emission: &'static str,
  /// 典型带宽。
  pub bandwidth: &'static str,
  /// 简述。
  pub desc: &'static str,
  /// 特点 / 优点。
  pub pros: &'static str,
  /// 典型用途。
  pub usage: &'static str,
}

/// 常见模拟模式。
pub const ANALOG_MODES: &[AnalogMode] = &[
  AnalogMode {
    name: "等幅电报",
    abbr: "CW",
    emission: "A1A",
    bandwidth: "100–300 Hz",
    desc: "键控载波通断，用莫尔斯电码通信。",
    pros: "带宽极窄、能量集中，弱信号下仍可抄收，穿透力强。",
    usage: "远距离 DX、弱信号、应急通信",
  },
  AnalogMode {
    name: "单边带",
    abbr: "SSB",
    emission: "J3E",
    bandwidth: "约 2.7 kHz",
    desc: "抑制载波与一个边带，只传一个边带（USB / LSB）。",
    pros: "功率利用率高，约为 AM 的 4 倍。",
    usage: "HF 话音通信主力",
  },
  AnalogMode {
    name: "双边带调幅",
    abbr: "AM",
    emission: "A3E",
    bandwidth: "约 6 kHz",
    desc: "载波加两个边带，设备简单。",
    pros: "音质好、解调简单。",
    usage: "广播、航空、部分业余通信",
  },
  AnalogMode {
    name: "调频",
    abbr: "FM",
    emission: "F3E",
    bandwidth: "12.5 / 25 kHz",
    desc: "频率调制，抗干扰能力强。",
    pros: "音质好、有捕获效应，强信号压制弱信号。",
    usage: "VHF/UHF 本地、中继台",
  },
  AnalogMode {
    name: "慢扫描电视",
    abbr: "SSTV",
    emission: "F3F",
    bandwidth: "约 2.5 kHz",
    desc: "用语音信道传输静止图像。",
    pros: "可在 SSB 带宽内传图。",
    usage: "图像传输、通联奖状",
  },
];

/// USB / LSB 边带选择惯例。
pub const SIDEBAND_RULES: &[&str] = &[
  "10 MHz 以下（80m、40m 等）惯例使用 LSB（下边带）。",
  "10 MHz 以上（20m、15m、10m 等）惯例使用 USB（上边带）。",
  "VHF/UHF 及 60m、30m 等特殊波段统一使用 USB。",
  "CW 与数据模式不使用边带约定，通常在载波频率附近工作。",
];

/// 模拟与数字模式对比。
pub const ANALOG_VS_DIGITAL: &[(&str, &str)] = &[
  (
    "模拟模式",
    "连续波形承载信息，设备简单、话音自然，但抗噪声相对弱。",
  ),
  (
    "数字模式",
    "离散编码，弱信号解码能力强、带宽窄，适合数据与远距离。",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn analog_modes_populated() {
    assert!(ANALOG_MODES.len() >= 4);
    for m in ANALOG_MODES {
      assert!(!m.name.is_empty());
      assert!(!m.abbr.is_empty());
      assert!(!m.emission.is_empty());
      assert!(!m.bandwidth.is_empty());
      assert!(!m.usage.is_empty());
    }
    assert!(!SIDEBAND_RULES.is_empty());
    assert!(!ANALOG_VS_DIGITAL.is_empty());
  }
}

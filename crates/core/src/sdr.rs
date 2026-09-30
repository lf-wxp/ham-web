//! 软件定义无线电 SDR：概念、架构与常用软件。

/// 核心概念。
pub const SDR_CONCEPTS: &[(&str, &str)] = &[
  (
    "SDR",
    "Software Defined Radio，用软件完成调制解调、滤波等大部分信号处理。",
  ),
  (
    "正交采样",
    "I/Q 两路信号采样，可还原幅度与相位，是 SDR 的核心。",
  ),
  (
    "直接采样",
    "ADC 直接对射频采样，无需模拟变频，代表高端 SDR。",
  ),
  (
    "RTL-SDR",
    "基于电视棒芯片的廉价接收机，覆盖约 24MHz–1.7GHz，入门首选。",
  ),
  (
    "频谱瀑布图",
    "实时显示频率-时间-强度的三维视图，直观观察信号。",
  ),
];

/// 常用软件。
pub const SDR_SOFTWARE: &[(&str, &str)] = &[
  ("SDR#", "Windows 平台最流行的 SDR 接收软件。"),
  ("Gqrx", "Linux/macOS 常用 SDR 接收软件。"),
  ("SDR++", "跨平台、开源、界面现代的 SDR 软件。"),
  ("HDSDR", "Windows 常用，支持多种硬件与插件。"),
  (
    "WSJT-X / JTDX",
    "基于 SDR 收发的弱信号数字模式软件（FT8/FT4/JT65）。",
  ),
];

/// 在线 SDR 平台（无需本地硬件，浏览器直接收听）。
pub const WEB_SDR: &[(&str, &str)] = &[
  (
    "WebSDR",
    "荷兰 Twente 大学运营，全球多站点，浏览器直接收听。",
  ),
  ("KiwiSDR", "全球爱好者架设的在线接收机，覆盖 0–30MHz。"),
  ("KFS WebSDR", "美国 Half Moon Bay 站点，覆盖范围广。"),
  ("GlobalTuners", "远程电台控制，可操作全球各地接收机。"),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sdr_data_populated() {
    assert!(!SDR_CONCEPTS.is_empty());
    assert!(!SDR_SOFTWARE.is_empty());
    assert!(!WEB_SDR.is_empty());
  }
}

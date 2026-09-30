//! 业余无线电历史：重要人物与里程碑时间线。

/// 时间线。
pub const HISTORY_TIMELINE: &[(&str, &str)] = &[
  ("1888", "赫兹用实验证实电磁波存在。"),
  ("1895", "马可尼实现早期无线电通信。"),
  ("1901", "马可尼实现跨大西洋无线电通信。"),
  ("1914", "ARRL（美国业余无线电联盟）成立。"),
  ("1925", "IARU（国际业余无线电联盟）成立。"),
  ("1957", "苏联发射首颗人造卫星，催生卫星业余通信。"),
  ("1961", "首颗业余卫星 OSCAR-1 发射。"),
  (
    "1990s–",
    "数字模式兴起，PSK31、JT65、FT8 相继普及，弱信号通信进入新时代。",
  ),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn history_populated() {
    assert!(HISTORY_TIMELINE.len() >= 6);
    for (year, event) in HISTORY_TIMELINE {
      assert!(!year.is_empty());
      assert!(!event.is_empty());
    }
  }
}

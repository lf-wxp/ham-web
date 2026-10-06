//! WSPR 弱信号传播报告：原理、使用与传播研究。

/// 核心概念。
pub const WSPR_CONCEPTS: &[(&str, &str)] = &[
  (
    "WSPR",
    "Weak Signal Propagation Reporter，用极低功率（mW 级）测试传播的弱信号模式。",
  ),
  (
    "信标",
    "周期发送呼号、网格、功率等信息，接收站解码并上传到 WSPRnet 数据库。",
  ),
  ("传播研究", "全球接收站汇总数据，可查看各波段实时传播图。"),
];

/// 使用要点。
pub const WSPR_NOTES: &[&str] = &[
  "发射功率可低至 mW 级（WSPR 编码下限 0 dBm = 1mW），传播良好时即可被全球接收站解码。",
  "与 FT8 同样需要严格时间同步。",
  "发射后到 wsprnet.org 查看自己的信号被哪些台站接收。",
  "遵守功率限制，WSPR 是信标模式，不应占用话音通联频率。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wspr_data_populated() {
    assert!(!WSPR_CONCEPTS.is_empty());
    assert!(!WSPR_NOTES.is_empty());
  }
}

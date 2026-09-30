//! Maidenhead 网格定位：网格层级、精度与应用。

/// 网格层级。
pub const GRID_LEVELS: &[(&str, &str, &str)] = &[
  ("2 位", "字段（Field），如 OM", "约 20°×10°，覆盖大区域。"),
  (
    "4 位",
    "网格（Square），如 OM89",
    "约 1°×2°，最常用，交换台址的标准精度。",
  ),
  (
    "6 位",
    "子网格（Subsquare），如 OM89EW",
    "约 2.5′×5′，精度约 5×10 km。",
  ),
  ("8 位", "扩展精度", "更高精度，用于卫星、测向等。"),
];

/// 说明要点。
pub const GRID_NOTES: &[&str] = &[
  "网格由经纬度编码而成，第一个字母按经度 20° 分，第二个字母按纬度 10° 分。",
  "通联中通常交换 4 位网格（如 OM89），用于距离与方向计算。",
  "本工具支持经纬度 ↔ 网格互转，见「小工具 · Maidenhead 网格定位」。",
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn grid_system_populated() {
    assert!(GRID_LEVELS.len() >= 3);
    assert!(!GRID_NOTES.is_empty());
  }
}

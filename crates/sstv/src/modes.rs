//! SSTV 模式参数：Martin / Scottie / Robot 的时序与颜色编码。

/// 颜色编码方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorKind {
  /// RGB 逐像素（Martin / Scottie）。
  Rgb,
  /// 亮度 + 色度（Robot）。
  Yc,
}

/// 一种 SSTV 模式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mode {
  pub name: &'static str,
  /// VIS 7 位数据码（不含偶校验位，即 `raw & 0x7F`）。
  pub vis_code: u8,
  pub width: usize,
  pub height: usize,
  /// 行同步脉冲时长（毫秒）。
  pub sync_ms: f32,
  /// 同步后 porch 时长（毫秒），Scottie / Robot 为 0。
  pub porch_ms: f32,
  /// 每颜色分量时长（微秒）。
  pub pixel_us: f32,
  pub color: ColorKind,
}

/// 全部支持的模式。
pub const MODES: &[Mode] = &[
  Mode {
    name: "Martin M1",
    vis_code: 44,
    width: 320,
    height: 256,
    sync_ms: 4.862,
    porch_ms: 0.572,
    pixel_us: 457.6,
    color: ColorKind::Rgb,
  },
  Mode {
    name: "Martin M2",
    vis_code: 40,
    width: 320,
    height: 256,
    sync_ms: 4.862,
    porch_ms: 0.572,
    pixel_us: 228.8,
    color: ColorKind::Rgb,
  },
  Mode {
    name: "Scottie S1",
    vis_code: 60,
    width: 320,
    height: 256,
    sync_ms: 9.0,
    porch_ms: 0.0,
    pixel_us: 432.0,
    color: ColorKind::Rgb,
  },
  Mode {
    name: "Scottie S2",
    vis_code: 56,
    width: 320,
    height: 256,
    sync_ms: 9.0,
    porch_ms: 0.0,
    pixel_us: 275.0,
    color: ColorKind::Rgb,
  },
  Mode {
    name: "Scottie DX",
    vis_code: 76,
    width: 320,
    height: 256,
    sync_ms: 9.0,
    porch_ms: 0.0,
    pixel_us: 172.8,
    color: ColorKind::Rgb,
  },
  Mode {
    name: "Robot 36",
    vis_code: 8,
    width: 320,
    height: 240,
    sync_ms: 9.0,
    porch_ms: 0.0,
    pixel_us: 275.0,
    color: ColorKind::Yc,
  },
  Mode {
    name: "Robot 72",
    vis_code: 12,
    width: 320,
    height: 240,
    sync_ms: 9.0,
    porch_ms: 0.0,
    pixel_us: 138.0,
    color: ColorKind::Yc,
  },
];

/// 由 VIS 7 位数据码查模式。
pub fn by_vis_code(code: u8) -> Option<&'static Mode> {
  MODES.iter().find(|m| m.vis_code == code)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn modes_have_valid_codes() {
    assert!(MODES.len() >= 5);
    for m in MODES {
      assert!(m.vis_code < 128, "VIS 数据码应为 7 位：{}", m.name);
      assert!(m.width > 0 && m.height > 0);
      assert!(m.sync_ms > 0.0);
      assert!(m.pixel_us > 0.0);
    }
  }

  #[test]
  fn lookup_by_vis_code() {
    assert_eq!(by_vis_code(44).map(|m| m.name), Some("Martin M1"));
    assert_eq!(by_vis_code(60).map(|m| m.name), Some("Scottie S1"));
    assert_eq!(by_vis_code(8).map(|m| m.name), Some("Robot 36"));
    assert!(by_vis_code(0).is_none());
  }
}

//! SSTV 模式参数：Martin / Scottie / Robot 的行结构。
//!
//! 每个模式用「**相对行同步脉冲起点**的扫描段表」描述（单位毫秒），而不是「同步 + 肩 +
//! 像素时长 × 分量数」的简化模型。简化模型对 Martin / Scottie S1 / S2 恰好吻合，对其余模式
//! 全部错误：
//!
//! - **Scottie**：同步脉冲在 **蓝之后、红之前**（行中部），颜色顺序是 绿 → 蓝 → 红，且每个分量
//!   前有 1.5 ms 分隔；
//! - **Robot 36**：每行 150 ms，Y 占 88 ms，色度只有 44 ms 且**每行只发一路**（偶行 R-Y、
//!   奇行 B-Y），水平分辨率减半；
//! - **Robot 72**：每行 300 ms，Y 138 ms、V（R-Y）69 ms、U（B-Y）69 ms，色度水平分辨率减半。
//!
//! 数值来源（互相印证）：SSTV Handbook / Dayton Paper 的时序表、MMSSTV 与 Robot36（xdsopl）
//! 的实现、以及各模式整图时长的公开值（Martin M1 114 s、M2 58 s、Scottie S1 110 s、S2 71 s、
//! DX 269 s、Robot 36 36 s、Robot 72 72 s，见 `tests/spec.rs` 的对账测试）。

/// 一个扫描段承载的分量。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comp {
  /// 红。
  R,
  /// 绿。
  G,
  /// 蓝。
  B,
  /// 亮度。
  Y,
  /// 色差 R-Y（Robot 72 的 V）。
  Ry,
  /// 色差 B-Y（Robot 72 的 U）。
  By,
  /// Robot 36 的交替色度：这一行是 R-Y 还是 B-Y 由行首的分隔脉冲频率决定。
  ChromaAlt,
}

/// 一行中的一个扫描段。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scan {
  pub comp: Comp,
  /// 段起点，相对**本行同步脉冲起点**（毫秒，可为负：Scottie 的绿 / 蓝在同步之前）。
  pub start_ms: f64,
  /// 段时长（毫秒）。
  pub len_ms: f64,
  /// 段内像素数（Robot 的色度只有 `width / 2`）。
  pub pixels: usize,
}

/// 颜色编码方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorKind {
  /// RGB 三个分量各一段（Martin / Scottie）。
  Rgb,
  /// 每行 Y + R-Y + B-Y（Robot 72）。
  Yc,
  /// 每行 Y + 一路交替色度，相邻两行配对（Robot 36）。
  YcInterlaced,
}

/// 边沿偏移的证据等级：诚实标明哪些模式有真实录音支撑。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeEvidence {
  /// 在真实录音上标定（对参考图 / 白底行扫描得到最优值）。
  Calibrated,
  /// 沿用同族已标定模式的值（同步与肩的结构相同），本模式**没有真实录音验证**。
  Inherited,
  /// 没有任何可参照的录音，取 0。解码结果可能有 1 ~ 3 像素的水平偏移。
  Unverified,
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
  pub sync_ms: f64,
  /// 相邻两行同步脉冲起点的间隔（毫秒）。
  pub line_ms: f64,
  /// 同步下降沿半高点相对脉冲标称起点的偏移（毫秒；负 = 半高点在标称起点之前）。
  ///
  /// 这是**每个模式各自经验标定**的小量（FM 解调的群延迟 + 不同模式同步前后的像素频率
  /// 不同，下降沿形状不同），见 [`EdgeEvidence`]。**不是物理常数，不能跨模式共用**：Martin M1
  /// 与 Robot 36 的最优值差约 0.8 ms（Martin 0.4576 ms / 像素，Robot 0.275 ms / 像素，所以
  /// 同一个错误在 Robot 上更显眼）。
  pub edge_ms: f64,
  /// [`Mode::edge_ms`] 的证据等级。
  pub edge_evidence: EdgeEvidence,
  /// VIS 之后**第一个**同步脉冲到**第一行锚点同步**的间隔（毫秒）。
  ///
  /// Martin / Robot 为 0。Scottie 的行内同步在中部，而整幅图的开头另有一个 9 ms 同步
  /// （这就是它第一行比其余行长 10.5 ms 的原因），所以第一行锚点在其后一整个「分隔 + 绿 +
  /// 分隔 + 蓝」之后。
  pub lead_in_ms: f64,
  pub scans: &'static [Scan],
  pub color: ColorKind,
}

impl Mode {
  /// 一行内所有扫描段覆盖的时间范围 `(最早起点, 最晚终点)`，相对同步脉冲起点（毫秒）。
  pub fn extent_ms(&self) -> (f64, f64) {
    self.scans.iter().fold((f64::MAX, f64::MIN), |(lo, hi), s| {
      (lo.min(s.start_ms), hi.max(s.start_ms + s.len_ms))
    })
  }
}

const fn scan(comp: Comp, start_ms: f64, len_ms: f64, pixels: usize) -> Scan {
  Scan {
    comp,
    start_ms,
    len_ms,
    pixels,
  }
}

// Martin：同步 → 分隔 → G → 分隔 → B → 分隔 → R → 分隔（共 4 个 0.572 ms 分隔）。
const MARTIN_M1_SCANS: &[Scan] = &[
  scan(Comp::G, 5.434, 146.432, 320),
  scan(Comp::B, 152.438, 146.432, 320),
  scan(Comp::R, 299.442, 146.432, 320),
];
const MARTIN_M2_SCANS: &[Scan] = &[
  scan(Comp::G, 5.434, 73.216, 320),
  scan(Comp::B, 79.222, 73.216, 320),
  scan(Comp::R, 153.010, 73.216, 320),
];

// Scottie：分隔 → G → 分隔 → B → **同步** → 分隔 → R。锚点是中部的同步，G / B 在它之前。
const SCOTTIE_S1_SCANS: &[Scan] = &[
  scan(Comp::G, -277.98, 138.24, 320),
  scan(Comp::B, -138.24, 138.24, 320),
  scan(Comp::R, 10.5, 138.24, 320),
];
const SCOTTIE_S2_SCANS: &[Scan] = &[
  scan(Comp::G, -177.628, 88.064, 320),
  scan(Comp::B, -88.064, 88.064, 320),
  scan(Comp::R, 10.5, 88.064, 320),
];
const SCOTTIE_DX_SCANS: &[Scan] = &[
  scan(Comp::G, -692.7, 345.6, 320),
  scan(Comp::B, -345.6, 345.6, 320),
  scan(Comp::R, 10.5, 345.6, 320),
];

// Robot 36：同步 9 + 同步肩 3 + Y 88 + 分隔 4.5 + 肩 1.5 + 色度 44 = 150。
const ROBOT_36_SCANS: &[Scan] = &[
  scan(Comp::Y, 12.0, 88.0, 320),
  scan(Comp::ChromaAlt, 106.0, 44.0, 160),
];
// Robot 72：9 + 3 + Y 138 + (4.5 + 1.5 + V 69) + (4.5 + 1.5 + U 69) = 300。
const ROBOT_72_SCANS: &[Scan] = &[
  scan(Comp::Y, 12.0, 138.0, 320),
  scan(Comp::Ry, 156.0, 69.0, 160),
  scan(Comp::By, 231.0, 69.0, 160),
];

/// Robot 36 行首「偶 / 奇」分隔脉冲相对同步起点的位置与时长（毫秒）。
pub const ROBOT_36_SEPARATOR: (f64, f64) = (100.0, 4.5);

/// 全部支持的模式。
pub const MODES: &[Mode] = &[
  Mode {
    name: "Martin M1",
    vis_code: 44,
    width: 320,
    height: 256,
    sync_ms: 4.862,
    line_ms: 446.446,
    edge_ms: -0.7,
    edge_evidence: EdgeEvidence::Calibrated,
    lead_in_ms: 0.0,
    scans: MARTIN_M1_SCANS,
    color: ColorKind::Rgb,
  },
  Mode {
    name: "Martin M2",
    vis_code: 40,
    width: 320,
    height: 256,
    sync_ms: 4.862,
    line_ms: 226.798,
    edge_ms: -0.7,
    edge_evidence: EdgeEvidence::Inherited,
    lead_in_ms: 0.0,
    scans: MARTIN_M2_SCANS,
    color: ColorKind::Rgb,
  },
  Mode {
    name: "Scottie S1",
    vis_code: 60,
    width: 320,
    height: 256,
    sync_ms: 9.0,
    line_ms: 428.22,
    edge_ms: 0.0,
    edge_evidence: EdgeEvidence::Unverified,
    lead_in_ms: 288.48,
    scans: SCOTTIE_S1_SCANS,
    color: ColorKind::Rgb,
  },
  Mode {
    name: "Scottie S2",
    vis_code: 56,
    width: 320,
    height: 256,
    sync_ms: 9.0,
    line_ms: 277.692,
    edge_ms: 0.0,
    edge_evidence: EdgeEvidence::Unverified,
    lead_in_ms: 188.128,
    scans: SCOTTIE_S2_SCANS,
    color: ColorKind::Rgb,
  },
  Mode {
    name: "Scottie DX",
    vis_code: 76,
    width: 320,
    height: 256,
    sync_ms: 9.0,
    line_ms: 1050.3,
    edge_ms: 0.0,
    edge_evidence: EdgeEvidence::Unverified,
    lead_in_ms: 703.2,
    scans: SCOTTIE_DX_SCANS,
    color: ColorKind::Rgb,
  },
  Mode {
    name: "Robot 36",
    vis_code: 8,
    width: 320,
    height: 240,
    sync_ms: 9.0,
    line_ms: 150.0,
    edge_ms: 0.1,
    edge_evidence: EdgeEvidence::Calibrated,
    lead_in_ms: 0.0,
    scans: ROBOT_36_SCANS,
    color: ColorKind::YcInterlaced,
  },
  Mode {
    name: "Robot 72",
    vis_code: 12,
    width: 320,
    height: 240,
    sync_ms: 9.0,
    line_ms: 300.0,
    edge_ms: 0.1,
    edge_evidence: EdgeEvidence::Inherited,
    lead_in_ms: 0.0,
    scans: ROBOT_72_SCANS,
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
  fn vis_codes_are_unique_and_seven_bit() {
    for (i, m) in MODES.iter().enumerate() {
      assert!(m.vis_code < 128, "VIS 数据码应为 7 位：{}", m.name);
      assert!(
        MODES[i + 1..].iter().all(|o| o.vis_code != m.vis_code),
        "VIS 码重复：{}",
        m.name
      );
    }
  }

  #[test]
  fn lookup_by_vis_code() {
    assert_eq!(by_vis_code(44).map(|m| m.name), Some("Martin M1"));
    assert_eq!(by_vis_code(60).map(|m| m.name), Some("Scottie S1"));
    assert_eq!(by_vis_code(76).map(|m| m.name), Some("Scottie DX"));
    assert_eq!(by_vis_code(8).map(|m| m.name), Some("Robot 36"));
    assert_eq!(by_vis_code(12).map(|m| m.name), Some("Robot 72"));
    assert!(by_vis_code(0).is_none());
  }

  #[test]
  fn edge_offsets_are_small_and_calibrated_modes_are_the_ones_with_recordings() {
    for m in MODES {
      assert!(
        m.edge_ms.abs() <= 2.0,
        "{}：边沿偏移 {} ms 过大",
        m.name,
        m.edge_ms
      );
    }
    // 只有 Martin M1 / Robot 36 有真实录音；其余必须如实标为 Inherited / Unverified。
    let calibrated: Vec<_> = MODES
      .iter()
      .filter(|m| m.edge_evidence == EdgeEvidence::Calibrated)
      .map(|m| m.name)
      .collect();
    assert_eq!(calibrated, ["Martin M1", "Robot 36"]);
  }

  #[test]
  fn every_scan_fits_inside_one_line() {
    // 扫描段必须落在「上一行锚点 + line_ms」与「下一行锚点」之间，且互不重叠。
    for m in MODES {
      let (lo, hi) = m.extent_ms();
      assert!(hi - lo <= m.line_ms, "{}：扫描段跨度超过一行", m.name);
      let mut spans: Vec<_> = m
        .scans
        .iter()
        .map(|s| (s.start_ms, s.start_ms + s.len_ms))
        .collect();
      spans.sort_by(|a, b| a.0.total_cmp(&b.0));
      for w in spans.windows(2) {
        assert!(w[0].1 <= w[1].0 + 1e-9, "{}：扫描段重叠", m.name);
      }
    }
  }
}

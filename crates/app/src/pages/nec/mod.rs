//! NEC 矩量法天线求解：几何 → 阻抗 / 驻波比 / 增益 / 效率 / 方向图。
//!
//! 全部由 `ham_web_core::nec` 的纯函数驱动（浏览器本地计算，离线可用）。
//! 数值精度与适用边界写在核心模块的文档里，页面上也要如实提示。
//!
//! 几何、负载、地面三块都是**可编辑**的：预设只是「把一张表填好」，用户可以接着
//! 改坐标、增删导线与集总负载、切换地面模型，也可以用 `.nec` 文本导入导出。
//!
//! # 模块划分
//!
//! 每个文件只放一个组件（或一个视图构造器）：页面壳 [`nec_page::NecPage`]、
//! 几何画布 [`canvas::WireCanvas`]、Yagi 设计向导 [`design::DesignSection`]、
//! 导线表 [`wire_table::WireTable`]、负载表 [`load_table::LoadTable`]、
//! 地面与材质 [`ground_material::GroundMaterialSection`]、
//! 导入导出 [`import_export::ImportExportSection`]、结果卡片
//! [`result_section::ResultSection`]、三维方向图 [`pattern_3d::Pattern3d`]、
//! 数值输入框 [`num_field::NumField`]、预设按钮 [`preset_chip::chip`]，以及两张
//! 方向图的网格 [`polar_grid::polar_grid`] / [`elevation_grid::elevation_grid`]。
//! 本文件只保留**跨文件共享**的内容：样式常量、材质表、领域类型（[`Preset`] /
//! [`WireRow`] / [`LoadRow`]）与方向图的坐标 / 路径计算。

mod canvas;
mod design;
mod elevation_grid;
mod ground_material;
mod import_export;
mod lines;
mod load_table;
mod nec_page;
mod num_field;
mod nvis;
mod pattern_3d;
mod polar_grid;
mod preset_chip;
mod result_section;
mod sphere;
mod sweep;
mod wire_table;

pub use nec_page::NecPage;

use ham_web_core::nec::{Load, Wire};

use crate::i18n::t;

/// 导线材质（电导率 S/m）。
///
/// 「地面与材质」卡片的按钮与本页的 `sigma_wire()` 共用这一份：按钮下标必须与这里
/// 一一对应，否则点「铜」却按铝算，界面上还看不出来。
///
/// 第一列是**译文取值函数**而不是中文原文：`check-i18n` 判死条目时认「`t("key")` 的直接
/// 实参」，这里写的 `t("tools.nec-material-copper")` 就是字面量实参，转成 `fn` 后仍能在
/// 视图闭包里响应式求值（详见 `pages::waveform_lab::wave_section::kind_text`）。
type Material = (fn() -> String, Option<f64>);
const MATERIALS: [Material; 4] = [
  (|| t("tools.nec-material-ideal"), None),
  (|| t("tools.nec-material-copper"), Some(5.8e7)),
  (|| t("tools.nec-material-aluminium"), Some(3.5e7)),
  (|| t("tools.nec-material-brass"), Some(1.6e7)),
];

const RESULT: &str = "rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground";
const NOTE: &str = "text-xs text-muted-foreground";
const PLOT: &str = "mx-auto w-full";

/// 极坐标图边长与内边距。
const POLAR: f64 = 280.0;
/// 直角坐标图尺寸。
const CART_W: f64 = 320.0;
const CART_H: f64 = 200.0;
const CART_PAD: f64 = 28.0;
/// 方向图纵轴范围（相对最大增益的 dB 下限）。
const DB_FLOOR: f64 = -30.0;

impl Preset {
  /// 稳定的字符串标识：`ChipGroup` 的选项值用它，重排 / 增删预设都不会换掉用户的选择。
  pub(super) const fn key(self) -> &'static str {
    match self {
      Self::Dipole => "dipole",
      Self::Vertical => "vertical",
      Self::Yagi2 => "yagi2",
      Self::Yagi3 => "yagi3",
      Self::InvertedV => "inverted-v",
    }
  }
}

/// 可选的预设结构。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Preset {
  /// 水平半波偶极。
  Dipole,
  /// 垂直（四分之一波长，接地面）。
  Vertical,
  /// 两单元八木。
  Yagi2,
  /// 三单元八木。
  Yagi3,
  /// 倒 V（两臂各下垂 45°）。
  InvertedV,
}

const PRESETS: [Preset; 5] = [
  Preset::Dipole,
  Preset::Vertical,
  Preset::Yagi2,
  Preset::Yagi3,
  Preset::InvertedV,
];

impl Preset {
  /// 该预设需要的参数项。
  fn needs_spacing(self) -> bool {
    matches!(self, Self::Yagi2 | Self::Yagi3)
  }

  /// 构建几何：`(导线, 主元段数, 是否接地面)`。
  fn build(self, len: f64, height: f64, spacing: f64, radius: f64) -> (Vec<Wire>, usize, bool) {
    let segs = 20usize;
    match self {
      Self::Dipole => {
        let ground = height > 1e-6;
        let z = if ground { height } else { 0.0 };
        (
          vec![Wire::new(
            [-len / 2.0, 0.0, z],
            [len / 2.0, 0.0, z],
            radius,
            2 * segs,
          )],
          2 * segs,
          ground,
        )
      }
      Self::Vertical => (
        vec![Wire::new(
          [0.0, 0.0, 0.0],
          [0.0, 0.0, len],
          radius,
          2 * segs,
        )],
        2 * segs,
        true,
      ),
      Self::Yagi2 | Self::Yagi3 => {
        let driven =
          |x: f64, l: f64| Wire::new([x, -l / 2.0, 0.0], [x, l / 2.0, 0.0], radius, segs);
        // 经典三单元比例（以有源振子长度为基准）：反射器长 4.7%、引向器短 7.8%，
        // 间距 0.15λ / 0.1λ。比例错了前后比会掉到 1 dB 量级 —— 八木对单元长度
        // 极其敏感，这也是这些参数值得放进交互界面的原因。
        let mut wires = vec![driven(0.0, len), driven(-spacing, len * 1.047)];
        if self == Self::Yagi3 {
          wires.push(driven(spacing * 0.68, len * 0.922));
        }
        (wires, segs, false)
      }
      Self::InvertedV => {
        let arm = len / 2.0;
        let drop = arm * 0.707;
        let apex = [0.0, 0.0, height];
        (
          vec![
            Wire::new(apex, [arm * 0.707, 0.0, height - drop], radius, segs),
            Wire::new(apex, [-arm * 0.707, 0.0, height - drop], radius, segs),
          ],
          segs,
          false,
        )
      }
    }
  }

  /// 馈电点：`(导线号, 相对位置)`。
  fn feed(self) -> (usize, f64) {
    match self {
      Self::Vertical | Self::InvertedV => (0, 0.0),
      _ => (0, 0.5),
    }
  }
}

/// 表格里的导线行（全部用字符串承载，允许中途出现无效输入）。
#[derive(Clone, PartialEq)]
struct WireRow {
  id: usize,
  ax: String,
  ay: String,
  az: String,
  bx: String,
  by: String,
  bz: String,
  radius_mm: String,
  segments: String,
}

impl WireRow {
  fn from_wire(id: usize, w: &Wire) -> Self {
    let f = |v: f64| format!("{v:.3}");
    Self {
      id,
      ax: f(w.a[0]),
      ay: f(w.a[1]),
      az: f(w.a[2]),
      bx: f(w.b[0]),
      by: f(w.b[1]),
      bz: f(w.b[2]),
      radius_mm: format!("{:.2}", w.radius * 1000.0),
      segments: w.segments.max(1).to_string(),
    }
  }

  fn to_wire(&self) -> Option<Wire> {
    let p = |s: &str| s.trim().parse::<f64>().ok();
    Some(Wire::new(
      [p(&self.ax)?, p(&self.ay)?, p(&self.az)?],
      [p(&self.bx)?, p(&self.by)?, p(&self.bz)?],
      p(&self.radius_mm)? / 1000.0,
      self.segments.trim().parse::<usize>().unwrap_or(1),
    ))
  }
}

/// 表格里的负载行。
#[derive(Clone, PartialEq)]
struct LoadRow {
  id: usize,
  wire: String,
  at: String,
  r_ohm: String,
  l_uh: String,
  c_pf: String,
}

impl LoadRow {
  fn to_load(&self) -> Option<Load> {
    Some(Load {
      // 表面上的导线序号是 1 起的（导出到 .nec 也是 1 起），内层模型是 0 起。
      wire: self.wire.trim().parse::<usize>().ok()?.saturating_sub(1),
      at: self.at.trim().parse::<f64>().ok()?,
      r_ohm: self.r_ohm.trim().parse::<f64>().unwrap_or(0.0),
      l_uh: self.l_uh.trim().parse::<f64>().unwrap_or(0.0),
      c_pf: self.c_pf.trim().parse::<f64>().unwrap_or(0.0),
    })
  }
}

/// 按量级挑选小数位。
fn fmt(v: f64) -> String {
  if v.abs() >= 100.0 {
    format!("{v:.0}")
  } else if v.abs() >= 10.0 {
    format!("{v:.1}")
  } else {
    format!("{v:.2}")
  }
}

/// 复阻抗文案：`R ± jX`（数字按量级挑小数位）。符号逻辑与 `/smith` 共用实现。
fn fmt_z(r: f64, x: f64) -> String {
  crate::util::fmt_z(r, x, fmt)
}

/// 方位角 + 半径 → 画布坐标。
///
/// 按核心的约定换算：`0°` 指向 `+X`（正右）、`90°` 指向 `+Y`（正上），与极坐标网格上的
/// `+X` / `+Y` 标注一致（见 `core::nec` 的方向图单测）。曾经是 `ang = 90° − az`，
/// 等于把整张图旋转 90°：指向 `+X` 的主瓣被画到 `+Y` 方向，与读数对不上。
fn polar_xy(az_deg: f64, r: f64) -> (f64, f64) {
  let c = POLAR / 2.0;
  let ang = az_deg.to_radians();
  (c + r * ang.cos(), c - r * ang.sin())
}

/// 方位面极坐标路径。
fn polar_path(points: &[(f64, f64)], peak: f64) -> String {
  let r_max = POLAR / 2.0 - 18.0;
  let mut out = String::new();
  for (i, &(az, db)) in points.iter().enumerate() {
    let frac = ((db - peak - DB_FLOOR) / -DB_FLOOR).clamp(0.0, 1.0);
    let (x, y) = polar_xy(az, frac * r_max);
    if i == 0 {
      out.push_str(&format!("M{x:.1} {y:.1}"));
    } else {
      out.push_str(&format!(" L{x:.1} {y:.1}"));
    }
  }
  out.push_str(" Z");
  out
}

/// 仰角（度）→ 横轴坐标。
///
/// [`elevation_path`] 与 [`elevation_grid::elevation_grid`] **共用**这一个映射：两处各写一份时，
/// 刻度会与曲线错开半幅（曾经网格按 `el / 90 × 半幅` 画，有地面时读出的仰角整个
/// 偏移半个图宽）。
fn el_to_x(el_deg: f64) -> f64 {
  CART_PAD + (el_deg + 90.0) / 180.0 * (CART_W - 2.0 * CART_PAD)
}

/// 仰角面直角路径：横轴仰角 −90…+90°，纵轴 dB。
fn elevation_path(points: &[(f64, f64)], peak: f64) -> String {
  let h = CART_H - 2.0 * CART_PAD;
  let mut out = String::new();
  for (i, &(el, db)) in points.iter().enumerate() {
    let x = el_to_x(el);
    let y = CART_PAD + (peak - db).clamp(0.0, -DB_FLOOR) / -DB_FLOOR * h;
    if i == 0 {
      out.push_str(&format!("M{x:.1} {y:.1}"));
    } else {
      out.push_str(&format!(" L{x:.1} {y:.1}"));
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 断言 `az` 度的极坐标点落在给定位置上（容差 1e-6）。
  fn assert_polar_at(az: f64, x: f64, y: f64, what: &str) {
    let (got_x, got_y) = polar_xy(az, 10.0);
    assert!(
      (got_x - x).abs() < 1e-6 && (got_y - y).abs() < 1e-6,
      "{what}：{az}° 应落在 ({x:.1}, {y:.1})，实为 ({got_x:.3}, {got_y:.3})"
    );
  }

  #[test]
  fn polar_chart_puts_azimuth_zero_on_the_right() {
    // 核心约定：方位 0° = +X（正右）、90° = +Y（正上）—— 与网格上的 +X / +Y 标注一致。
    // 曾经用 `90° − az`，整张图旋转 90°，指向 +X 的主瓣被画到 +Y 方向。
    let c = POLAR / 2.0;
    assert_polar_at(0.0, c + 10.0, c, "0° 应在正右");
    assert_polar_at(90.0, c, c - 10.0, "90° 应在正上");
    assert_polar_at(180.0, c - 10.0, c, "180° 应在正左");
    assert_polar_at(270.0, c, c + 10.0, "270° 应在正下");
  }

  #[test]
  fn elevation_axis_maps_minus_90_to_plus_90_across_the_plot() {
    // 横轴刻度与曲线共用同一个映射：−90° 贴左内边距、0° 在正中、+90° 贴右内边距。
    let (left, right) = (CART_PAD, CART_W - CART_PAD);
    assert!((el_to_x(-90.0) - left).abs() < 1e-9);
    assert!((el_to_x(0.0) - (left + right) / 2.0).abs() < 1e-9);
    assert!((el_to_x(90.0) - right).abs() < 1e-9);
    // 单调递增：刻度不会互相穿越。
    assert!(el_to_x(-30.0) < el_to_x(0.0) && el_to_x(0.0) < el_to_x(30.0));
  }
}

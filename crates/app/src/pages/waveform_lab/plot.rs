//! 波形实验室的 SVG 绘图辅助：把数据坐标线性映射到 SVG 用户坐标。
//!
//! 只做折线，不做刻度自动生成 —— 四个面板的坐标含义差别很大（调制周期、
//! fm 倍数、dB、度、秒），刻度文案由各面板自己写死，反而更清楚。

/// 线性坐标映射。
#[derive(Debug, Clone, Copy)]
pub(super) struct Axes {
  /// 数据 x 下限（对应绘图区左边界）。
  pub x0: f64,
  /// 数据 x 上限。
  pub x1: f64,
  /// 数据 y 下限（对应绘图区下边界）。
  pub y0: f64,
  /// 数据 y 上限。
  pub y1: f64,
  /// SVG 宽度。
  pub w: f64,
  /// SVG 高度。
  pub h: f64,
  /// 绘图区内边距。
  pub pad: f64,
}

impl Axes {
  /// 数据 x → SVG x。
  pub(super) fn px(&self, x: f64) -> f64 {
    let span = (self.x1 - self.x0).abs().max(1e-12);
    self.pad + (x - self.x0) / span * (self.w - 2.0 * self.pad)
  }

  /// 数据 y → SVG y（已按 `y0..y1` 夹取，避免曲线跑到画布外几十倍远处）。
  pub(super) fn py(&self, y: f64) -> f64 {
    let (lo, hi) = (self.y0.min(self.y1), self.y0.max(self.y1));
    let y = y.clamp(lo, hi);
    let span = (self.y1 - self.y0).abs().max(1e-12);
    self.h - self.pad - (y - self.y0) / span * (self.h - 2.0 * self.pad)
  }

  /// 折线路径。
  pub(super) fn path(&self, pts: &[(f64, f64)]) -> String {
    let mut d = String::with_capacity(pts.len() * 14);
    for (i, &(x, y)) in pts.iter().enumerate() {
      let (px, py) = (self.px(x), self.py(y));
      if i == 0 {
        d.push_str(&format!("M {px:.1} {py:.1}"));
      } else {
        d.push_str(&format!(" L {px:.1} {py:.1}"));
      }
    }
    d
  }

  /// 水平网格线（数据 y 值 → SVG y）。
  pub(super) fn hlines(&self, values: &[f64]) -> Vec<f64> {
    values.iter().map(|&y| self.py(y)).collect()
  }

  /// 垂直网格线（数据 x 值 → SVG x）。
  pub(super) fn vlines(&self, values: &[f64]) -> Vec<f64> {
    values.iter().map(|&x| self.px(x)).collect()
  }
}

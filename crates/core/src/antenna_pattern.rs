//! 简化天线方向图计算（用于可视化教学）。
//!
//! 分两个视图维度：
//! - 方位角图（水平面，0–360°）：偶极子 E/H 面、三单元 Yagi；
//! - 仰角图（垂直面，0–90°，0°=地平线、90°=天顶）：垂直天线、水平偶极子、水平环。

/// 方向图类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
  /// 半波偶极子 E 面：**包含振子轴**的平面，呈「8」字形。
  ///
  /// 水平架设的偶极子，其水平面方位图就是 E 面，故最大辐射垂直于振子、轴向为零。
  /// （E 面的定义是「含振子轴且含最大辐射方向的平面」，不是「垂直于振子的平面」。）
  DipoleE,
  /// 半波偶极子 H 面：**垂直于振子轴**的平面，全向圆。
  ///
  /// 只有垂直架设的偶极子，其水平面方位图才是 H 面（全向）；水平架设时 H 面是竖直的。
  DipoleH,
  /// 三单元 Yagi（水平面，前向主瓣 + 后瓣）。
  Yagi,
  /// 垂直天线仰角图（1/4λ 接地，低仰角主瓣，适合 DX）。
  Vertical,
  /// 水平偶极子仰角图（架高可调，见 [`dipole_elevation`]）。
  DipoleEl,
  /// 水平环天线仰角图（1λ，天顶主瓣最强，适合 NVIS 近距通信）。
  ///
  /// 注意与 [`Self::SquareLoop`] / [`Self::DeltaLoop`] 的区别：本项是**仰角**图，
  /// 而方环 / 三角环画的是**方位**图。低仰角辐射才是 DX 关心的部分，仰角图上的
  /// 零点方向（地平线）并不代表方位图无意义 —— 实际架设高度与地面反射会填出低仰角分量。
  Loop,
  /// 方形环天线方位角图（水平面，4 个边中点方向略强）。
  SquareLoop,
  /// 三角环天线方位角图（水平面，3 个方向略强）。
  DeltaLoop,
  /// 二单元 Yagi（反射器 + 激励，水平面）。
  Yagi2,
  /// 四单元 Yagi（水平面，主瓣更尖锐、后瓣更小）。
  Yagi4,
  /// 1/4λ 垂直 + 地网仰角图（低仰角主瓣，略优于无地网垂直）。
  VerticalGP,
}

impl PatternKind {
  /// 中文标签。
  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::DipoleE => "偶极子 E 面",
      Self::DipoleH => "偶极子 H 面",
      Self::Yagi => "三单元 Yagi",
      Self::Vertical => "垂直天线",
      Self::DipoleEl => "偶极子（架高可调）",
      Self::Loop => "水平环天线",
      Self::SquareLoop => "方形环天线",
      Self::DeltaLoop => "三角环天线",
      Self::Yagi2 => "二单元 Yagi",
      Self::Yagi4 => "四单元 Yagi",
      Self::VerticalGP => "垂直天线（地网）",
    }
  }

  /// 是否为仰角图（垂直面，0–90°）；否则为方位角图（水平面，0–360°）。
  #[must_use]
  pub const fn is_elevation(self) -> bool {
    matches!(
      self,
      Self::Vertical | Self::DipoleEl | Self::Loop | Self::VerticalGP
    )
  }

  /// 全部类型。
  #[must_use]
  pub const fn all() -> &'static [Self] {
    &[
      Self::DipoleE,
      Self::DipoleH,
      Self::Yagi,
      Self::Yagi2,
      Self::Yagi4,
      Self::SquareLoop,
      Self::DeltaLoop,
      Self::Vertical,
      Self::VerticalGP,
      Self::DipoleEl,
      Self::Loop,
    ]
  }
}

/// 计算归一化增益（0–1）。
///
/// 方位角图 `theta_deg` 为方位角（0–360°）；仰角图 `theta_deg` 为仰角（0–90°，0°=地平线）。
#[must_use]
pub fn gain(kind: PatternKind, theta_deg: f64) -> f64 {
  let t = theta_deg.to_radians();
  match kind {
    // 偶极子最大辐射方向垂直于振子，故在 90°/270° 处为 1，轴向为 0。
    PatternKind::DipoleE => t.sin().abs(),
    // 水平面全向。
    PatternKind::DipoleH => 1.0,
    // 前向 cos³ 主瓣，后瓣约 -20 dB（0.01）。典型三单元八木 F/B 为 15–20 dB。
    PatternKind::Yagi => {
      let c = t.cos();
      if c >= 0.0 {
        c.powi(3)
      } else {
        c.abs().powi(3) * 0.01
      }
    }
    // 垂直天线：低仰角主瓣（地平线最强，天顶为零）。
    PatternKind::Vertical => t.cos(),
    // 水平偶极子：架高 0.5λ 时的仰角主瓣（约 30° 最强）。
    PatternKind::DipoleEl => dipole_elevation(theta_deg, 0.5),
    // 水平环：高仰角主瓣（天顶最强），适合 NVIS。
    PatternKind::Loop => t.sin(),
    // 方形环：水平面近似全向，方位起伏约 2 dB（0.8–1.0）。实测随馈电位置（角馈 /
    // 边中点馈）与架设高度变化，可达 3–4 dB；且最大方向随馈电点改变。
    PatternKind::SquareLoop => 0.9 + 0.1 * (4.0 * t).cos(),
    // 三角环：水平面近似全向，3 个方向略强，起伏同样约 2 dB。
    PatternKind::DeltaLoop => 0.9 + 0.1 * (3.0 * t).cos(),
    // 二单元 Yagi：前向 cos^2.5 主瓣，后瓣约 -10 dB（0.1，功率口径）。
    PatternKind::Yagi2 => {
      let c = t.cos();
      let mag = c.abs().powf(2.5);
      if c >= 0.0 { mag } else { mag * 0.1 }
    }
    // 四单元 Yagi：前向 cos^3.5 主瓣，后瓣约 -22 dB（0.006，功率口径）。
    PatternKind::Yagi4 => {
      let c = t.cos();
      let mag = c.abs().powf(3.5);
      if c >= 0.0 { mag } else { mag * 0.006 }
    }
    // 1/4λ 垂直 + 地网：地网改善地面损耗，主瓣比无地网垂直更低、更集中（指数 >1）。
    // 注意指数必须大于 1 才是「更集中」；小于 1 会把波束展宽（0.9 时 cos30°=0.866
    // 反而抬到 0.879，离轴增益比 cos θ 更高）。
    PatternKind::VerticalGP => t.cos().powf(1.15),
  }
}

/// 计算一组采样点，返回 `(角度°, 归一化增益)`，闭合一圈（首尾重合）。
///
/// 方位角图角度跨 360°，仰角图跨 90°。
#[must_use]
pub fn pattern(kind: PatternKind, steps: usize) -> Vec<(f64, f64)> {
  let steps = steps.max(4);
  let span = if kind.is_elevation() { 90.0 } else { 360.0 };
  let mut out = Vec::with_capacity(steps + 1);
  for i in 0..=steps {
    let theta = span * i as f64 / steps as f64;
    out.push((theta, gain(kind, theta)));
  }
  out
}

/// 水平偶极子的仰角方向图（考虑地面镜像反射，架高可调）。
///
/// `height_wl` 为架高（波长比，约 0.1–1.0），`theta_deg` 为仰角（0–90°，0°=地平线）。
/// 主瓣位置随架高变化：低架（0.25λ）主瓣在天顶，0.5λ 时约 30°，高架（1λ）约 14.5°。
#[must_use]
pub fn dipole_elevation(theta_deg: f64, height_wl: f64) -> f64 {
  let h = std::f64::consts::TAU * height_wl;
  let theta = theta_deg.to_radians();
  (h * theta.sin()).sin().abs()
}

/// 生成架高可调的偶极子仰角图采样点（角度 0–90°）。
#[must_use]
pub fn elevation_pattern(height_wl: f64, steps: usize) -> Vec<(f64, f64)> {
  let steps = steps.max(4);
  let mut out = Vec::with_capacity(steps + 1);
  for i in 0..=steps {
    let theta = 90.0 * i as f64 / steps as f64;
    out.push((theta, dipole_elevation(theta, height_wl)));
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dipole_e_peaks_perpendicular() {
    assert!((gain(PatternKind::DipoleE, 90.0) - 1.0).abs() < 1e-9);
    assert!(gain(PatternKind::DipoleE, 0.0) < 1e-9);
    assert!(gain(PatternKind::DipoleE, 180.0) < 1e-9);
  }

  #[test]
  fn dipole_h_is_omnidirectional() {
    for t in [0.0, 45.0, 90.0, 180.0, 270.0] {
      assert!((gain(PatternKind::DipoleH, t) - 1.0).abs() < 1e-9);
    }
  }

  #[test]
  fn yagi_has_front_to_back_ratio() {
    assert!((gain(PatternKind::Yagi, 0.0) - 1.0).abs() < 1e-9);
    let back = gain(PatternKind::Yagi, 180.0);
    assert!(back > 0.0 && back < 0.05);
  }

  #[test]
  fn vertical_peaks_at_low_angle() {
    assert!((gain(PatternKind::Vertical, 0.0) - 1.0).abs() < 1e-9);
    assert!(gain(PatternKind::Vertical, 90.0) < 1e-9);
    assert!(gain(PatternKind::Vertical, 30.0) > gain(PatternKind::Vertical, 70.0));
  }

  #[test]
  fn dipole_elevation_peaks_at_30_for_half_wave() {
    // 架高 0.5λ 时主瓣约 30°。
    assert!((gain(PatternKind::DipoleEl, 30.0) - 1.0).abs() < 1e-9);
    assert!(gain(PatternKind::DipoleEl, 0.0) < 1e-9);
    assert!(gain(PatternKind::DipoleEl, 90.0) < 1e-9);
  }

  #[test]
  fn dipole_elevation_peak_moves_with_height() {
    // 低架（0.25λ）主瓣在天顶（90°）。
    assert!((dipole_elevation(90.0, 0.25) - 1.0).abs() < 1e-9);
    // 0.5λ 主瓣约 30°。
    assert!((dipole_elevation(30.0, 0.5) - 1.0).abs() < 1e-9);
    // 高架（1λ）主瓣约 14.5°（低仰角）。
    assert!((dipole_elevation(14.5, 1.0) - 1.0).abs() < 1e-3);
    assert!(dipole_elevation(90.0, 1.0) < 1e-9);
  }

  #[test]
  fn elevation_pattern_spans_0_to_90() {
    let pts = elevation_pattern(0.5, 18);
    assert_eq!(pts.first().unwrap().0, 0.0);
    assert_eq!(pts.last().unwrap().0, 90.0);
  }

  #[test]
  fn loop_peaks_overhead() {
    assert!((gain(PatternKind::Loop, 90.0) - 1.0).abs() < 1e-9);
    assert!(gain(PatternKind::Loop, 0.0) < 1e-9);
  }

  #[test]
  fn shaped_loops_are_nearly_omnidirectional() {
    for kind in [PatternKind::SquareLoop, PatternKind::DeltaLoop] {
      for t in [0.0, 30.0, 60.0, 90.0, 180.0, 270.0] {
        let g = gain(kind, t);
        assert!((0.8..=1.0).contains(&g), "{kind:?} @ {t}° = {g}");
      }
    }
    // 方形环在 0° 最强、45° 最弱（4 重对称）。
    assert!(gain(PatternKind::SquareLoop, 0.0) > gain(PatternKind::SquareLoop, 45.0));
  }

  #[test]
  fn pattern_is_closed_loop() {
    let pts = pattern(PatternKind::DipoleE, 36);
    assert_eq!(pts.first().unwrap().0, 0.0);
    assert_eq!(pts.last().unwrap().0, 360.0);
    let elev = pattern(PatternKind::Vertical, 18);
    assert_eq!(elev.first().unwrap().0, 0.0);
    assert_eq!(elev.last().unwrap().0, 90.0);
  }

  #[test]
  fn more_yagi_elements_sharpen_lobe() {
    assert!((gain(PatternKind::Yagi2, 0.0) - 1.0).abs() < 1e-9);
    assert!((gain(PatternKind::Yagi4, 0.0) - 1.0).abs() < 1e-9);
    // 后瓣随单元数增加而减小：四单元 < 三单元 < 二单元。
    assert!(gain(PatternKind::Yagi4, 180.0) < gain(PatternKind::Yagi, 180.0));
    assert!(gain(PatternKind::Yagi, 180.0) < gain(PatternKind::Yagi2, 180.0));
    assert!(!PatternKind::Yagi4.is_elevation());
  }

  #[test]
  fn vertical_ground_plane_peaks_low() {
    assert!((gain(PatternKind::VerticalGP, 0.0) - 1.0).abs() < 1e-9);
    assert!(gain(PatternKind::VerticalGP, 90.0) < 1e-9);
    assert!(gain(PatternKind::VerticalGP, 30.0) > gain(PatternKind::VerticalGP, 70.0));
    assert!(PatternKind::VerticalGP.is_elevation());
  }
}

//! 路径剖面：沿 A→B 的一串高程点，逐点算地球曲率与第一菲涅尔区净空，找出最危险的一点。
//!
//! 在 [`crate::fresnel`] 的单障碍物模型上推广到任意多个采样点：每个内部点都当作一个刃形峰，
//! 用同一组公式求净空，取「净空 / 第一菲涅尔区半径」最小的那一点作为整条路径的结论。
//!
//! 输入文本每行一个点：`距离(km) 海拔(m)`，分隔符可以是逗号、分号、空白；空行与 `#` 开头的行忽略。
//!
//! 建模边界：
//! - 只在给定的采样点上判定，点与点之间的地形按「没有隆起」处理 —— 采样越密越可信；
//! - 至少要有 3 个点（A、B 和中间至少一点）：只有两端时地面形状未知，无法判断曲率是否挡住路径，
//!   所以不给结论，要求补一个中点；
//! - 高度与海拔用同一基准；不含植被、建筑、地面反射与衍射附加损耗。

use crate::fresnel::{
  FULL_CLEARANCE_RATIO, MIN_CLEARANCE_RATIO, Verdict, earth_bulge_m, fresnel_radius_m, wavelength_m,
};
use crate::math::positive;

/// 最多接受的采样点数：够描述几百公里的路径，又不会让页面卡住。
pub const MAX_POINTS: usize = 1000;

/// 一个剖面采样点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfilePoint {
  /// 距离（km），沿路径单调递增；起点不必为 0。
  pub dist_km: f64,
  /// 地面海拔（m）。
  pub elev_m: f64,
}

/// 文本解析失败的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
  /// 一行里不是恰好两个字段。
  FieldCount,
  /// 字段不是有限的数字。
  NotANumber,
}

/// 文本解析错误，`line` 是 1 起算的行号。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
  pub line: usize,
  pub kind: ParseErrorKind,
}

/// 把多行文本解析成采样点。空文本得到空列表（由 [`analyze`] 负责判定点数）。
pub fn parse_points(text: &str) -> Result<Vec<ProfilePoint>, ParseError> {
  let mut points = Vec::new();
  for (i, raw) in text.lines().enumerate() {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
      continue;
    }
    let fields: Vec<&str> = line
      .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
      .filter(|f| !f.is_empty())
      .collect();
    let at = |kind| ParseError { line: i + 1, kind };
    let [d, e] = fields[..] else {
      return Err(at(ParseErrorKind::FieldCount));
    };
    let parse = |s: &str| {
      s.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| at(ParseErrorKind::NotANumber))
    };
    points.push(ProfilePoint {
      dist_km: parse(d)?,
      elev_m: parse(e)?,
    });
  }
  Ok(points)
}

/// 分析失败的原因（输入不足以下结论）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
  /// 少于 3 个点：缺中间的采样点，见模块文档。
  TooFewPoints,
  /// 超过 [`MAX_POINTS`]。
  TooManyPoints,
  /// 第 `n`（1 起算）个点的距离没有比前一个点更大。
  NotIncreasing(usize),
  /// 频率、K 或天线高度不合法。
  BadParameter,
}

/// 路径参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileParams {
  pub freq_mhz: f64,
  /// A 端（第一个点）天线离地高度（m），必须 ≥ 0。
  pub tx_height_m: f64,
  /// B 端（最后一个点）天线离地高度（m），必须 ≥ 0。
  pub rx_height_m: f64,
  /// 等效地球半径系数，标准大气取 [`crate::fresnel::K_STANDARD`]。
  pub k: f64,
}

/// 剖面上一个点的计算结果（距离已折算成「距 A 端」）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileRow {
  pub dist_km: f64,
  pub elev_m: f64,
  /// 地球曲率隆起（m）；两端为 0。
  pub bulge_m: f64,
  /// 地面加隆起后的等效高度（m）。
  pub terrain_effective_m: f64,
  /// 两端天线连线在此处的高度（m）。
  pub los_m: f64,
  /// 第一菲涅尔区半径（m）；两端为 0。
  pub fresnel1_m: f64,
  /// 视线到等效地面的净空（m）；两端是天线离地高度，故只在内部点有意义，两端为 `None`。
  pub clearance_m: Option<f64>,
  /// 净空 / 第一菲涅尔区半径；两端为 `None`。
  pub clearance_ratio: Option<f64>,
}

/// 整条路径的结论。
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileReport {
  pub rows: Vec<ProfileRow>,
  pub freq_mhz: f64,
  pub wavelength_m: f64,
  /// A、B 间地面距离（km）。
  pub total_km: f64,
  /// A、B 两端天线的海拔（地面海拔 + 天线高，m）。
  pub tx_abs_m: f64,
  pub rx_abs_m: f64,
  /// 最危险的内部点在 `rows` 里的下标（净空比最小；并列取靠近 A 端的）。
  pub worst_index: usize,
  pub worst_ratio: f64,
  pub verdict: Verdict,
  /// 相对 60% 线的最小余量（m）：正为有余，负为不足。
  pub margin_60_m: f64,
  /// 两端天线**同时**抬高多少米，才能让所有点至少过 60% 线 / 完整第一菲涅尔区 / 仅仅视线通；
  /// 已经满足时为 0。
  pub raise_60_m: f64,
  pub raise_full_m: f64,
  pub raise_los_m: f64,
}

/// 分析一条剖面。点要按距离严格递增，至少 3 个点；起点距离不必为 0。
pub fn analyze(
  points: &[ProfilePoint],
  params: &ProfileParams,
) -> Result<ProfileReport, ProfileError> {
  if points.len() < 3 {
    return Err(ProfileError::TooFewPoints);
  }
  if points.len() > MAX_POINTS {
    return Err(ProfileError::TooManyPoints);
  }
  let ProfileParams {
    freq_mhz,
    tx_height_m,
    rx_height_m,
    k,
  } = *params;
  if !positive(freq_mhz)
    || !positive(k)
    || !tx_height_m.is_finite()
    || !rx_height_m.is_finite()
    // 天线离地高度为负没有物理意义；地形海拔（elev）可以为负（低于海平面）。
    || tx_height_m < 0.0
    || rx_height_m < 0.0
  {
    return Err(ProfileError::BadParameter);
  }
  for (i, w) in points.windows(2).enumerate() {
    let valid = w[0].dist_km.is_finite()
      && w[1].dist_km.is_finite()
      && w[1].dist_km > w[0].dist_km
      && w[0].elev_m.is_finite()
      && w[1].elev_m.is_finite();
    if !valid {
      return Err(ProfileError::NotIncreasing(i + 2));
    }
  }

  let origin = points[0].dist_km;
  let total = points[points.len() - 1].dist_km - origin;
  let tx_abs = points[0].elev_m + tx_height_m;
  let rx_abs = points[points.len() - 1].elev_m + rx_height_m;
  let wavelength = wavelength_m(freq_mhz).ok_or(ProfileError::BadParameter)?;
  let last = points.len() - 1;

  let mut rows = Vec::with_capacity(points.len());
  for (i, p) in points.iter().enumerate() {
    let d1 = p.dist_km - origin;
    let los = tx_abs + (rx_abs - tx_abs) * d1 / total;
    if i == 0 || i == last {
      rows.push(ProfileRow {
        dist_km: d1,
        elev_m: p.elev_m,
        bulge_m: 0.0,
        terrain_effective_m: p.elev_m,
        los_m: los,
        fresnel1_m: 0.0,
        clearance_m: None,
        clearance_ratio: None,
      });
      continue;
    }
    let d2 = total - d1;
    let bulge = earth_bulge_m(d1, d2, k).ok_or(ProfileError::BadParameter)?;
    let fresnel = fresnel_radius_m(1, freq_mhz, d1, d2).ok_or(ProfileError::BadParameter)?;
    let effective = p.elev_m + bulge;
    let clearance = los - effective;
    rows.push(ProfileRow {
      dist_km: d1,
      elev_m: p.elev_m,
      bulge_m: bulge,
      terrain_effective_m: effective,
      los_m: los,
      fresnel1_m: fresnel,
      clearance_m: Some(clearance),
      clearance_ratio: Some(clearance / fresnel),
    });
  }

  let mut worst_index = 1;
  let mut worst_ratio = f64::INFINITY;
  let mut margin_60 = f64::INFINITY;
  let (mut raise_60, mut raise_full, mut raise_los) = (0.0_f64, 0.0_f64, 0.0_f64);
  for (i, r) in rows.iter().enumerate() {
    let (Some(c), Some(ratio)) = (r.clearance_m, r.clearance_ratio) else {
      continue;
    };
    if ratio < worst_ratio {
      worst_ratio = ratio;
      worst_index = i;
    }
    margin_60 = margin_60.min(c - MIN_CLEARANCE_RATIO * r.fresnel1_m);
    raise_60 = raise_60.max(MIN_CLEARANCE_RATIO * r.fresnel1_m - c);
    raise_full = raise_full.max(FULL_CLEARANCE_RATIO * r.fresnel1_m - c);
    raise_los = raise_los.max(-c);
  }
  if ![worst_ratio, margin_60, raise_60, raise_full, raise_los]
    .iter()
    .all(|v| v.is_finite())
  {
    return Err(ProfileError::BadParameter);
  }

  Ok(ProfileReport {
    rows,
    freq_mhz,
    wavelength_m: wavelength,
    total_km: total,
    tx_abs_m: tx_abs,
    rx_abs_m: rx_abs,
    worst_index,
    worst_ratio,
    verdict: Verdict::from_ratio(worst_ratio),
    margin_60_m: margin_60,
    raise_60_m: raise_60,
    raise_full_m: raise_full,
    raise_los_m: raise_los,
  })
}

/// 菲涅尔区包络上的一点：距 A 端 `dist_km` 处，视线高度 `los_m` 与第一菲涅尔区半径 `fresnel1_m`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvelopePoint {
  pub dist_km: f64,
  pub los_m: f64,
  pub fresnel1_m: f64,
}

/// 沿整条路径等间距采样 `steps + 1` 个点的第一菲涅尔区包络（两端半径为 0），用来画连续的椭圆带；
/// 采样与剖面点的疏密无关。`steps` 至少取 2。
#[must_use]
pub fn zone_envelope(report: &ProfileReport, steps: usize) -> Vec<EnvelopePoint> {
  let steps = steps.max(2);
  let total = report.total_km;
  (0..=steps)
    .map(|i| {
      let d1 = total * i as f64 / steps as f64;
      let fresnel = if i == 0 || i == steps {
        0.0
      } else {
        fresnel_radius_m(1, report.freq_mhz, d1, total - d1).unwrap_or(0.0)
      };
      EnvelopePoint {
        dist_km: d1,
        los_m: report.tx_abs_m + (report.rx_abs_m - report.tx_abs_m) * d1 / total,
        fresnel1_m: fresnel,
      }
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fresnel::{self, K_STANDARD, PathInput};

  fn p(d: f64, e: f64) -> ProfilePoint {
    ProfilePoint {
      dist_km: d,
      elev_m: e,
    }
  }

  fn params() -> ProfileParams {
    ProfileParams {
      freq_mhz: 145.0,
      tx_height_m: 100.0,
      rx_height_m: 100.0,
      k: K_STANDARD,
    }
  }

  fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
  }

  #[test]
  fn single_interior_point_agrees_with_the_single_obstacle_model() {
    for peak in [0.0, 40.0, 200.0] {
      let profile = analyze(&[p(0.0, 0.0), p(15.0, peak), p(30.0, 0.0)], &params()).unwrap();
      let single = fresnel::analyze(&PathInput {
        freq_mhz: 145.0,
        dist_km: 30.0,
        tx_height_m: 100.0,
        rx_height_m: 100.0,
        k: K_STANDARD,
        obstacle_km: 15.0,
        obstacle_height_m: peak,
      })
      .unwrap();
      assert!(
        near(profile.worst_ratio, single.clearance_ratio, 1e-9),
        "peak {peak}"
      );
      assert!(near(profile.margin_60_m, single.margin_60_m, 1e-9));
      assert_eq!(profile.verdict, single.verdict);
      assert!(near(profile.rows[1].bulge_m, single.bulge_m, 1e-9));
    }
  }

  #[test]
  fn the_worst_point_is_the_tightest_not_the_tallest() {
    // 5 km 处 60 m：净空比 ≈ 0.35；15 km 处 55 m：净空比 ≈ 0.26。
    // 靠近 A 端的小丘更高，但那里的菲涅尔区窄；中点的矮丘处区最宽，净空比反而更差。
    let pts = [
      p(0.0, 0.0),
      p(5.0, 60.0),
      p(15.0, 55.0),
      p(25.0, 0.0),
      p(30.0, 0.0),
    ];
    let r = analyze(&pts, &params()).unwrap();
    let ratios: Vec<f64> = r.rows.iter().filter_map(|x| x.clearance_ratio).collect();
    assert_eq!(ratios.len(), 3);
    assert_eq!(r.worst_index, 2);
    assert!(near(
      r.worst_ratio,
      ratios.iter().cloned().fold(f64::INFINITY, f64::min),
      1e-12
    ));
  }

  #[test]
  fn ties_resolve_to_the_point_nearest_a() {
    let pts = [p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0), p(30.0, 0.0)];
    // 对称路径：10 km 与 20 km 处净空比相同。
    let r = analyze(&pts, &params()).unwrap();
    assert_eq!(r.worst_index, 1);
  }

  #[test]
  fn raising_both_antennas_by_the_reported_amount_reaches_each_target() {
    let pts = [
      p(0.0, 0.0),
      p(10.0, 80.0),
      p(15.0, 120.0),
      p(22.0, 60.0),
      p(30.0, 0.0),
    ];
    let base = analyze(&pts, &params()).unwrap();
    assert!(base.raise_60_m > 0.0 && base.raise_full_m > base.raise_60_m);
    assert!(base.raise_los_m < base.raise_60_m);

    let lifted = |h: f64| {
      analyze(
        &pts,
        &ProfileParams {
          tx_height_m: 100.0 + h,
          rx_height_m: 100.0 + h,
          ..params()
        },
      )
      .unwrap()
    };
    assert!(near(
      lifted(base.raise_60_m).worst_ratio,
      MIN_CLEARANCE_RATIO,
      1e-9
    ));
    assert!(near(lifted(base.raise_60_m).raise_60_m, 0.0, 1e-9));
    assert!(near(
      lifted(base.raise_full_m).worst_ratio,
      FULL_CLEARANCE_RATIO,
      1e-9
    ));
    // 只要视线通：此时最差点净空为 0，仍算 Partial（≥ 0）。
    assert!(near(lifted(base.raise_los_m).worst_ratio, 0.0, 1e-9));
  }

  #[test]
  fn nothing_to_raise_when_the_path_is_clear() {
    let r = analyze(
      &[p(0.0, 0.0), p(15.0, 0.0), p(30.0, 0.0)],
      &ProfileParams {
        tx_height_m: 300.0,
        rx_height_m: 300.0,
        ..params()
      },
    )
    .unwrap();
    assert_eq!(r.verdict, Verdict::Full);
    assert_eq!(
      (r.raise_60_m, r.raise_full_m, r.raise_los_m),
      (0.0, 0.0, 0.0)
    );
    assert!(r.margin_60_m > 0.0);
  }

  #[test]
  fn start_offset_does_not_change_the_result() {
    let a = analyze(&[p(0.0, 0.0), p(12.0, 50.0), p(30.0, 10.0)], &params()).unwrap();
    let b = analyze(&[p(100.0, 0.0), p(112.0, 50.0), p(130.0, 10.0)], &params()).unwrap();
    assert!(near(a.worst_ratio, b.worst_ratio, 1e-12));
    assert!(near(b.total_km, 30.0, 1e-12));
    assert_eq!(b.rows[0].dist_km, 0.0);
  }

  #[test]
  fn antenna_heights_are_above_each_endpoint_ground() {
    let r = analyze(&[p(0.0, 500.0), p(15.0, 520.0), p(30.0, 300.0)], &params()).unwrap();
    assert_eq!(r.tx_abs_m, 600.0);
    assert_eq!(r.rx_abs_m, 400.0);
    assert!(near(r.rows[1].los_m, 500.0, 1e-9));
  }

  #[test]
  fn endpoints_have_no_zone_or_clearance() {
    let r = analyze(&[p(0.0, 0.0), p(15.0, 0.0), p(30.0, 0.0)], &params()).unwrap();
    for i in [0, 2] {
      assert_eq!(r.rows[i].fresnel1_m, 0.0);
      assert_eq!(r.rows[i].bulge_m, 0.0);
      assert_eq!(r.rows[i].clearance_ratio, None);
    }
    assert!(r.rows[1].fresnel1_m > 100.0);
  }

  #[test]
  fn rejects_what_it_cannot_judge() {
    let ok = [p(0.0, 0.0), p(15.0, 0.0), p(30.0, 0.0)];
    assert_eq!(
      analyze(&ok[..2], &params()),
      Err(ProfileError::TooFewPoints)
    );
    assert_eq!(analyze(&[], &params()), Err(ProfileError::TooFewPoints));
    assert_eq!(
      analyze(&[p(0.0, 0.0), p(15.0, 0.0), p(15.0, 0.0)], &params()),
      Err(ProfileError::NotIncreasing(3))
    );
    assert_eq!(
      analyze(&[p(0.0, 0.0), p(20.0, 0.0), p(10.0, 0.0)], &params()),
      Err(ProfileError::NotIncreasing(3))
    );
    assert_eq!(
      analyze(&[p(0.0, f64::NAN), p(15.0, 0.0), p(30.0, 0.0)], &params()),
      Err(ProfileError::NotIncreasing(2))
    );
    for bad in [
      ProfileParams {
        freq_mhz: 0.0,
        ..params()
      },
      ProfileParams {
        k: -1.0,
        ..params()
      },
      ProfileParams {
        tx_height_m: f64::NAN,
        ..params()
      },
      // 天线埋在地下没有物理意义；地形海拔为负（低于海平面）不在其列。
      ProfileParams {
        tx_height_m: -1.0,
        ..params()
      },
      ProfileParams {
        rx_height_m: -0.5,
        ..params()
      },
    ] {
      assert_eq!(analyze(&ok, &bad), Err(ProfileError::BadParameter));
    }
    let many: Vec<_> = (0..=MAX_POINTS).map(|i| p(i as f64, 0.0)).collect();
    assert_eq!(analyze(&many, &params()), Err(ProfileError::TooManyPoints));
  }

  #[test]
  fn parses_commas_spaces_semicolons_tabs_comments_and_blank_lines() {
    let text = "# km, m\n0, 12.5\n\n 5 80 \n10;95\n15\t60\r\n";
    assert_eq!(
      parse_points(text).unwrap(),
      vec![p(0.0, 12.5), p(5.0, 80.0), p(10.0, 95.0), p(15.0, 60.0)]
    );
    assert_eq!(parse_points("").unwrap(), vec![]);
    assert_eq!(parse_points("  \n# only a comment").unwrap(), vec![]);
    // 负海拔（低于海平面）是合法的。
    assert_eq!(parse_points("1, -20").unwrap(), vec![p(1.0, -20.0)]);
  }

  #[test]
  fn parse_errors_carry_the_one_based_line_number() {
    assert_eq!(
      parse_points("0 0\n5\n"),
      Err(ParseError {
        line: 2,
        kind: ParseErrorKind::FieldCount
      })
    );
    assert_eq!(
      parse_points("# c\n0 0\n5 6 7"),
      Err(ParseError {
        line: 3,
        kind: ParseErrorKind::FieldCount
      })
    );
    assert_eq!(
      parse_points("0 0\n5 abc"),
      Err(ParseError {
        line: 2,
        kind: ParseErrorKind::NotANumber
      })
    );
    assert_eq!(
      parse_points("0 inf"),
      Err(ParseError {
        line: 1,
        kind: ParseErrorKind::NotANumber
      })
    );
    assert_eq!(
      parse_points("NaN 1"),
      Err(ParseError {
        line: 1,
        kind: ParseErrorKind::NotANumber
      })
    );
  }

  #[test]
  fn envelope_is_an_ellipse_that_pinches_at_both_ends() {
    let r = analyze(&[p(0.0, 0.0), p(15.0, 0.0), p(30.0, 0.0)], &params()).unwrap();
    let env = zone_envelope(&r, 30);
    assert_eq!(env.len(), 31);
    assert_eq!((env[0].fresnel1_m, env[30].fresnel1_m), (0.0, 0.0));
    assert_eq!(env[0].dist_km, 0.0);
    assert!(near(env[30].dist_km, 30.0, 1e-12));
    // 第 15 个采样点正好落在 15 km：与剖面行的半径一致，且是整条包络的最大值。
    assert!(near(env[15].fresnel1_m, r.rows[1].fresnel1_m, 1e-9));
    assert!(
      env
        .iter()
        .all(|e| e.fresnel1_m <= env[15].fresnel1_m + 1e-9)
    );
    // 对称路径：两侧对称。
    assert!(near(env[5].fresnel1_m, env[25].fresnel1_m, 1e-9));
    // 视线高度与剖面行一致。
    assert!(near(env[15].los_m, r.rows[1].los_m, 1e-9));
    // 步数下限。
    assert_eq!(zone_envelope(&r, 0).len(), 3);
  }
}

//! 单刃绕射损耗：一座山脊挡在两端之间时，信号翻过去大约掉多少分贝。
//!
//! 路径剖面（[`crate::path_profile`]）只回答「挡没挡住、侵入第一菲涅尔区多深」，这里补上
//! VHF / UHF 真正要的那个数。**只算一个刃**——路径上最挡的那一点，把它当成无限薄的刃形障碍：
//!
//! ```text
//! ν = h · √( 2·(d₁+d₂) / (λ·d₁·d₂) )          h：刃顶高出两端连线的高度（m），挡住为正
//!   = −√2 · 净空 / 第一菲涅尔区半径             （与 `fresnel` 的净空比是同一个量）
//!
//! J(ν) = 6.9 + 20·log₁₀( √((ν−0.1)²+1) + ν − 0.1 )   dB，ν > −0.78；ν ≤ −0.78 取 0
//! ```
//!
//! `J(ν)` 是刃形绕射损耗的闭式近似（ν = 0，即视线刚好擦过刃顶，是 6.02 dB；ν = −0.78 处接近 0）。
//! 它的参照是**精确的菲涅尔积分**：`|E/E₀|² = ((½−C(ν))² + (½−S(ν))²) / 2`，测试里用数值积分
//! 另算一遍对拍，两者在 −0.78 < ν ≤ 10 时最大相差约 0.12 dB（ν ≈ 4.6 处），见 `tests::closed_form_matches_the_exact_integral`。
//!
//! 建模边界：
//! - 只有**一个**刃。几座山头相继挡住时，总损耗比最高那个的单刃值大（简单的 Deygout 叠加也只是经验
//!   法），这里不算，只提示有几个采样点挡住了视线；
//! - 刃形是理想模型：真实山顶是有曲率的圆弧，损耗比刃形大；植被、建筑与地面反射都不算；
//! - ν ≤ −0.78（净空超过约 0.55 个第一菲涅尔区）时按 0 dB，不计高于自由空间的那点起伏；
//! - 山脊在两端之间的位置、高度全部来自 [`crate::path_profile`] 的采样点：采样稀了，真正的峰顶可能
//!   落在两个点之间，损耗会被低估。

use crate::path_profile::ProfileReport;

/// 闭式近似的有效下界：ν 不大于它就取 0 dB。
pub const NU_FLOOR: f64 = -0.78;

/// 刃形绕射损耗（dB，≥ 0）；`nu` 不是有限数时返回 `None`。
#[must_use]
pub fn knife_edge_loss_db(nu: f64) -> Option<f64> {
  if !nu.is_finite() {
    return None;
  }
  if nu <= NU_FLOOR {
    return Some(0.0);
  }
  let t = nu - 0.1;
  Some(6.9 + 20.0 * ((t * t + 1.0).sqrt() + t).log10())
}

/// 菲涅尔—基尔霍夫衍射参数 ν：`h_m` 是刃顶高出两端连线的高度（挡住为正），`d1_m` / `d2_m` 是刃到两端的距离，
/// `wavelength_m` 是波长。任一距离或波长不是正的有限数、`h_m` 不是有限数时返回 `None`。
#[must_use]
pub fn fresnel_parameter(h_m: f64, d1_m: f64, d2_m: f64, wavelength_m: f64) -> Option<f64> {
  let positive = |v: f64| v.is_finite() && v > 0.0;
  if !h_m.is_finite() || !positive(d1_m) || !positive(d2_m) || !positive(wavelength_m) {
    return None;
  }
  Some(h_m * (2.0 * (d1_m + d2_m) / (wavelength_m * d1_m * d2_m)).sqrt())
}

/// 路径上最挡的那个点的单刃绕射结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
  /// 该点在 [`ProfileReport::rows`] 里的下标。
  pub index: usize,
  /// 该点距 A 端的距离（km）。
  pub dist_km: f64,
  /// 刃顶高出两端连线的高度（m）；负数是视线从上方越过。
  pub height_m: f64,
  /// 衍射参数 ν。
  pub nu: f64,
  /// 单刃绕射损耗（dB）。
  pub loss_db: f64,
  /// 视线被挡住（净空 < 0）的内部采样点数，含 `index` 这一个；不挡时为 0。
  pub blocking_points: usize,
}

/// 从一份路径剖面里取出 ν 最大的内部点，算它的单刃绕射损耗。
///
/// 地球曲率隆起已经包含在 [`crate::path_profile::ProfileRow::clearance_m`] 里（它是视线到「地面 + 隆起」
/// 的距离），所以 `h = −净空`。ν 并列时取靠近 A 端的。
#[must_use]
pub fn dominant_edge(report: &ProfileReport) -> Option<Edge> {
  let mut best: Option<(usize, f64, f64)> = None; // (下标, 高度 h, ν)
  let mut blocking = 0;
  for (i, row) in report.rows.iter().enumerate() {
    let Some(clearance) = row.clearance_m else {
      continue;
    };
    let d1 = row.dist_km * 1000.0;
    let d2 = (report.total_km - row.dist_km) * 1000.0;
    let h = -clearance;
    let nu = fresnel_parameter(h, d1, d2, report.wavelength_m)?;
    if clearance < 0.0 {
      blocking += 1;
    }
    if best.is_none_or(|(_, _, v)| nu > v) {
      best = Some((i, h, nu));
    }
  }
  let (index, height_m, nu) = best?;
  Some(Edge {
    index,
    dist_km: report.rows[index].dist_km,
    height_m,
    nu,
    loss_db: knife_edge_loss_db(nu)?,
    blocking_points: blocking,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fresnel::K_STANDARD;
  use crate::path_profile::{ProfileParams, ProfilePoint, analyze};

  /// 精确的刃形绕射损耗：数值积分菲涅尔积分。**与被测的闭式近似没有任何共用代码**。
  ///
  /// `|E/E₀|² = ((½−C(ν))² + (½−S(ν))²) / 2`，`C` 与 `S` 各是 `∫₀^ν cos / sin(πt²/2) dt`，
  /// 这里用复合辛普森法（步长 ≤ 2e-5，被积函数每个振荡周期有上百个点）。
  fn exact_loss_db(nu: f64) -> f64 {
    let steps = ((nu.abs() / 2e-5) as usize).max(2000) & !1; // 偶数
    let h = nu / steps as f64;
    let f = |t: f64, trig: fn(f64) -> f64| trig(std::f64::consts::FRAC_PI_2 * t * t);
    let simpson = |trig: fn(f64) -> f64| {
      let mut s = f(0.0, trig) + f(nu, trig);
      for i in 1..steps {
        s += f(i as f64 * h, trig) * if i % 2 == 1 { 4.0 } else { 2.0 };
      }
      s * h / 3.0
    };
    let (c, s) = (simpson(f64::cos), simpson(f64::sin));
    let power = ((0.5 - c).powi(2) + (0.5 - s).powi(2)) / 2.0;
    -10.0 * power.log10()
  }

  #[test]
  fn the_reference_integral_is_itself_right() {
    // 参照要先自证：ν = 0 时 `|E/E₀| = ½`（6.0206 dB），这是教科书的精确值；
    // ν 很大时趋近 `20·log₁₀(π√2·ν) ≈ 12.96 + 20·log₁₀ν`（刃后深阴影的渐近式，
    // 即常说的「13 + 20·log ν」）。
    assert!((exact_loss_db(0.0) - 20.0 * 2.0_f64.log10()).abs() < 1e-6);
    for nu in [10.0, 20.0] {
      let asymptote = 20.0 * (std::f64::consts::PI * 2.0_f64.sqrt() * nu).log10();
      assert!(
        (exact_loss_db(nu) - asymptote).abs() < 0.05,
        "ν = {nu}：{} vs {asymptote}",
        exact_loss_db(nu)
      );
    }
  }

  #[test]
  fn closed_form_matches_the_exact_integral() {
    let mut worst = (0.0_f64, 0.0_f64);
    let mut nu = NU_FLOOR + 0.01;
    while nu <= 10.0 {
      let diff = (knife_edge_loss_db(nu).unwrap() - exact_loss_db(nu)).abs();
      if diff > worst.0 {
        worst = (diff, nu);
      }
      nu += 0.05;
    }
    // 实测最大偏差 0.123 dB（ν ≈ 4.58）；上限 0.2 dB。
    assert!(worst.0 < 0.2, "ν = {} 处偏差 {:.3} dB", worst.1, worst.0);
  }

  #[test]
  fn grazing_incidence_is_six_db_and_the_floor_is_continuous() {
    // 视线刚好擦过刃顶（ν = 0）：精确值 6.0206 dB，闭式给 6.03 dB。
    assert!((knife_edge_loss_db(0.0).unwrap() - 6.02).abs() < 0.02);
    // 下界处连续：ν = −0.78 取 0，略高一点也接近 0（不会跳到几 dB）。
    assert_eq!(knife_edge_loss_db(NU_FLOOR), Some(0.0));
    assert!(knife_edge_loss_db(NU_FLOOR + 0.001).unwrap() < 0.05);
    // 净空很大：0。
    assert_eq!(knife_edge_loss_db(-3.0), Some(0.0));
  }

  #[test]
  fn loss_grows_with_nu_and_follows_the_deep_shadow_slope() {
    let mut prev = -1.0;
    for i in 0..400 {
      let nu = -0.7 + f64::from(i) * 0.03;
      let v = knife_edge_loss_db(nu).unwrap();
      assert!(v >= prev, "ν = {nu} 处不单调");
      prev = v;
    }
    // 深阴影区：ν 每翻一倍，损耗约加 6 dB。
    let d = knife_edge_loss_db(20.0).unwrap() - knife_edge_loss_db(10.0).unwrap();
    assert!((d - 6.02).abs() < 0.1, "{d}");
  }

  #[test]
  fn non_finite_input_is_rejected() {
    assert_eq!(knife_edge_loss_db(f64::NAN), None);
    assert_eq!(knife_edge_loss_db(f64::INFINITY), None);
    assert_eq!(fresnel_parameter(10.0, 0.0, 1000.0, 2.0), None);
    assert_eq!(fresnel_parameter(10.0, 1000.0, -5.0, 2.0), None);
    assert_eq!(fresnel_parameter(10.0, 1000.0, 1000.0, 0.0), None);
    assert_eq!(fresnel_parameter(f64::NAN, 1000.0, 1000.0, 2.0), None);
  }

  #[test]
  fn fresnel_parameter_is_the_textbook_geometry() {
    // 手算：h = 20 m、d1 = 10 km、d2 = 20 km、λ = 2 m（150 MHz）：
    // ν = 20 · √(2·30000 / (2·10000·20000)) = 20 · √0.00015 = 0.24495
    let nu = fresnel_parameter(20.0, 10_000.0, 20_000.0, 2.0).unwrap();
    assert!((nu - 0.244_949).abs() < 1e-5, "{nu}");
    // 高度反号，ν 反号；两端对调，ν 不变。
    assert!((fresnel_parameter(-20.0, 10_000.0, 20_000.0, 2.0).unwrap() + nu).abs() < 1e-12);
    assert!((fresnel_parameter(20.0, 20_000.0, 10_000.0, 2.0).unwrap() - nu).abs() < 1e-12);
  }

  fn p(d: f64, e: f64) -> ProfilePoint {
    ProfilePoint {
      dist_km: d,
      elev_m: e,
    }
  }

  fn params(freq: f64, tx: f64, rx: f64) -> ProfileParams {
    ProfileParams {
      freq_mhz: freq,
      tx_height_m: tx,
      rx_height_m: rx,
      k: K_STANDARD,
    }
  }

  #[test]
  fn a_ridge_on_the_line_of_sight_costs_six_db() {
    // 取 k 极大（地球近乎平的），两端天线 30 m、山脊顶比视线只低 1 cm：视线擦过刃顶，ν ≈ 0。
    let mut prm = params(145.0, 30.0, 30.0);
    prm.k = 1.0e9;
    let report = analyze(&[p(0.0, 0.0), p(15.0, 29.99), p(30.0, 0.0)], &prm).unwrap();
    let edge = dominant_edge(&report).unwrap();
    // 峰顶比视线低 1 cm：ν ≈ −0.0002，损耗仍是 6.0 dB 上下。
    assert!(edge.nu.abs() < 0.001, "{}", edge.nu);
    assert!((edge.loss_db - 6.02).abs() < 0.05, "{}", edge.loss_db);
    assert_eq!(edge.index, 1);
    assert_eq!(edge.blocking_points, 0, "视线从上方擦过不算挡住");
  }

  #[test]
  fn profile_nu_matches_the_independent_geometry() {
    // 山脊 310 m（示例剖面），两端 120 m / 130 m 地面 + 30 m 天线。ν 用另一条路算：
    // 视线高度线性插值 + 曲率隆起 → h → 教科书公式 ν = h·√(2(d1+d2)/(λ d1 d2))。
    let pts = [
      p(0.0, 120.0),
      p(5.0, 180.0),
      p(15.0, 310.0),
      p(25.0, 170.0),
      p(30.0, 130.0),
    ];
    let prm = params(145.0, 30.0, 30.0);
    let report = analyze(&pts, &prm).unwrap();
    let edge = dominant_edge(&report).unwrap();
    let lambda = 299_792_458.0 / 145.0e6;
    let (d1, d2) = (15_000.0_f64, 15_000.0_f64);
    let los = (120.0 + 30.0) + ((130.0 + 30.0) - (120.0 + 30.0)) * 0.5;
    let bulge = 15.0 * 15.0 / (2.0 * K_STANDARD * 6371.0) * 1000.0;
    let h = 310.0 + bulge - los;
    let nu = h * (2.0 * (d1 + d2) / (lambda * d1 * d2)).sqrt();
    assert_eq!(edge.index, 2);
    assert!((edge.nu - nu).abs() < 1e-9, "{} vs {nu}", edge.nu);
    assert!((edge.height_m - h).abs() < 1e-9);
    // 与「净空比」同一个量：ν = −√2 · 净空 / 第一菲涅尔区半径。
    let row = report.rows[2];
    assert!((edge.nu + 2f64.sqrt() * row.clearance_ratio.unwrap()).abs() < 1e-9);
    // 损耗再对着精确积分核一遍。
    assert!((edge.loss_db - exact_loss_db(edge.nu)).abs() < 0.2);
  }

  #[test]
  fn the_dominant_edge_is_the_largest_nu_not_the_highest_point() {
    // 靠近 B 端的低山比中间的高山更「挡」（ν 看的是净空相对菲涅尔半径，近端的菲涅尔区窄）。
    let pts = [
      p(0.0, 0.0),
      p(14.0, 60.0),
      p(15.0, 20.0),
      p(28.0, 55.0),
      p(30.0, 0.0),
    ];
    let report = analyze(&pts, &params(145.0, 30.0, 30.0)).unwrap();
    let edge = dominant_edge(&report).unwrap();
    let best = (1..report.rows.len() - 1)
      .max_by(|&a, &b| {
        let nu = |i: usize| -report.rows[i].clearance_ratio.unwrap();
        nu(a).total_cmp(&nu(b))
      })
      .unwrap();
    assert_eq!(edge.index, best);
    assert_eq!(edge.index, 3, "28 km 处的 55 m 比 14 km 处的 60 m 更挡");
  }

  #[test]
  fn a_clear_path_costs_nothing_and_counts_no_blockers() {
    let report = analyze(
      &[p(0.0, 0.0), p(15.0, 0.0), p(30.0, 0.0)],
      &params(145.0, 100.0, 100.0),
    )
    .unwrap();
    let edge = dominant_edge(&report).unwrap();
    assert!(edge.nu < NU_FLOOR, "{}", edge.nu);
    assert_eq!(edge.loss_db, 0.0);
    assert_eq!(edge.blocking_points, 0);
  }

  #[test]
  fn blocking_points_are_counted() {
    // 示例剖面：多个采样点同时高过视线。
    let pts = [
      p(0.0, 120.0),
      p(5.0, 180.0),
      p(10.0, 260.0),
      p(15.0, 310.0),
      p(20.0, 240.0),
      p(25.0, 170.0),
      p(30.0, 130.0),
    ];
    let report = analyze(&pts, &params(145.0, 30.0, 30.0)).unwrap();
    let blocking = report
      .rows
      .iter()
      .filter(|r| r.clearance_m.is_some_and(|c| c < 0.0))
      .count();
    let edge = dominant_edge(&report).unwrap();
    assert_eq!(edge.blocking_points, blocking);
    assert!(blocking >= 2);
  }

  #[test]
  fn higher_frequency_loses_more_over_the_same_ridge() {
    // 同一座山，频率越高波长越短、ν 越大、损耗越大。
    let pts = [p(0.0, 0.0), p(15.0, 80.0), p(30.0, 0.0)];
    let loss = |f| {
      dominant_edge(&analyze(&pts, &params(f, 30.0, 30.0)).unwrap())
        .unwrap()
        .loss_db
    };
    let (l145, l435, l1296) = (loss(145.0), loss(435.0), loss(1296.0));
    assert!(l145 < l435 && l435 < l1296, "{l145} {l435} {l1296}");
  }
}

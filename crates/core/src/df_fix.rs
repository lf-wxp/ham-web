//! 测向交汇定位：多条方位线的最小二乘交点与 95% 误差椭圆。
//!
//! 用于无线电测向（ARDF）找发射台、以及排查干扰源时把几次测向结果合成一个位置。
//! 采用**局部切平面近似**：以第一条测向点为原点做东 / 北等距投影，几十公里内误差
//! 可以忽略；跨几百公里的测向本来就不该用平面交汇。
//!
//! 每条方位线的权重取自「横向位置误差」= 该站到目标的距离 × 角度误差，所以远处一次
//! 粗略测向不会把近处一次精确测向淹掉。距离只能取到估计点，因此先等权解一轮求初值，
//! 再按初值迭代加权。

/// 地球平均半径（米）。
const EARTH_R: f64 = 6_371_008.8;

/// 2 自由度、95% 置信区间的卡方分位数，用来把协方差放大成误差椭圆。
const CHI2_95_2DOF: f64 = 5.991_464_547;

/// 一次测向。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DfBearing {
  pub lat: f64,
  pub lon: f64,
  /// 指向目标的方位角：正北 0°，顺时针增大（可超出 0–360，内部取模）。
  pub azimuth_deg: f64,
  /// 这次测向的 1σ 角度误差（度），必须大于 0。
  pub sigma_deg: f64,
}

/// 交汇结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DfFix {
  pub lat: f64,
  pub lon: f64,
  /// 95% 误差椭圆的长半轴（米）。
  pub semi_major_m: f64,
  /// 短半轴（米）。
  pub semi_minor_m: f64,
  /// 长轴的方位角（正北 0°，顺时针）。
  pub major_bearing_deg: f64,
  /// 残差（度，1σ 口径）；只有两条方位线时恒为 0。
  pub rms_deg: f64,
  /// 参与解算的测向条数。
  pub used: usize,
  /// 交点落在测向点**身后**的方位线条数。
  ///
  /// 方位线按整条直线求交，所以方位角抄反（差 180°）也能得到同一个交点——但那个点在
  /// 测向点身后，通常说明方位角记错了或站位弄反了。非 0 时页面应提示核对。
  pub behind: usize,
}

/// 无法解算的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DfError {
  /// 少于两条方位线。
  TooFewBearings,
  /// 坐标越界、角度误差非正或非有限值。
  InvalidInput,
  /// 方位线几乎平行（含方向相反），没有唯一交点。
  Parallel,
}

fn valid(b: &DfBearing) -> bool {
  b.lat.is_finite()
    && b.lon.is_finite()
    && b.azimuth_deg.is_finite()
    && b.sigma_deg.is_finite()
    && b.sigma_deg > 0.0
    && b.sigma_deg <= 45.0
    && (-90.0..=90.0).contains(&b.lat)
    && (-180.0..=180.0).contains(&b.lon)
}

/// 加权迭代轮数：1 轮等权求初值 + 2 轮按「到估计点的距离」加权。
const WEIGHT_ROUNDS: usize = 2;

/// 参考距离下限（米）：目标与测向点重合时横向误差没有物理意义，避免权重发散。
const MIN_RANGE_M: f64 = 50.0;

/// 累加正规方程的 5 个系数（`sxx` / `sxy` / `syy` / `sxb` / `syb`）。
///
/// `weights == None` 时等权，只用于求初值；否则按 `centre` 到各站的距离加权。
fn accumulate(
  bearings: &[DfBearing],
  pts: &[(f64, f64)],
  weights: Option<(f64, f64)>,
) -> (f64, f64, f64, f64, f64) {
  let (mut sxx, mut sxy, mut syy, mut sxb, mut syb) = (0.0, 0.0, 0.0, 0.0, 0.0);
  for (b, &(x, y)) in bearings.iter().zip(pts) {
    let az = b.azimuth_deg.to_radians();
    let (sin, cos) = az.sin_cos();
    let (ax, ay) = (cos, -sin);
    let bias = ax * x + ay * y;
    let w = match weights {
      None => 1.0,
      Some((cx, cy)) => {
        let dist = ((cx - x).powi(2) + (cy - y).powi(2))
          .sqrt()
          .max(MIN_RANGE_M);
        let sigma_m = dist * b.sigma_deg.to_radians();
        1.0 / (sigma_m * sigma_m)
      }
    };
    sxx += w * ax * ax;
    sxy += w * ax * ay;
    syy += w * ay * ay;
    sxb += w * ax * bias;
    syb += w * ay * bias;
  }
  (sxx, sxy, syy, sxb, syb)
}

/// 解算全部方位线的交点。
///
/// 返回的 `semi_major_m` 是 95% 椭圆长半轴：它很大（几十公里）说明几何太差
/// （方位线夹角太小），页面应提示「再换一个位置测一次」，而不是把这个点当真。
pub fn solve(bearings: &[DfBearing]) -> Result<DfFix, DfError> {
  if bearings.len() < 2 {
    return Err(DfError::TooFewBearings);
  }
  if !bearings.iter().all(valid) {
    return Err(DfError::InvalidInput);
  }

  let lat0 = bearings[0].lat.to_radians();
  let lon0 = bearings[0].lon;

  // 平面坐标（米）：x 东、y 北，原点取第一条测向点。
  let pts: Vec<(f64, f64)> = bearings
    .iter()
    .map(|b| {
      let lat = b.lat.to_radians();
      // 经度差先折到 [-180°, 180°]：两个测向点分处日期变更线两侧（如 179.9 与 -179.9，
      // 地理上仅隔约 0.2°）时直接相减得到 -359.8°，会把相邻两点投成相隔约 4 万公里的远点，
      // 交汇退化成「只信一条方位线」或误判 `Parallel`。
      let dlon = (b.lon - lon0 + 180.0).rem_euclid(360.0) - 180.0;
      let x = dlon.to_radians() * ((lat0 + lat) / 2.0).cos() * EARTH_R;
      let y = (lat - lat0) * EARTH_R;
      (x, y)
    })
    .collect();

  // 正规方程 Σ w·a·aᵀ · p = Σ w·a·b，其中 a 与直线方向正交，b = a·P。
  //
  // 权重 = 1 / 横向误差²，横向误差 =「该站到目标的距离」× 角度误差。距离只能取到
  // 估计点，所以先等权解一轮拿初值，再按初值算距离加权迭代。不能拿「到原点的距离」：
  // 原点就是第一条测向点，它自己的距离恒为 0，被兜成 1 m 后权重比别的站大 7 个数量级，
  // 交汇就退化成「只信第一条方位线」，误差椭球也跟着失真。
  let mut weights: Option<(f64, f64)> = None;
  let mut normal = (0.0, 0.0, 0.0, 0.0, 0.0);
  let mut det = 0.0;
  let (mut px, mut py) = (0.0, 0.0);
  for round in 0..=WEIGHT_ROUNDS {
    normal = accumulate(bearings, &pts, weights);
    let (sxx, sxy, syy, sxb, syb) = normal;
    det = sxx * syy - sxy * sxy;
    let trace = sxx + syy;
    if !det.is_finite() || trace <= 0.0 || det.abs() < 1e-8 * trace * trace {
      return Err(DfError::Parallel);
    }
    px = (syy * sxb - sxy * syb) / det;
    py = (sxx * syb - sxy * sxb) / det;
    if round < WEIGHT_ROUNDS {
      weights = Some((px, py));
    }
  }

  let (sxx, sxy, syy, _, _) = normal;
  // 协方差 = 正规矩阵的逆（观测噪声已由权重归一到单位方差）。
  let cxx = syy / det;
  let cyy = sxx / det;
  let cxy = -sxy / det;
  let half_trace = (cxx + cyy) / 2.0;
  let disc = (((cxx - cyy) / 2.0).powi(2) + cxy * cxy).sqrt();
  let semi_major = (CHI2_95_2DOF * (half_trace + disc).max(0.0)).sqrt();
  let semi_minor = (CHI2_95_2DOF * (half_trace - disc).max(0.0)).sqrt();

  // 主特征向量相对 x 轴的角度 → 相对正北的方位角。
  let theta_east = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
  let major_bearing_deg = (90.0 - theta_east.to_degrees()).rem_euclid(360.0);

  // 残差同样按「到估计点的距离」折算成角度，才与 `sigma_deg` 同一口径。
  let mut sum_sq = 0.0;
  for (b, &(x, y)) in bearings.iter().zip(&pts) {
    let az = b.azimuth_deg.to_radians();
    let (sin, cos) = az.sin_cos();
    let residual = cos * (px - x) - sin * (py - y);
    let dist = ((px - x).powi(2) + (py - y).powi(2))
      .sqrt()
      .max(MIN_RANGE_M);
    sum_sq += (residual / dist).powi(2);
  }
  let dof = bearings.len() - 2;
  let rms_deg = if dof > 0 {
    (sum_sq / dof as f64).sqrt().to_degrees()
  } else {
    0.0
  };

  // 交点沿该方位线方向的投影为负，说明目标在测向点身后。距离小于 `MIN_RANGE_M` 时符号
  // 没有物理意义（测向点本来就在目标上），不计。
  let behind = bearings
    .iter()
    .zip(&pts)
    .filter(|&(b, &(x, y))| {
      let (sin, cos) = b.azimuth_deg.to_radians().sin_cos();
      (px - x) * sin + (py - y) * cos < -MIN_RANGE_M
    })
    .count();

  // 近共线但刚好越过 `det` 门限时交点能跑到几千公里外：平面近似与 ±90° 纬度都已失效，
  // 这种解比「解不出来」更危险，直接按几何太差处理。
  let lat_fix = (lat0 + py / EARTH_R).to_degrees();
  if !lat_fix.is_finite() || lat_fix.abs() >= 90.0 {
    return Err(DfError::Parallel);
  }
  let mid_lat = (lat0 + lat_fix.to_radians()) / 2.0;
  if mid_lat.cos().abs() < 1e-6 {
    return Err(DfError::Parallel);
  }
  let lon_raw = lon0 + (px / (EARTH_R * mid_lat.cos())).to_degrees();
  if !lon_raw.is_finite() {
    return Err(DfError::Parallel);
  }
  let lon_fix = ((lon_raw + 180.0).rem_euclid(360.0) - 180.0).clamp(-180.0, 180.0);

  Ok(DfFix {
    lat: lat_fix,
    lon: lon_fix,
    semi_major_m: semi_major,
    semi_minor_m: semi_minor,
    major_bearing_deg,
    rms_deg,
    used: bearings.len(),
    behind,
  })
}

/// 误差留空时采用的角度误差（度）：新手不会给自己填误差，但不能因此算不出来。
pub const DEFAULT_SIGMA_DEG: f64 = 2.0;

/// 解析面板里的一行输入（四个文本框的原文）。
///
/// - 纬度 / 经度 / 方位角**缺任何一项**都算「这一行还没填完」，返回 `Ok(None)`：用户先填
///   纬度再填经度、或正在补第三条测向时，已经算出来的交汇点不该被「输入无效」顶掉；
/// - 三项都填了但有一项不是数字，才是 [`DfError::InvalidInput`]；
/// - 误差留空按 [`DEFAULT_SIGMA_DEG`]，填了就必须是数字。
///
/// 范围（纬度 ±90、σ ∈ (0, 45] 等）不在这里查，交给 [`solve`] 统一判定。
pub fn parse_row(
  lat: &str,
  lon: &str,
  azimuth: &str,
  sigma: &str,
) -> Result<Option<DfBearing>, DfError> {
  let (lat, lon, azimuth, sigma) = (lat.trim(), lon.trim(), azimuth.trim(), sigma.trim());
  if [lat, lon, azimuth].iter().any(|v| v.is_empty()) {
    return Ok(None);
  }
  let number = |text: &str| text.parse::<f64>().map_err(|_| DfError::InvalidInput);
  let sigma_deg = if sigma.is_empty() {
    DEFAULT_SIGMA_DEG
  } else {
    number(sigma)?
  };
  Ok(Some(DfBearing {
    lat: number(lat)?,
    lon: number(lon)?,
    azimuth_deg: number(azimuth)?,
    sigma_deg,
  }))
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 从给定点按方位角与距离走一步（球面近似，够本模块的测试用）。
  fn offset(lat: f64, lon: f64, bearing_deg: f64, dist_m: f64) -> (f64, f64) {
    let d = dist_m / EARTH_R;
    let br = bearing_deg.to_radians();
    let lat_rad = lat.to_radians();
    let lat2 = lat_rad + d * br.cos();
    let lon2 = lon.to_radians() + d * br.sin() / lat_rad.cos();
    (lat2.to_degrees(), lon2.to_degrees())
  }

  /// 目标周围 5 km、间隔 120° 的三次测向，噪声为零。
  fn three_bearings(target_lat: f64, target_lon: f64) -> Vec<DfBearing> {
    [0.0, 120.0, 240.0]
      .iter()
      .map(|&brg| {
        let (lat, lon) = offset(target_lat, target_lon, brg, 5000.0);
        DfBearing {
          lat,
          lon,
          azimuth_deg: (brg + 180.0).rem_euclid(360.0),
          sigma_deg: 1.0,
        }
      })
      .collect()
  }

  fn metres_between(a_lat: f64, a_lon: f64, b_lat: f64, b_lon: f64) -> f64 {
    let dy = (a_lat - b_lat).to_radians() * EARTH_R;
    let dx = (a_lon - b_lon).to_radians() * a_lat.to_radians().cos() * EARTH_R;
    (dx * dx + dy * dy).sqrt()
  }

  #[test]
  fn noiseless_bearings_meet_at_target() {
    let (lat, lon) = (39.9, 116.4);
    let fix = solve(&three_bearings(lat, lon)).expect("应能解算");
    assert!(metres_between(fix.lat, fix.lon, lat, lon) < 10.0);
    assert_eq!(fix.used, 3);
    assert!(fix.semi_major_m > 0.0);
    assert!(fix.rms_deg < 4.0);
  }

  #[test]
  fn two_bearings_are_enough_but_leave_a_long_axis() {
    let (lat, lon) = (39.9, 116.4);
    let mut list = three_bearings(lat, lon);
    list.truncate(2);
    let fix = solve(&list).expect("两条方位线也应能解算");
    assert_eq!(fix.used, 2);
    assert_eq!(fix.rms_deg, 0.0);
    assert!(fix.semi_major_m >= fix.semi_minor_m);
  }

  #[test]
  fn more_error_grows_the_ellipse() {
    let (lat, lon) = (39.9, 116.4);
    let tight = solve(&three_bearings(lat, lon)).expect("解算");
    let mut loose = three_bearings(lat, lon);
    for b in &mut loose {
      b.sigma_deg = 5.0;
    }
    let wide = solve(&loose).expect("解算");
    assert!(wide.semi_major_m > tight.semi_major_m * 2.0);
    assert!(wide.semi_major_m > wide.semi_minor_m);
  }

  #[test]
  fn parallel_lines_have_no_solution() {
    let list = [
      DfBearing {
        lat: 39.9,
        lon: 116.4,
        azimuth_deg: 45.0,
        sigma_deg: 1.0,
      },
      DfBearing {
        lat: 39.91,
        lon: 116.4,
        azimuth_deg: 45.0,
        sigma_deg: 1.0,
      },
      DfBearing {
        lat: 39.9,
        lon: 116.41,
        azimuth_deg: 225.0,
        sigma_deg: 1.0,
      },
    ];
    assert_eq!(solve(&list), Err(DfError::Parallel));
  }

  #[test]
  fn rejects_bad_input() {
    let one = [DfBearing {
      lat: 39.9,
      lon: 116.4,
      azimuth_deg: 0.0,
      sigma_deg: 1.0,
    }];
    assert_eq!(solve(&one), Err(DfError::TooFewBearings));

    let mut list = three_bearings(39.9, 116.4);
    list[0].sigma_deg = 0.0;
    assert_eq!(solve(&list), Err(DfError::InvalidInput));

    let mut list = three_bearings(39.9, 116.4);
    list[1].lat = 100.0;
    assert_eq!(solve(&list), Err(DfError::InvalidInput));
  }

  #[test]
  fn azimuth_wraps_below_zero() {
    let (lat, lon) = (39.9, 116.4);
    let normal = solve(&three_bearings(lat, lon)).expect("解算");
    let mut wrapped = three_bearings(lat, lon);
    for b in &mut wrapped {
      b.azimuth_deg -= 360.0;
    }
    let other = solve(&wrapped).expect("解算");
    assert!(metres_between(normal.lat, normal.lon, other.lat, other.lon) < 1.0);
  }

  #[test]
  fn a_rough_first_station_does_not_dominate_the_fix() {
    // 第一条测向点就是平面坐标原点。按「到原点的距离」加权时它的距离恒为 0，权重比
    // 其它站大 7 个数量级，交汇结果几乎等于它自己那条（这里故意偏了 10°）的方位线。
    let (lat, lon) = (39.9, 116.4);
    let mut list = three_bearings(lat, lon);
    list[0].sigma_deg = 30.0;
    list[0].azimuth_deg += 10.0;
    for b in &mut list[1..] {
      b.sigma_deg = 0.5;
    }
    let fix = solve(&list).expect("解算");
    let miss = metres_between(fix.lat, fix.lon, lat, lon);
    assert!(miss < 100.0, "第一条的粗测向把交点拉走了：{miss} m");
    assert!(fix.rms_deg < 45.0, "rms 口径失真：{}°", fix.rms_deg);
  }

  #[test]
  fn output_stays_inside_the_coordinate_ranges() {
    // 近共线的几何要么报 Parallel，要么给出合法坐标：不许把 ±90 之外的纬度丢给页面。
    let list = [
      DfBearing {
        lat: 39.9,
        lon: 116.4,
        azimuth_deg: 10.0,
        sigma_deg: 1.0,
      },
      DfBearing {
        lat: 39.95,
        lon: 116.4,
        azimuth_deg: 12.0,
        sigma_deg: 30.0,
      },
    ];
    match solve(&list) {
      Ok(fix) => {
        assert!((-90.0..=90.0).contains(&fix.lat), "lat {}", fix.lat);
        assert!((-180.0..=180.0).contains(&fix.lon), "lon {}", fix.lon);
      }
      Err(err) => assert_eq!(err, DfError::Parallel),
    }
  }

  #[test]
  fn a_reversed_bearing_is_flagged_as_pointing_away_from_the_fix() {
    // 把一条方位角抄反 180°：直线没变，交点还是目标，但目标落在这个测向点身后。
    let (lat, lon) = (39.9, 116.4);
    let ok = solve(&three_bearings(lat, lon)).expect("解算");
    assert_eq!(ok.behind, 0, "三条都指向目标时不该有身后的方位线");

    let mut list = three_bearings(lat, lon);
    list[1].azimuth_deg = (list[1].azimuth_deg + 180.0).rem_euclid(360.0);
    let flipped = solve(&list).expect("直线没变，仍应解得出");
    assert_eq!(flipped.behind, 1);
    assert!(metres_between(flipped.lat, flipped.lon, lat, lon) < 10.0);
  }

  #[test]
  fn a_station_standing_on_the_target_is_not_counted_as_behind() {
    // 距离小于 `MIN_RANGE_M` 时前后没有物理意义，不能凭浮点误差计数。
    let (lat, lon) = (39.9, 116.4);
    let mut list = three_bearings(lat, lon);
    list.push(DfBearing {
      lat,
      lon,
      azimuth_deg: 77.0,
      sigma_deg: 2.0,
    });
    assert_eq!(solve(&list).expect("解算").behind, 0);
  }

  #[test]
  fn an_unfinished_row_is_skipped_not_rejected() {
    // 缺任何一项都是「还没填完」，不是错误；σ 默认已填 "2" 也不改变这一点。
    assert_eq!(parse_row("", "", "", "2"), Ok(None));
    assert_eq!(parse_row("39.9", "", "45", "2"), Ok(None));
    assert_eq!(parse_row("39.9", "116.4", "", ""), Ok(None));
    assert_eq!(parse_row("  ", "116.4", "45", "2"), Ok(None));
  }

  #[test]
  fn a_complete_row_parses_with_the_documented_defaults() {
    let row = parse_row(" -33.9 ", "151.2", "270", "")
      .expect("合法")
      .expect("完整");
    assert_eq!(
      row,
      DfBearing {
        lat: -33.9,
        lon: 151.2,
        azimuth_deg: 270.0,
        sigma_deg: DEFAULT_SIGMA_DEG,
      }
    );
    let explicit = parse_row("39.9", "116.4", "45", "0.5")
      .expect("合法")
      .expect("完整");
    assert_eq!(explicit.sigma_deg, 0.5);
  }

  #[test]
  fn a_filled_but_non_numeric_row_is_an_error() {
    // 三项都填了才查数字：半截的 `-` 只要有一项还空着就不报错，填满了才报。
    assert_eq!(
      parse_row("-", "116.4", "45", "2"),
      Err(DfError::InvalidInput)
    );
    assert_eq!(
      parse_row("39.9", "abc", "45", "2"),
      Err(DfError::InvalidInput)
    );
    assert_eq!(
      parse_row("39.9", "116.4", "45", "x"),
      Err(DfError::InvalidInput)
    );
    // 范围不归解析管：越界交给 `solve` 判（返回 InvalidInput）。
    let out_of_range = parse_row("95", "116.4", "45", "2")
      .expect("能解析")
      .expect("完整");
    assert_eq!(
      solve(&[out_of_range, out_of_range]),
      Err(DfError::InvalidInput)
    );
  }

  #[test]
  fn ellipse_major_axis_points_where_the_geometry_is_weak() {
    // 两条南北向（方位 0° / 0°）的平行方案解不出来，退而求其次：
    // 两条夹角大的方位线，长轴应大致落在夹角平分线的垂直方向 —— 这里只钉
    // 「长轴方位角是合法角度」，具体方向由协方差特征向量决定，不给主观期望。
    let fix = solve(&three_bearings(39.9, 116.4)).expect("解算");
    assert!((0.0..360.0).contains(&fix.major_bearing_deg));
    assert!(fix.semi_minor_m <= fix.semi_major_m);
  }

  #[test]
  fn bearings_across_the_antimeridian_stay_close() {
    // 两个测向点分处日期变更线两侧（179.9 与 -179.9，地理上仅隔约 0.2°）：经度差不先
    // 折到 [-180°, 180°] 就会得到 -359.8°，把 B 投到约 4 万公里外，交点要么误判 Parallel、
    // 要么给出远离目标的错误坐标。
    let list = [
      DfBearing {
        lat: 5.0,
        lon: 179.9,
        azimuth_deg: 45.0,
        sigma_deg: 1.0,
      },
      DfBearing {
        lat: 5.0,
        lon: -179.9,
        azimuth_deg: 315.0,
        sigma_deg: 1.0,
      },
    ];
    let fix = solve(&list).expect("跨日期变更线应能解算");
    assert!(
      fix.lon.abs() > 179.0,
      "交点经度应落在日期变更线附近：{}",
      fix.lon
    );
    assert!(fix.lat > 5.0, "交点应在两测向点连线的北侧：{}", fix.lat);
  }
}

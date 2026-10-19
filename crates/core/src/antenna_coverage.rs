//! 天线方向增益 × DXCC 实体：回答「这副天线朝那个方向能给出多少增益」。
//!
//! **只算天线本身**（含求解器当前的地面模型与效率），不计传播、噪声、功率与对方天线。
//! 因此这里的输出不能叫「覆盖」也不能叫「可达性」——口径见 `docs/antenna-coverage.md`。
//!
//! 两种口径：
//! - [`GainMode::BestElevation`]：在该方位的全部仰角上取三维方向图的最大值（默认）；
//! - [`GainMode::Horizontal`]：在水平面切面（固定仰角 = 最大增益仰角）上取值。
//!
//! # 角度口径
//!
//! 实体的「方位」是地理方位（正北 0°、顺时针），方向图的 φ 是 NEC 口径（+X 起算、
//! 逆时针，见 `crate::nec`）。两者差一次 `φ = 90° − bearing`，换算只在
//! [`direction_gains`] 里做一次 —— 直接透传会让整张方向表偏 90° 且左右镜像。
//! 这等于假设模型里 **+X 指东、+Y 指北**：天线在水平面上怎么摆是用户的自由，
//! 表里的方位只有在这个约定下才有意义，页面说明里也写了这一句。

use crate::dxcc::entities;
use crate::grid::{distance_bearing, lat_lon_from_grid};
use crate::nec::NecResult;

/// 取值口径。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GainMode {
  /// 该方位上所有仰角里的最大值。
  BestElevation,
  /// 水平面切面（仰角固定为最大增益仰角）。
  Horizontal,
}

/// 一个实体的方向增益。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityGain {
  pub dxcc: u16,
  pub name: &'static str,
  pub name_en: &'static str,
  /// 本台指向实体中心的方位角（0–360，正北为 0）。
  pub azimuth_deg: f64,
  /// 大圆距离（km）。
  pub distance_km: f64,
  /// 该方向的增益（dBi）。
  pub gain_dbi: f64,
  /// 相对该天线最大增益的损失（dB，>= 0）。
  pub loss_db: f64,
}

/// 只取方向表需要的三份数据，避免把整个 [`NecResult`] 传进纯计算层。
#[derive(Debug, Clone, Copy)]
pub struct PatternSlices<'a> {
  /// 水平面切面：`(方位角, 增益)`，360° 全周。
  pub azimuth: &'a [(f64, f64)],
  /// 三维方向图，`theta` 行主序，长度 `(n_theta + 1) × n_phi`。
  pub pattern3d: &'a [f64],
  /// `(n_theta, n_phi)`。
  pub shape: (usize, usize),
  /// 该天线在所有方向上的最大增益。
  pub gain_max_dbi: f64,
}

impl<'a> From<&'a NecResult> for PatternSlices<'a> {
  fn from(result: &'a NecResult) -> Self {
    Self {
      azimuth: &result.azimuth,
      pattern3d: &result.pattern3d,
      shape: result.pattern3d_shape,
      gain_max_dbi: result.gain_max_dbi,
    }
  }
}

/// 三维方向图的双线性插值（`θ` 不环绕，`φ` 环绕）。
fn sample_grid(slices: &PatternSlices, theta_deg: f64, phi_deg: f64) -> f64 {
  let (n_theta, n_phi) = slices.shape;
  if n_theta == 0 || n_phi == 0 || slices.pattern3d.is_empty() {
    return slices.gain_max_dbi;
  }
  // 插值最多访问到第 `n_theta` 行，网格短于 `(n_theta + 1) × n_phi` 就说明 shape 与数据
  // 不符。此时任何降级值都会被插值放大成 NaN 或负无穷（`0 × -inf = NaN`），直接按
  // 「取不到」处理：退回峰值，不污染整张表与排序。
  let expected = (n_theta + 1).checked_mul(n_phi);
  if expected.is_none_or(|n| slices.pattern3d.len() < n)
    || !theta_deg.is_finite()
    || !phi_deg.is_finite()
  {
    return slices.gain_max_dbi;
  }
  let ft = (theta_deg.clamp(0.0, 180.0) / 180.0) * n_theta as f64;
  let i0 = ft.floor() as usize;
  let i1 = (i0 + 1).min(n_theta);
  let wt = ft - i0 as f64;

  let fp = (phi_deg.rem_euclid(360.0) / 360.0) * n_phi as f64;
  let j0 = (fp.floor() as usize) % n_phi;
  let j1 = (j0 + 1) % n_phi;
  let wp = fp - fp.floor();

  // 上面的长度校验保证这里恒命中；`unwrap_or` 只是「不 panic」的兜底，且不能用
  // `NEG_INFINITY`（`0 × -inf = NaN`，会把整行增益与排序都毁掉）。
  let at = |i: usize, j: usize| {
    slices
      .pattern3d
      .get(i * n_phi + j)
      .copied()
      .unwrap_or(slices.gain_max_dbi)
  };
  let near = at(i0, j0) * (1.0 - wp) + at(i0, j1) * wp;
  let far = at(i1, j0) * (1.0 - wp) + at(i1, j1) * wp;
  near * (1.0 - wt) + far * wt
}

/// 该方位上所有仰角里的最大增益。
fn best_gain(slices: &PatternSlices, phi_deg: f64) -> f64 {
  let (n_theta, _) = slices.shape;
  if n_theta == 0 {
    return slices.gain_max_dbi;
  }
  (0..=n_theta)
    .map(|i| {
      let theta = 180.0 * i as f64 / n_theta as f64;
      sample_grid(slices, theta, phi_deg)
    })
    .fold(f64::NEG_INFINITY, f64::max)
}

/// 水平面切面在给定方位上的线性插值。
fn horizontal_gain_at(azimuth: &[(f64, f64)], phi_deg: f64) -> Option<f64> {
  let n = azimuth.len();
  if n == 0 {
    return None;
  }
  if n == 1 {
    return Some(azimuth[0].1);
  }
  // 切面按等步长生成（0..=360，步长 360/(n-1)），不做通用查找。
  let step = 360.0 / (n - 1) as f64;
  let pos = phi_deg.rem_euclid(360.0) / step;
  let i = (pos.floor() as usize).min(n - 1);
  let frac = pos - i as f64;
  let a = azimuth[i].1;
  let b = azimuth[(i + 1).min(n - 1)].1;
  Some(a + (b - a) * frac)
}

/// 切面是否是 `horizontal_gain_at` 期望的形状：φ 从 0 起、到 360 止、等步长。
///
/// 满足它时「第 i 个点 = i × step」才成立；`0..359` 这类缺终点的写法会让每个方向
/// 静默偏 1°，在表上完全看不出来。
fn azimuth_slice_is_complete(azimuth: &[(f64, f64)]) -> bool {
  let n = azimuth.len();
  if n < 2 {
    return false;
  }
  if azimuth[0].0.abs() > 1e-6 || (azimuth[n - 1].0 - 360.0).abs() > 1e-6 {
    return false;
  }
  let step = (azimuth[n - 1].0 - azimuth[0].0) / (n - 1) as f64;
  step > 0.0
    && azimuth
      .windows(2)
      .all(|w| (w[1].0 - w[0].0 - step).abs() < 1e-6)
}

fn gain_at(slices: &PatternSlices, phi_deg: f64, mode: GainMode) -> f64 {
  match mode {
    GainMode::BestElevation => best_gain(slices, phi_deg),
    GainMode::Horizontal => {
      horizontal_gain_at(slices.azimuth, phi_deg).unwrap_or(slices.gain_max_dbi)
    }
  }
}

/// 地理方位（正北 0°、顺时针）→ 方向图的 φ（+X 起算、逆时针）。
///
/// 假设模型里 +X 指东、+Y 指北：方位 β 的方向向量是 `(sin β, cos β)`，而方向图取
/// `(cos φ, sin φ)`，故 `φ = 90° − β`。少了这一步，整张表偏 90° 且左右镜像。
#[must_use]
pub fn pattern_phi(azimuth_deg: f64) -> f64 {
  (90.0 - azimuth_deg).rem_euclid(360.0)
}

/// 生成方向表：每个 DXCC 实体一行，按「相对峰值的损失」升序排列。
///
/// 本台网格非法时返回 `None`（没有原点就算不出方位）。
#[must_use]
pub fn direction_gains(
  slices: &PatternSlices,
  home_grid: &str,
  mode: GainMode,
) -> Option<Vec<EntityGain>> {
  let (home_lat, home_lon) = lat_lon_from_grid(home_grid)?;
  // 基准增益不有限说明求解器给了脏方向图：整表没有可比参照，宁可不出表，也不要给一列
  // `NaN` 增益配 `0` 损失 —— 那会被读成「每个方向都完美对准」。
  let base = slices.gain_max_dbi;
  if !base.is_finite() {
    return None;
  }
  // 水平模式依赖「φ = 0..=360 等步长」的切面约定（见 `horizontal_gain_at`）。形状不符时
  // 静默错一格在表上完全看不出来，所以直接不出表。
  if matches!(mode, GainMode::Horizontal) && !azimuth_slice_is_complete(slices.azimuth) {
    return None;
  }
  let mut rows: Vec<EntityGain> = entities()
    .iter()
    .map(|entity| {
      let (distance_km, azimuth_deg) = distance_bearing(home_lat, home_lon, entity.lat, entity.lon);
      let gain = gain_at(slices, pattern_phi(azimuth_deg), mode);
      // 切面按 2° 采样、基准峰值来自更细的扫描，个别方向可能比基准还高。夹到基准，免得
      // 出现「增益大于最大值、损失却是 0」的自相矛盾行；非有限值同样兜回基准。
      let gain = if gain.is_finite() {
        gain.min(base)
      } else {
        base
      };
      EntityGain {
        dxcc: entity.dxcc,
        name: entity.name,
        name_en: entity.name_en,
        azimuth_deg,
        distance_km,
        gain_dbi: gain,
        loss_db: base - gain,
      }
    })
    .collect();
  rows.sort_by(|a, b| {
    a.loss_db
      .total_cmp(&b.loss_db)
      .then_with(|| a.distance_km.total_cmp(&b.distance_km))
  });
  Some(rows)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn slices<'a>(
    azimuth: &'a [(f64, f64)],
    pattern3d: &'a [f64],
    shape: (usize, usize),
    gain_max_dbi: f64,
  ) -> PatternSlices<'a> {
    PatternSlices {
      azimuth,
      pattern3d,
      shape,
      gain_max_dbi,
    }
  }

  /// θ 三行（0 / 90 / 180）× φ 四列（0 / 90 / 180 / 270）。
  fn uniform_rows(value: f64) -> Vec<f64> {
    vec![value; 3 * 4]
  }

  #[test]
  fn grid_interpolates_between_phi_points() {
    let mut grid = vec![0.0; 3 * 4];
    for i in 0..3 {
      for j in 0..4 {
        grid[i * 4 + j] = j as f64 * 10.0;
      }
    }
    let s = slices(&[], &grid, (1, 4), 30.0);
    assert!((sample_grid(&s, 90.0, 45.0) - 5.0).abs() < 1e-9);
    // φ=350° 落在 270°（30）与 0°（0）之间，环绕后取到中间值。
    let wrapped = sample_grid(&s, 90.0, 350.0);
    assert!(wrapped > 0.0 && wrapped < 30.0, "{wrapped}");
  }

  #[test]
  fn best_elevation_picks_the_peak_row() {
    let mut grid = vec![-300.0; 3 * 4];
    for j in 0..4 {
      grid[4 + j] = 12.0;
    }
    let s = slices(&[], &grid, (1, 4), 12.0);
    assert!((best_gain(&s, 0.0) - 12.0).abs() < 1e-9);
    assert!((best_gain(&s, 180.0) - 12.0).abs() < 1e-9);
  }

  #[test]
  fn front_to_back_shows_up_as_loss() {
    let mut grid = vec![-300.0; 3 * 4];
    grid[4] = 10.0; // θ=90、φ=0
    grid[6] = -10.0; // θ=90、φ=180
    let s = slices(&[], &grid, (1, 4), 10.0);
    let front = best_gain(&s, 0.0);
    let back = best_gain(&s, 180.0);
    assert!((front - 10.0).abs() < 1e-9);
    assert!((back + 10.0).abs() < 1e-9);
    assert!(((10.0 - back) - 20.0).abs() < 1e-9);
  }

  #[test]
  fn horizontal_slice_interpolates() {
    let az: Vec<(f64, f64)> = (0..=360)
      .map(|d| (f64::from(d), f64::from(d) * 0.1))
      .collect();
    assert!((horizontal_gain_at(&az, 45.0).unwrap() - 4.5).abs() < 1e-9);
    assert!((horizontal_gain_at(&az, 350.0).unwrap() - 35.0).abs() < 1e-9);
    assert!(horizontal_gain_at(&[], 0.0).is_none());
  }

  #[test]
  fn a_slice_above_the_reference_peak_is_clamped() {
    // 水平切面按 2° 采样、基准峰值来自更细的扫描：个别方向可能「比最大值还高」。
    // 不夹会把这类行显示成「增益 20 dBi、损失 0 dB」。
    let az: Vec<(f64, f64)> = (0..=360).map(|d| (f64::from(d), 20.0)).collect();
    let grid = uniform_rows(10.0);
    let s = slices(&az, &grid, (1, 4), 15.0);
    let rows = direction_gains(&s, "OM89", GainMode::Horizontal).expect("本台网格有效");
    for row in &rows {
      assert!(row.gain_dbi <= 15.0 + 1e-9, "{} 的增益超过基准", row.name);
      assert!(row.loss_db >= 0.0, "{} 的损失为负", row.name);
      assert!((row.loss_db - (15.0 - row.gain_dbi)).abs() < 1e-9);
    }
  }

  #[test]
  fn incomplete_horizontal_slice_or_nan_peak_yields_no_table() {
    // `0..359`（缺终点、360 点）不是 `horizontal_gain_at` 的约定形状：宁可不给表，
    // 也不要让每个实体静默偏 1°。
    let short: Vec<(f64, f64)> = (0..360).map(|d| (f64::from(d), 0.0)).collect();
    let grid = uniform_rows(1.0);
    let s = slices(&short, &grid, (1, 4), 10.0);
    assert!(direction_gains(&s, "OM89", GainMode::Horizontal).is_none());

    // 基准增益 NaN：整表没有参照，不能给出一列「NaN 增益 + 0 损失」。
    let full: Vec<(f64, f64)> = (0..=360).map(|d| (f64::from(d), 0.0)).collect();
    let bad = slices(&full, &grid, (1, 4), f64::NAN);
    assert!(direction_gains(&bad, "OM89", GainMode::BestElevation).is_none());
  }

  #[test]
  fn geographic_bearings_are_converted_to_pattern_phi() {
    // 东（90°）→ φ 0°、北（0°）→ φ 90°、西 → 180°、南 → 270°。
    for (bearing, phi) in [(90.0, 0.0), (0.0, 90.0), (270.0, 180.0), (180.0, 270.0)] {
      let got = pattern_phi(bearing);
      assert!((got - phi).abs() < 1e-9, "{bearing}° → {got}°，应为 {phi}°");
    }
    // 负方位先取模，符号不影响结果。
    assert!((pattern_phi(-90.0) - 180.0).abs() < 1e-9);
  }

  #[test]
  fn a_broadside_east_antenna_peaks_toward_the_east() {
    // 水平面切面 `gain = 10·cos(φ)`：峰值只在 φ=0（即正东）。
    let az: Vec<(f64, f64)> = (0..=360)
      .map(|d| {
        #[allow(clippy::cast_precision_loss)]
        let phi = f64::from(d).to_radians();
        (f64::from(d), 10.0 * phi.cos())
      })
      .collect();
    let grid = uniform_rows(0.0);
    let s = slices(&az, &grid, (1, 4), 10.0);
    let rows = direction_gains(&s, "OM89", GainMode::Horizontal).expect("本台网格有效");

    // 挑出「指向最接近正东」的实体：它必须拿到整张表的最大增益。
    let east_gap = |r: &&EntityGain| {
      let d = (r.azimuth_deg - 90.0).abs();
      d.min(360.0 - d)
    };
    let east_most = rows
      .iter()
      .min_by(|a, b| east_gap(a).total_cmp(&east_gap(b)));
    let east_most = east_most.expect("DXCC 表非空");
    let max_gain = rows
      .iter()
      .map(|r| r.gain_dbi)
      .fold(f64::NEG_INFINITY, f64::max);
    assert!(
      (east_most.gain_dbi - max_gain).abs() < 0.6,
      "正东的实体没拿到峰值：{}° 处 {:.2} dBi，整表峰值 {:.2} dBi",
      east_most.azimuth_deg,
      east_most.gain_dbi,
      max_gain
    );
  }

  #[test]
  fn horizontal_mode_uses_the_slice_not_the_3d_grid() {
    let az: Vec<(f64, f64)> = (0..=360)
      .map(|d| (f64::from(d), f64::from(d) * 0.1))
      .collect();
    let grid = uniform_rows(-300.0);
    let s = slices(&az, &grid, (1, 4), 36.0);
    assert!((gain_at(&s, 90.0, GainMode::Horizontal) - 9.0).abs() < 1e-9);
    // 同一份数据里 3D 全是 -300，两种口径必须给出不同结果。
    assert!(gain_at(&s, 90.0, GainMode::BestElevation) < -299.0);
  }

  #[test]
  fn rows_cover_every_entity_and_are_sorted() {
    let grid = uniform_rows(0.0);
    let s = slices(&[], &grid, (1, 4), 0.0);
    let rows = direction_gains(&s, "OM89", GainMode::BestElevation).expect("本台网格有效");
    assert_eq!(rows.len(), entities().len());
    assert!(rows.windows(2).all(|w| w[0].loss_db <= w[1].loss_db));
    for row in &rows {
      assert!(row.loss_db.abs() < 1e-9, "全向图不该有损失");
      assert!(
        (0.0..360.0).contains(&row.azimuth_deg),
        "{}",
        row.azimuth_deg
      );
      assert!(row.distance_km > 0.0);
    }
  }

  #[test]
  fn invalid_home_grid_has_no_rows() {
    let grid = uniform_rows(0.0);
    let s = slices(&[], &grid, (1, 4), 0.0);
    assert!(direction_gains(&s, "", GainMode::BestElevation).is_none());
    assert!(direction_gains(&s, "ZZ99", GainMode::BestElevation).is_none());
  }

  #[test]
  fn empty_slices_fall_back_to_gain_max() {
    let s = slices(&[], &[], (0, 0), 7.0);
    assert!((best_gain(&s, 0.0) - 7.0).abs() < 1e-9);
    assert!((gain_at(&s, 123.0, GainMode::Horizontal) - 7.0).abs() < 1e-9);
  }
}

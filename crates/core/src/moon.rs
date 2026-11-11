//! 月球位置、月相与 EME 用的站心量：方位 / 仰角、月距、距离变化率（回波多普勒）、受照比例、双站共同可见窗口。
//!
//! 纯函数、零依赖，不碰浏览器，供 `/eme` 的计划器使用。
//!
//! # 算法与出处
//!
//! 月球轨道根数与摄动项取自 Paul Schlyter 的低精度公式（stjarnhimlen.se，《How to compute planetary positions》）：
//! 6 个开普勒根数 + 经度 12 项、纬度 5 项、距离 2 项周期摄动。太阳取同一页的 2 体根数。
//! 站心量由「地心赤道矢量 → 绕自转轴转 GMST → 减去 WGS84 椭球上的观测者 ECEF 矢量 → 东北天」得到，
//! **视差是严格算进去的**（月球视差约 57′，不能当作地心方向用）。
//! 距离变化率用 ±30 s 的中心差分。
//!
//! # 精度（对拍 JPL Horizons，见单测）
//!
//! 对 Horizons（DE441）取回的 48 个站心参照值（三个站点 × 2025–2031 年的 14 个时刻，加北京 1992–2056 年的抽样），
//! 实测最大误差：方位 / 仰角合成的角度 < 0.06°，站心距离 < 0.15%，距离变化率 < 0.002 km/s
//! （折成回波多普勒：144 MHz 约 2 Hz、1296 MHz 约 16 Hz），受照比例 < 0.8 个百分点。单测的容限留了约 2 倍余量。
//!
//! 这对 50–432 MHz 的阵列天线（波束宽度远大于 1°）绰绰有余；**10 GHz 级抛物面（波束约 0.5°）不够**，
//! 那里要用完整的月球理论。
//!
//! # 没有做的
//!
//! - 时间标度：UTC 直接当作力学时用（差约 69 s，月球 69 s 只走 0.01°），不做 ΔT；
//! - 大气折射：输出是真仰角（Horizons 的 `a-app` 在无折射时同口径）；低仰角处折射可达 0.5°；
//! - 月面噪声温度、EME 路径损耗的绝对值、天平动衰落、法拉第旋转：没有可对拍的来源，**不收**。
//!   只给相对量：[`path_loss_delta_db`]（按 1/d⁴ 的雷达方程，相对平均月距）。

use crate::sat_doppler::C_KM_S;

const D2R: f64 = std::f64::consts::PI / 180.0;
const R2D: f64 = 180.0 / std::f64::consts::PI;
/// 地球赤道半径（km，WGS84），也是月球根数里「地球半径」的单位。
const RE_KM: f64 = 6378.137;
const AU_KM: f64 = 149_597_870.7;
/// WGS84 扁率。
const FLATTENING: f64 = 1.0 / 298.257_223_563;
/// 平均地月距离（km）。
pub const MEAN_DISTANCE_KM: f64 = 384_400.0;

/// 观测者（大地坐标）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observer {
  /// 北纬为正（°）。
  pub lat_deg: f64,
  /// 东经为正（°）。
  pub lon_deg: f64,
  /// 海拔（km）。
  pub alt_km: f64,
}

impl Observer {
  /// 海拔取 0（网格中心本来就没有海拔信息）。
  #[must_use]
  pub const fn at(lat_deg: f64, lon_deg: f64) -> Self {
    Self {
      lat_deg,
      lon_deg,
      alt_km: 0.0,
    }
  }
}

/// 月球相对观测者的位置。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Topo {
  /// 方位（°，正北 0、顺时针）。
  pub az_deg: f64,
  /// 仰角（°，真仰角，不含大气折射）。
  pub el_deg: f64,
  /// 站心距离（km）。
  pub range_km: f64,
}

/// 月相。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Phase {
  /// 受照比例（0–100 %）。
  pub illumination_pct: f64,
  /// 渐盈（朔 → 望）为真，渐亏（望 → 朔）为假。
  pub waxing: bool,
}

/// 一个可用窗口：一端或两端同时在最低仰角之上。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
  pub start_unix: f64,
  pub end_unix: f64,
  /// 窗口内「各端仰角中较低者」的最大值（°）：双站时它是共同可见的最好程度。
  pub peak_el_deg: f64,
  /// 起点是被查询起点截断的：查询开始时月球已在最低仰角之上，真正的开始更早。
  pub clipped_start: bool,
  /// 终点是被查询终点截断的：查询结束时月球仍在最低仰角之上，真正的结束更晚。
  pub clipped_end: bool,
}

/// Unix 秒 → 儒略日（UT）。
#[must_use]
pub fn julian_day(unix_s: f64) -> f64 {
  unix_s / 86_400.0 + 2_440_587.5
}

fn norm360(x: f64) -> f64 {
  x.rem_euclid(360.0)
}

/// 开普勒方程（度输入、弧度输出）：牛顿迭代。
fn kepler(m_deg: f64, e: f64) -> f64 {
  let m = m_deg * D2R;
  let mut big_e = m + e * m.sin() * (1.0 + e * m.cos());
  for _ in 0..6 {
    big_e -= (big_e - e * big_e.sin() - m) / (1.0 - e * big_e.cos());
  }
  big_e
}

/// 太阳地心黄经（°）与距离（km）；根数取自 Schlyter。
fn sun_ecliptic(d: f64) -> (f64, f64) {
  let ws = 282.9404 + 4.70935e-5 * d;
  let es = 0.016709 - 1.151e-9 * d;
  let ms = norm360(356.0470 + 0.9856002585 * d);
  let big_e = kepler(ms, es);
  let xv = big_e.cos() - es;
  let yv = (1.0 - es * es).sqrt() * big_e.sin();
  let v = yv.atan2(xv) * R2D;
  (norm360(v + ws), xv.hypot(yv) * AU_KM)
}

/// 月球地心黄经（°）、黄纬（°）与距离（km），含 Schlyter 的周期摄动项。
fn moon_ecliptic(d: f64) -> (f64, f64, f64) {
  let n = norm360(125.1228 - 0.0529538083 * d);
  let inc = 5.1454_f64;
  let w = norm360(318.0634 + 0.1643573223 * d);
  let a = 60.2666;
  let e = 0.054900;
  let mm = norm360(115.3654 + 13.0649929509 * d);
  let ws = 282.9404 + 4.70935e-5 * d;
  let ms = norm360(356.0470 + 0.9856002585 * d);
  let ls = norm360(ws + ms);
  let lm = norm360(n + w + mm);
  let big_d = norm360(lm - ls);
  let f = norm360(lm - n);

  let big_e = kepler(mm, e);
  let xv = a * (big_e.cos() - e);
  let yv = a * (1.0 - e * e).sqrt() * big_e.sin();
  let v = yv.atan2(xv) * R2D;
  let r = xv.hypot(yv);
  let vw = (v + w) * D2R;
  let (nr, ir) = (n * D2R, inc * D2R);
  let xh = r * (nr.cos() * vw.cos() - nr.sin() * vw.sin() * ir.cos());
  let yh = r * (nr.sin() * vw.cos() + nr.cos() * vw.sin() * ir.cos());
  let zh = r * vw.sin() * ir.sin();
  let mut lon = yh.atan2(xh) * R2D;
  let mut lat = zh.atan2(xh.hypot(yh)) * R2D;

  let s = |x: f64| (x * D2R).sin();
  let c = |x: f64| (x * D2R).cos();
  lon += -1.274 * s(mm - 2.0 * big_d) + 0.658 * s(2.0 * big_d)
    - 0.186 * s(ms)
    - 0.059 * s(2.0 * mm - 2.0 * big_d)
    - 0.057 * s(mm - 2.0 * big_d + ms)
    + 0.053 * s(mm + 2.0 * big_d)
    + 0.046 * s(2.0 * big_d - ms)
    + 0.041 * s(mm - ms)
    - 0.035 * s(big_d)
    - 0.031 * s(mm + ms)
    - 0.015 * s(2.0 * f - 2.0 * big_d)
    + 0.011 * s(mm - 4.0 * big_d);
  lat +=
    -0.173 * s(f - 2.0 * big_d) - 0.055 * s(mm - f - 2.0 * big_d) - 0.046 * s(mm + f - 2.0 * big_d)
      + 0.033 * s(f + 2.0 * big_d)
      + 0.017 * s(2.0 * mm + f);
  let dist_er = r - 0.58 * c(mm - 2.0 * big_d) - 0.46 * c(2.0 * big_d);
  (norm360(lon), lat, dist_er * RE_KM)
}

/// 黄道坐标 → 赤道直角坐标（km）。
fn ecliptic_to_equatorial(lon_deg: f64, lat_deg: f64, dist_km: f64, d: f64) -> [f64; 3] {
  let (sl, cl) = (lon_deg * D2R).sin_cos();
  let (sb, cb) = (lat_deg * D2R).sin_cos();
  let (xe, ye, ze) = (dist_km * cl * cb, dist_km * sl * cb, dist_km * sb);
  let (se, ce) = ((23.4393 - 3.563e-7 * d) * D2R).sin_cos();
  [xe, ye * ce - ze * se, ye * se + ze * ce]
}

fn gmst_rad(jd: f64) -> f64 {
  norm360(280.460_618_37 + 360.985_647_366_29 * (jd - 2_451_545.0)) * D2R
}

/// 观测者在地固系下的矢量（km，WGS84 椭球）。
fn observer_ecef(obs: Observer) -> [f64; 3] {
  let e2 = FLATTENING * (2.0 - FLATTENING);
  let (sp, cp) = (obs.lat_deg * D2R).sin_cos();
  let (sl, cl) = (obs.lon_deg * D2R).sin_cos();
  let n = RE_KM / (1.0 - e2 * sp * sp).sqrt();
  [
    (n + obs.alt_km) * cp * cl,
    (n + obs.alt_km) * cp * sl,
    (n * (1.0 - e2) + obs.alt_km) * sp,
  ]
}

/// 月球在 `unix_s` 时刻相对 `obs` 的方位 / 仰角 / 站心距离。
#[must_use]
pub fn topocentric(unix_s: f64, obs: Observer) -> Topo {
  let jd = julian_day(unix_s);
  let d = jd - 2_451_543.5;
  let (lon, lat, dist) = moon_ecliptic(d);
  let [mx, my, mz] = ecliptic_to_equatorial(lon, lat, dist, d);
  let (st, ct) = gmst_rad(jd).sin_cos();
  // 赤道惯性系 → 地固系：绕自转轴转 GMST。
  let (ex, ey, ez) = (mx * ct + my * st, -mx * st + my * ct, mz);
  let [ox, oy, oz] = observer_ecef(obs);
  let (tx, ty, tz) = (ex - ox, ey - oy, ez - oz);

  let (sp, cp) = (obs.lat_deg * D2R).sin_cos();
  let (sl, cl) = (obs.lon_deg * D2R).sin_cos();
  let east = -sl * tx + cl * ty;
  let north = -sp * cl * tx - sp * sl * ty + cp * tz;
  let up = cp * cl * tx + cp * sl * ty + sp * tz;
  let range = (tx * tx + ty * ty + tz * tz).sqrt();
  Topo {
    az_deg: norm360(east.atan2(north) * R2D),
    el_deg: (up / range).clamp(-1.0, 1.0).asin() * R2D,
    range_km: range,
  }
}

/// 站心距离变化率（km/s，正 = 远离）：±30 s 的中心差分。
#[must_use]
pub fn range_rate_km_s(unix_s: f64, obs: Observer) -> f64 {
  const HALF_STEP_S: f64 = 30.0;
  let ahead = topocentric(unix_s + HALF_STEP_S, obs).range_km;
  let behind = topocentric(unix_s - HALF_STEP_S, obs).range_km;
  (ahead - behind) / (2.0 * HALF_STEP_S)
}

/// 自己的回波多普勒（Hz）：发射频率 `freq_hz` 经月球反射回来，相对发射频率的偏移。
///
/// 月球远离（距离变化率为正）时回波频率变低，所以带负号；系数 2 是往返。
/// 用的是一阶式 `−2·f·ṙ/c`，月球径向速度不到 0.4 km/s，二阶项（约 10⁻⁶ 的量级再乘 10⁻⁶）可忽略。
#[must_use]
pub fn echo_doppler_hz(freq_hz: f64, range_rate_km_s: f64) -> f64 {
  -2.0 * freq_hz * range_rate_km_s / C_KM_S
}

/// 路径损耗相对平均月距（[`MEAN_DISTANCE_KM`]）的变化（dB，正 = 比平均更差）。
///
/// 地月地是雷达方程，回波功率 ∝ 1/d⁴，所以是 `40·log10(d / d平均)`：近地点约 −1.3 dB、远地点约 +1.0 dB。
/// 这是纯几何缩放，不含任何经验常数；绝对路径损耗（约 250 dB 量级）要用月面反射率，本模块不收。
#[must_use]
pub fn path_loss_delta_db(range_km: f64) -> Option<f64> {
  if range_km.is_finite() && range_km > 0.0 {
    Some(40.0 * (range_km / MEAN_DISTANCE_KM).log10())
  } else {
    None
  }
}

/// 月相：由日月地心夹角算受照比例，由黄经差判渐盈 / 渐亏。
#[must_use]
pub fn phase(unix_s: f64) -> Phase {
  let d = julian_day(unix_s) - 2_451_543.5;
  let (m_lon, m_lat, m_dist) = moon_ecliptic(d);
  let (s_lon, s_dist) = sun_ecliptic(d);
  // 日月在黄道坐标系里的夹角（太阳黄纬取 0）。
  let cos_psi = (m_lat * D2R).cos() * ((m_lon - s_lon) * D2R).cos();
  let psi = cos_psi.clamp(-1.0, 1.0).acos();
  // 相位角 i：太阳—月球—地球的夹角（Meeus 第 48 章）。
  let i = (s_dist * psi.sin()).atan2(m_dist - s_dist * psi.cos());
  Phase {
    illumination_pct: (1.0 + i.cos()) / 2.0 * 100.0,
    waxing: norm360(m_lon - s_lon) < 180.0,
  }
}

/// 最长查询时长与最小采样步长：防止一次请求扫出几十万次三角函数。
pub const MAX_HOURS: f64 = 72.0;
pub const MIN_STEP_S: f64 = 10.0;

/// 从 `start_unix` 起 `hours` 小时内，一端（`b` 为 `None`）或两端同时在 `min_el_deg` 之上的窗口。
///
/// 按 `step_s` 采样，窗口边界的分辨率就是一个步长。参数不合法（时长 / 步长非正、非有限）返回空。
/// 起点已经在窗口里时，窗口从 `start_unix` 算起。
#[must_use]
pub fn windows(
  a: Observer,
  b: Option<Observer>,
  start_unix: f64,
  hours: f64,
  step_s: f64,
  min_el_deg: f64,
) -> Vec<Window> {
  let valid = [start_unix, hours, step_s, min_el_deg]
    .iter()
    .all(|x| x.is_finite());
  if !valid || hours <= 0.0 || step_s < MIN_STEP_S {
    return Vec::new();
  }
  let hours = hours.min(MAX_HOURS);
  let steps = (hours * 3600.0 / step_s).floor() as usize;
  let mut out: Vec<Window> = Vec::new();
  let mut open: Option<Window> = None;
  for k in 0..=steps {
    let t = start_unix + k as f64 * step_s;
    let el_a = topocentric(t, a).el_deg;
    let el = b.map_or(el_a, |b| el_a.min(topocentric(t, b).el_deg));
    if el >= min_el_deg {
      match open.as_mut() {
        Some(w) => {
          w.end_unix = t;
          w.peak_el_deg = w.peak_el_deg.max(el);
        }
        None => {
          open = Some(Window {
            start_unix: t,
            end_unix: t,
            peak_el_deg: el,
            clipped_start: k == 0,
            clipped_end: false,
          });
        }
      }
    } else if let Some(w) = open.take() {
      out.push(w);
    }
  }
  // 扫描结束时窗口还开着：终点只是查询的边界，不是月球真的落下去了。
  if let Some(mut w) = open {
    w.clipped_end = true;
    out.push(w);
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 一行 Horizons 参照值，字段顺序见 [`HORIZONS`]。
  type Reference = (f64, f64, f64, f64, f64, f64, f64, f64, f64);

  /// JPL Horizons（DE441，`COMMAND=301`、`CENTER=coord@399`、观测者表、无大气折射）取回的月球站心参照值，2026-11-10：
  /// `(东经°, 北纬°, 海拔 km, UT 儒略日, 方位°, 仰角°, 站心距离 km, 距离变化率 km/s, 受照 %)`。
  /// 前 42 行是三个站点 × 14 个时刻（2025–2031），后 6 行是北京 1992–2056 年的抽样。
  const HORIZONS: &[Reference] = &[
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2461000.00000,
      266.6734,
      -37.9124,
      410526.3,
      0.27027,
      0.244,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2461137.74000,
      282.3128,
      -59.3442,
      410396.7,
      0.17332,
      77.046,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2461275.48000,
      319.6160,
      -75.2455,
      410603.2,
      0.04808,
      75.936,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2461412.22000,
      68.8994,
      -64.5080,
      412217.6,
      -0.13377,
      1.202,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2461549.96000,
      87.8331,
      -43.2760,
      408380.3,
      -0.23811,
      85.034,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2461687.70000,
      98.9325,
      -21.3035,
      406709.0,
      -0.31428,
      66.484,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2461824.44000,
      122.0926,
      7.3343,
      404624.0,
      -0.27989,
      5.312,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2461962.18000,
      142.0013,
      28.6878,
      399253.6,
      -0.16710,
      91.878,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2462099.92000,
      171.8119,
      44.0416,
      398702.2,
      -0.01660,
      53.881,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2462236.66000,
      222.7713,
      36.0684,
      399912.4,
      0.21119,
      12.638,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2462374.40000,
      253.1977,
      21.3854,
      397559.9,
      0.33650,
      96.822,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2462511.14000,
      279.5645,
      -6.1782,
      396579.4,
      0.38690,
      31.000,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2462648.88000,
      303.7293,
      -22.1546,
      402956.4,
      0.29929,
      20.833,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2462786.62000,
      333.1757,
      -33.3072,
      400118.1,
      0.16273,
      99.548,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2461000.00000,
      206.7586,
      -26.7107,
      409484.6,
      0.14436,
      0.162,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2461137.74000,
      179.9524,
      -29.2280,
      408018.5,
      -0.00023,
      76.891,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2461275.48000,
      156.9229,
      -24.9960,
      407110.2,
      -0.14000,
      76.295,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2461412.22000,
      127.8908,
      -6.1583,
      407120.9,
      -0.28648,
      1.091,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2461549.96000,
      111.2955,
      11.3798,
      402741.8,
      -0.32498,
      84.860,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2461687.70000,
      94.2912,
      29.6579,
      401251.6,
      -0.31549,
      66.667,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2461824.44000,
      63.8697,
      60.7098,
      399924.4,
      -0.14845,
      5.324,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2461962.18000,
      355.8654,
      67.9021,
      396446.6,
      0.03691,
      91.874,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2462099.92000,
      310.4851,
      50.0403,
      398258.1,
      0.20978,
      53.868,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2462236.66000,
      279.4833,
      19.6237,
      401520.1,
      0.37705,
      12.630,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2462374.40000,
      267.7179,
      -8.1373,
      400784.7,
      0.40595,
      96.651,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2462511.14000,
      244.4901,
      -39.2966,
      399952.7,
      0.31639,
      31.489,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2462648.88000,
      216.4053,
      -61.4502,
      406179.9,
      0.14515,
      20.373,
    ),
    // 悉尼
    (
      151.2093,
      -33.8688,
      0.02,
      2462786.62000,
      149.5523,
      -67.2745,
      402515.7,
      -0.03517,
      99.419,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2461000.00000,
      199.2202,
      -6.8119,
      407337.2,
      0.05190,
      0.296,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2461137.74000,
      222.3559,
      -14.0978,
      406420.4,
      0.10507,
      76.853,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2461275.48000,
      243.1659,
      -20.8698,
      406658.6,
      0.12098,
      75.783,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2461412.22000,
      278.4022,
      -32.9517,
      409899.6,
      0.13341,
      1.345,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2461549.96000,
      307.0834,
      -38.7698,
      407998.8,
      0.10883,
      85.525,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2461687.70000,
      337.8978,
      -39.4634,
      408464.1,
      0.04827,
      65.825,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2461824.44000,
      24.7596,
      -37.6843,
      409363.3,
      -0.04172,
      5.508,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2461962.18000,
      54.7212,
      -25.5934,
      405077.7,
      -0.09647,
      91.958,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2462099.92000,
      79.2664,
      -10.2570,
      404257.8,
      -0.13459,
      54.011,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2462236.66000,
      115.3801,
      4.1022,
      403196.9,
      -0.12241,
      12.312,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2462374.40000,
      142.7908,
      17.7836,
      397933.0,
      -0.06462,
      96.665,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2462511.14000,
      182.9951,
      23.3842,
      393361.2,
      0.05348,
      31.412,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2462648.88000,
      214.3977,
      25.8094,
      397767.5,
      0.11170,
      20.617,
    ),
    // 特罗姆瑟
    (
      18.95,
      69.65,
      0.01,
      2462786.62000,
      245.5164,
      22.0515,
      394203.8,
      0.16432,
      99.555,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2448724.50000,
      2.5883,
      -37.0859,
      372261.7,
      -0.01896,
      68.049,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2451545.00000,
      348.7652,
      -61.0163,
      407986.2,
      0.06710,
      22.942,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2455349.00000,
      85.5017,
      -33.9969,
      404908.0,
      -0.25937,
      80.143,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2458849.50000,
      67.8274,
      -39.2374,
      407903.1,
      -0.23575,
      30.199,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2464328.50000,
      222.6964,
      39.7173,
      389223.4,
      0.24414,
      54.357,
    ),
    // 北京
    (
      116.4,
      39.9,
      0.05,
      2472000.75000,
      22.0330,
      -26.7052,
      386651.3,
      -0.15801,
      99.824,
    ),
  ];

  /// 球面上两个（方位、仰角）方向的夹角（°）。高仰角时方位差会被放大，所以不分开比。
  fn angular_separation(az1: f64, el1: f64, az2: f64, el2: f64) -> f64 {
    let (e1, e2, da) = (el1 * D2R, el2 * D2R, (az1 - az2) * D2R);
    (e1.sin() * e2.sin() + e1.cos() * e2.cos() * da.cos())
      .clamp(-1.0, 1.0)
      .acos()
      * R2D
  }

  fn unix_of(jd: f64) -> f64 {
    (jd - 2_440_587.5) * 86_400.0
  }

  /// 对拍 JPL Horizons：48 个站心参照值，覆盖三个半球位置、昼夜、近地点 / 远地点附近与 1992–2056 年。
  /// 实测最大误差：角度 0.053°、距离 0.12%、变化率 0.0018 km/s、受照 0.70 个百分点；容限留约 2 倍余量。
  #[test]
  fn matches_jpl_horizons() {
    let (mut worst_ang, mut worst_rng, mut worst_rate, mut worst_ill) =
      (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    for &(lon, lat, alt, jd, az_ref, el_ref, rng_ref, rate_ref, ill_ref) in HORIZONS {
      let t = unix_of(jd);
      let obs = Observer {
        lat_deg: lat,
        lon_deg: lon,
        alt_km: alt,
      };
      let topo = topocentric(t, obs);
      let ang = angular_separation(topo.az_deg, topo.el_deg, az_ref, el_ref);
      let rng = (topo.range_km - rng_ref).abs() / rng_ref * 100.0;
      let rate = (range_rate_km_s(t, obs) - rate_ref).abs();
      let ill = (phase(t).illumination_pct - ill_ref).abs();
      let what = format!("lon={lon} lat={lat} jd={jd}");
      assert!(ang < 0.12, "{what} 角度差 {ang}°");
      assert!(rng < 0.3, "{what} 距离差 {rng}%");
      assert!(rate < 0.004, "{what} 变化率差 {rate} km/s");
      assert!(ill < 1.6, "{what} 受照差 {ill}");
      (worst_ang, worst_rng, worst_rate, worst_ill) = (
        worst_ang.max(ang),
        worst_rng.max(rng),
        worst_rate.max(rate),
        worst_ill.max(ill),
      );
    }
    // 把实测的最坏值也钉住：回归时一眼看出精度是否退化（比容限紧得多）。
    assert!(
      worst_ang < 0.07 && worst_rng < 0.2 && worst_rate < 0.003 && worst_ill < 1.0,
      "最坏值：{worst_ang}° / {worst_rng}% / {worst_rate} km/s / {worst_ill}"
    );
  }

  #[test]
  fn azimuth_is_normalised_and_elevation_bounded() {
    for &(lon, lat, alt, jd, ..) in HORIZONS {
      let topo = topocentric(
        unix_of(jd),
        Observer {
          lat_deg: lat,
          lon_deg: lon,
          alt_km: alt,
        },
      );
      assert!((0.0..360.0).contains(&topo.az_deg), "{}", topo.az_deg);
      assert!((-90.0..=90.0).contains(&topo.el_deg));
      // 站心距离 = 地心距离 ± 地球半径：月球在地平线下的对跖侧时比地心距离多出最多一个地球半径（Horizons
      // 参照里就有 410 526 km 的行），在头顶时少一个。地心距离本身在近地点 356 400 到远地点 406 700 km 之间。
      assert!(
        (349_500.0..414_000.0).contains(&topo.range_km),
        "站心月距 {} 超出近地点 − 地球半径 … 远地点 + 地球半径",
        topo.range_km
      );
    }
  }

  /// 渐盈 / 渐亏的判断要和受照比例的实际变化方向一致（避开朔望附近，那里变化方向对 6 小时窗口太敏感）。
  #[test]
  fn waxing_flag_agrees_with_the_direction_of_illumination() {
    for &(.., jd, _, _, _, _, ill_ref) in HORIZONS {
      if !(3.0..97.0).contains(&ill_ref) {
        continue;
      }
      let t = unix_of(jd);
      let growing = phase(t + 6.0 * 3600.0).illumination_pct > phase(t).illumination_pct;
      assert_eq!(phase(t).waxing, growing, "jd={jd} 受照 {ill_ref}%");
    }
  }

  #[test]
  fn echo_doppler_sign_and_size() {
    // 远离（ṙ > 0）回波变低：144.174 MHz、+0.27 km/s → −259.69 Hz（独立算：2·144.174e6·0.27/299792.458）。
    assert!((echo_doppler_hz(144.174e6, 0.27) + 259.69).abs() < 0.01);
    // 靠近则抬高，大小相同。
    assert!((echo_doppler_hz(144.174e6, -0.27) - 259.69).abs() < 0.01);
    assert_eq!(echo_doppler_hz(1.296e9, 0.0), 0.0);
    // 线性于频率：1296 MHz 是 144 MHz 的 9 倍（1296.0 / 144.0 = 9）。
    let ratio = echo_doppler_hz(1.296e9, 0.3) / echo_doppler_hz(1.44e8, 0.3);
    assert!((ratio - 9.0).abs() < 1e-9);
    // 月球径向速度最大约 0.4 km/s：1296.1 MHz 的回波多普勒不到 ±3.5 kHz。
    assert!(echo_doppler_hz(1.2961e9, 0.4).abs() < 3_500.0);
  }

  #[test]
  fn path_loss_delta_follows_the_inverse_fourth_power() {
    assert!(path_loss_delta_db(MEAN_DISTANCE_KM).unwrap().abs() < 1e-12);
    // 近地点约 356 400 km、远地点约 406 700 km：独立算 40·log10(d/384400)。
    assert!((path_loss_delta_db(356_400.0).unwrap() + 1.3138).abs() < 0.001);
    assert!((path_loss_delta_db(406_700.0).unwrap() - 0.9796).abs() < 0.001);
    // 距离翻倍：40·log10(2) = 12.04 dB。
    assert!((path_loss_delta_db(2.0 * MEAN_DISTANCE_KM).unwrap() - 12.0412).abs() < 0.001);
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
      assert_eq!(path_loss_delta_db(bad), None, "{bad}");
    }
  }

  const BEIJING: Observer = Observer::at(39.9, 116.4);
  const SYDNEY: Observer = Observer::at(-33.87, 151.21);
  /// 2026-11-10 00:00 UTC。
  const T0: f64 = 1_794_268_800.0;

  /// 窗口内每个采样点都满足条件，窗口前后各一个采样点不满足（边界精确到一个步长）。
  fn assert_window_is_tight(
    w: &Window,
    a: Observer,
    b: Option<Observer>,
    start: f64,
    end: f64,
    step: f64,
    min_el: f64,
  ) {
    let ok = |t: f64| {
      let el_a = topocentric(t, a).el_deg;
      b.map_or(el_a, |b| el_a.min(topocentric(t, b).el_deg)) >= min_el
    };
    let mut t = w.start_unix;
    while t <= w.end_unix {
      assert!(ok(t), "窗口内 {t} 不满足");
      t += step;
    }
    if w.start_unix - step >= start {
      assert!(!ok(w.start_unix - step), "窗口前一步仍满足，边界不紧");
    }
    if w.end_unix + step <= end {
      assert!(!ok(w.end_unix + step), "窗口后一步仍满足，边界不紧");
    }
  }

  #[test]
  fn single_station_windows_are_tight_and_ordered() {
    let step = 60.0;
    let ws = windows(BEIJING, None, T0, 48.0, step, 5.0);
    assert!(!ws.is_empty(), "48 小时里月球总会升起一次");
    let end = T0 + 48.0 * 3600.0;
    for w in &ws {
      assert!(w.start_unix <= w.end_unix && w.peak_el_deg >= 5.0);
      assert_window_is_tight(w, BEIJING, None, T0, end, step, 5.0);
    }
    assert!(
      ws.windows(2).all(|p| p[0].end_unix < p[1].start_unix),
      "窗口按时间排序且不相交"
    );
  }

  /// 共同窗口一定落在各端自己的窗口里；两端相同时与单站一致。
  #[test]
  fn mutual_windows_sit_inside_each_stations_own_windows() {
    let step = 60.0;
    let a = windows(BEIJING, None, T0, 48.0, step, 5.0);
    let b = windows(SYDNEY, None, T0, 48.0, step, 5.0);
    let both = windows(BEIJING, Some(SYDNEY), T0, 48.0, step, 5.0);
    let end = T0 + 48.0 * 3600.0;
    for w in &both {
      assert_window_is_tight(w, BEIJING, Some(SYDNEY), T0, end, step, 5.0);
      for own in [&a, &b] {
        assert!(
          own
            .iter()
            .any(|o| o.start_unix <= w.start_unix && w.end_unix <= o.end_unix),
          "共同窗口 {w:?} 不在某一端自己的窗口内"
        );
      }
    }
    assert_eq!(windows(BEIJING, Some(BEIJING), T0, 48.0, step, 5.0), a);
  }

  /// 地球两端的对跖点永远不会同时看到月球高过 5°。
  #[test]
  fn antipodal_stations_never_share_the_moon() {
    let antipode = Observer::at(-BEIJING.lat_deg, BEIJING.lon_deg - 180.0);
    assert!(windows(BEIJING, Some(antipode), T0, 72.0, 120.0, 5.0).is_empty());
  }

  #[test]
  fn a_window_that_is_already_open_starts_at_the_query_time() {
    // 取一个月球已在北京地平线上的时刻（Horizons 参照里仰角 > 10° 的一行）。
    let &(lon, lat, _, jd, ..) = HORIZONS
      .iter()
      .find(|r| r.0 == 116.4 && r.5 > 10.0)
      .unwrap();
    let t = unix_of(jd);
    let ws = windows(Observer::at(lat, lon), None, t, 6.0, 60.0, 0.0);
    assert_eq!(ws.first().map(|w| w.start_unix), Some(t));
    assert!(ws.first().is_some_and(|w| w.clipped_start));
  }

  /// 起点 / 终点被查询边界截断要如实标出来，中间自然开合的窗口两端都不标。
  #[test]
  fn windows_cut_by_the_query_edges_are_flagged() {
    let om89 = Observer::at(39.5, 117.0);
    // 00:28 升起、08:31 落下：从 00:00 查 3 小时，终点被截在 03:00。
    let short = windows(om89, None, T0, 3.0, 60.0, 5.0);
    assert_eq!(short.len(), 1);
    assert!(!short[0].clipped_start && short[0].clipped_end);
    assert_eq!(short[0].end_unix, T0 + 3.0 * 3600.0);
    // 查询范围盖住整个窗口：两端都是月球自己的升落。
    let whole = windows(om89, None, T0, 12.0, 60.0, 5.0);
    assert_eq!(whole.len(), 1);
    assert!(!whole[0].clipped_start && !whole[0].clipped_end);
    // 从窗口中间开始、到窗口结束之前截止：两端都被截。
    let middle = windows(om89, None, T0 + 2.0 * 3600.0, 2.0, 60.0, 5.0);
    assert_eq!(middle.len(), 1);
    assert!(middle[0].clipped_start && middle[0].clipped_end);
  }

  #[test]
  fn invalid_window_arguments_give_nothing() {
    for (hours, step) in [
      (0.0, 60.0),
      (-1.0, 60.0),
      (24.0, 5.0),
      (24.0, 0.0),
      (f64::NAN, 60.0),
      (24.0, f64::NAN),
    ] {
      assert!(
        windows(BEIJING, None, T0, hours, step, 5.0).is_empty(),
        "{hours} {step}"
      );
    }
    assert!(windows(BEIJING, None, f64::NAN, 24.0, 60.0, 5.0).is_empty());
    assert!(windows(BEIJING, None, T0, 24.0, 60.0, f64::NAN).is_empty());
    // 时长被封顶在 MAX_HOURS，不会因为传了一个巨大的数就扫几十万个点。
    let capped = windows(BEIJING, None, T0, 1.0e9, 600.0, 0.0);
    assert!(capped.iter().all(|w| w.end_unix <= T0 + MAX_HOURS * 3600.0));
  }

  #[test]
  fn julian_day_matches_the_unix_epoch_and_j2000() {
    assert!((julian_day(0.0) - 2_440_587.5).abs() < 1e-9);
    // 2000-01-01 12:00 UTC = JD 2451545.0 = Unix 946728000。
    assert!((julian_day(946_728_000.0) - 2_451_545.0).abs() < 1e-9);
  }
}

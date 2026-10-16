//! 卫星轨迹代理：按固定步长给出某颗业余卫星在一段时间内相对地面站的方位、仰角、距离与径向速度，
//! 前端据此逐秒插值出多普勒修正频率（见 `ham_web_core::sat_doppler`）。
//!
//! TLE 复用 `/api/passes` 的 6 小时缓存（Celestrak 业余卫星组）；传播用 SGP4。
//!
//! 径向速度的算法：把卫星的 TEME 位置 / 速度转到随地球转动的 ECEF 系
//! （绕 z 轴转 −GMST，速度再减去 `ω × r`），地面站在 ECEF 里静止，
//! 于是 `ṙ = ρ̂ · v_rel`，`ρ` 为站到星的矢量。**远离为正。**
//! 测试里用另一条独立的路径（把站点转到惯性系再算）对拍，见 `tests`。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::Query;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use ham_web_core::sat_doppler::{DEFAULT_STEP_SECS, MAX_STEP_SECS, MAX_TRACK_SECS, Sample};
use serde::{Deserialize, Serialize};
use sgp4::{Constants, Elements, MinutesSinceEpoch};
use tracing::error;

use crate::passes::{self, SatGroup, Tles, enu_basis, geodetic_to_ecef, gmst_radians};

/// 起始时间允许落在「现在」之前多久（秒）：正在进行的过境要能取到。
const MAX_PAST_SECS: i64 = 3600;
/// 起始时间允许落在「现在」之后多久（秒）：TLE 外推超过两天误差已经不小，没必要给更远的。
const MAX_FUTURE_SECS: i64 = 2 * 86_400;
/// 轨迹结果缓存时长 / 条目上限（与 `/api/passes` 同口径：同一段轨迹短时间内不会变）。
const RESULT_CACHE_TTL: Duration = Duration::from_secs(300);
const MAX_RESULT_ENTRIES: usize = 1_000;

/// 地球自转角速度（rad/s）：取 `gmst_radians` 公式里 GMST 的变化率，两者必须一致，
/// 否则 `ω × r` 与旋转角会各用一套地球转速。
fn earth_rotation_rad_s() -> f64 {
  360.985_647_366_29_f64.to_radians() / 86_400.0
}

/// 查询参数。
#[derive(Deserialize)]
pub struct TrackQuery {
  norad: u64,
  lat: f64,
  lon: f64,
  /// 起始 Unix 秒。
  start: i64,
  /// 覆盖的总秒数。
  secs: u32,
  #[serde(default = "default_step")]
  step: u32,
}

const fn default_step() -> u32 {
  DEFAULT_STEP_SECS
}

/// 返回给前端的 JSON。
#[derive(Serialize)]
struct TrackPayload {
  norad: u64,
  name: String,
  step: u32,
  samples: Vec<Sample>,
  stale: bool,
}

/// 一次观测：方位、仰角（度）、距离（km）、径向速度（km/s，远离为正）。
#[derive(Debug, Clone, Copy, PartialEq)]
struct Look {
  az: f64,
  el: f64,
  range_km: f64,
  range_rate_km_s: f64,
}

/// 由卫星的 TEME 位置（km）与速度（km/s）算相对地面站的观测量。
fn observe(
  station_ecef: [f64; 3],
  basis: ([f64; 3], [f64; 3], [f64; 3]),
  unix_secs: f64,
  r_teme: [f64; 3],
  v_teme: [f64; 3],
) -> Look {
  let theta = gmst_radians(unix_secs / 86_400.0 + 2_440_587.5);
  let (c, s) = (theta.cos(), theta.sin());
  // TEME → ECEF：绕 z 轴转 −θ。
  let r = [
    r_teme[0] * c + r_teme[1] * s,
    -r_teme[0] * s + r_teme[1] * c,
    r_teme[2],
  ];
  let v_rot = [
    v_teme[0] * c + v_teme[1] * s,
    -v_teme[0] * s + v_teme[1] * c,
    v_teme[2],
  ];
  // 相对旋转地球的速度：v_rot − ω × r，ω = (0, 0, W)，ω × r = (−W·y, W·x, 0)。
  let w = earth_rotation_rad_s();
  let v = [v_rot[0] + w * r[1], v_rot[1] - w * r[0], v_rot[2]];

  let d = [
    r[0] - station_ecef[0],
    r[1] - station_ecef[1],
    r[2] - station_ecef[2],
  ];
  let range = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
  let rr = (d[0] * v[0] + d[1] * v[1] + d[2] * v[2]) / range;

  let (east, north, up) = basis;
  let dot = |a: [f64; 3]| d[0] * a[0] + d[1] * a[1] + d[2] * a[2];
  let (e, n, u) = (dot(east), dot(north), dot(up));
  Look {
    az: e.atan2(n).to_degrees().rem_euclid(360.0),
    el: u.atan2((e * e + n * n).sqrt()).to_degrees(),
    range_km: range,
    range_rate_km_s: rr,
  }
}

/// 在 `[start, start + secs]` 内每隔 `step` 秒采一个样；任何一点传播失败（轨道衰减等）就整体失败。
fn sample_track(
  elements: &Elements,
  constants: &Constants,
  lat: f64,
  lon: f64,
  start: i64,
  secs: u32,
  step: u32,
) -> Option<Vec<Sample>> {
  let epoch = elements.datetime.and_utc();
  let epoch_unix = epoch.timestamp() as f64 + f64::from(epoch.timestamp_subsec_nanos()) / 1e9;
  let station = geodetic_to_ecef(lat, lon, 0.0);
  let basis = enu_basis(lat, lon);
  let round = |v: f64, k: f64| (v * k).round() / k;

  (0..=secs)
    .step_by(step as usize)
    .map(|offset| {
      let t = start + i64::from(offset);
      let minutes = (t as f64 - epoch_unix) / 60.0;
      let pred = constants.propagate(MinutesSinceEpoch(minutes)).ok()?;
      let l = observe(station, basis, t as f64, pred.position, pred.velocity);
      // 压缩小数位：0.01° / 1 m / 0.1 m/s（折合 437.8 MHz 约 0.15 Hz），整条轨迹的 JSON 小一半。
      Some(Sample {
        t,
        az: round(l.az, 100.0),
        el: round(l.el, 100.0),
        range_km: round(l.range_km, 1000.0),
        range_rate_km_s: round(l.range_rate_km_s, 10_000.0),
      })
    })
    .collect()
}

fn bad_request(msg: &'static str) -> Response {
  (StatusCode::BAD_REQUEST, msg).into_response()
}

/// 结果缓存 key：经纬度按 0.01°（约 1.1 km）量化。
///
/// 前端传的是观测点（网格换算出来的固定值），重复加载与多标签页会命中同一份；
/// 量化误差对应的多普勒误差在 437.8 MHz 上远小于 1 Hz，不影响修正频率。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TrackKey {
  norad: u64,
  lat: i32,
  lon: i32,
  start: i64,
  secs: u32,
  step: u32,
}

fn track_key(q: &TrackQuery) -> TrackKey {
  TrackKey {
    norad: q.norad,
    lat: (q.lat * 100.0).round() as i32,
    lon: (q.lon * 100.0).round() as i32,
    start: q.start,
    secs: q.secs,
    step: q.step,
  }
}

/// 缓存值：`(写入时刻, (已序列化的 JSON, 是否来自过期 TLE))`。
///
/// 内层包成二元组是为了复用 [`crate::cache::evict_oldest`]（它按 `(Instant, T)` 取值）。
type TrackEntry = (Instant, (String, bool));

/// 轨迹结果缓存。
///
/// 存序列化后的字符串：命中时不再跑一遍 `serde_json`，也省掉一次 `TrackPayload` 构造。
#[derive(Default)]
pub struct Cache {
  results: Mutex<HashMap<TrackKey, TrackEntry>>,
}

/// JSON 响应；`stale` 时带上 `X-Cache: stale`（与其它接口同一口径）。
fn json_response(body: String, stale: bool) -> Response {
  let mut res = ([(CONTENT_TYPE, "application/json")], body).into_response();
  if stale {
    crate::cache::mark_stale(&mut res);
  }
  res
}

/// `GET /api/sat-track` 处理器。
///
/// `tle_cache` 与 `/api/passes` 共用（TLE 一份）；`cache` 是本端点自己的结果缓存。
pub async fn handler(
  Query(q): Query<TrackQuery>,
  tle_cache: Arc<passes::Cache>,
  cache: Arc<Cache>,
) -> Response {
  if !(q.lat.is_finite()
    && q.lon.is_finite()
    && (-90.0..=90.0).contains(&q.lat)
    && (-180.0..=180.0).contains(&q.lon))
  {
    return bad_request("invalid lat/lon");
  }
  if q.secs == 0 || q.secs > MAX_TRACK_SECS {
    return bad_request("invalid secs");
  }
  if q.step == 0 || q.step > MAX_STEP_SECS {
    return bad_request("invalid step");
  }
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or(0);
  if q.start < now - MAX_PAST_SECS || q.start > now + MAX_FUTURE_SECS {
    return bad_request("start out of range");
  }

  let key = track_key(&q);
  let hit = cache.results.lock().ok().and_then(|g| g.get(&key).cloned());
  if let Some((t, (json, stale))) = hit
    && t.elapsed() < RESULT_CACHE_TTL
  {
    return json_response(json, stale);
  }

  let Tles { lines, stale } = match passes::tles(&tle_cache, SatGroup::Amateur).await {
    Ok(t) => t,
    Err(resp) => return resp,
  };
  // 第一行 TLE 的 3–7 列是 NORAD 编号：先按它筛，免得为 90 多颗星都解析一遍。
  let Some((name, l1, l2)) = lines
    .iter()
    .find(|(_, l1, _)| l1.get(2..7).and_then(|s| s.trim().parse::<u64>().ok()) == Some(q.norad))
    .map(|(n, a, b)| (n.clone(), a.clone(), b.clone()))
  else {
    return (StatusCode::NOT_FOUND, "unknown satellite").into_response();
  };

  // SGP4 传播最多 1801 个点（`secs ≤ 1800`、`step ≥ 1`）：CPU 密集，必须放
  // `spawn_blocking`，否则会占住 tokio worker（`TimeoutLayer` 对同步 CPU 抢占不了）。
  // TLE 解析也一并挪进去，避免为一次请求在主线程上构造 `Constants`。
  let (lat, lon, start, secs, step) = (q.lat, q.lon, q.start, q.secs, q.step);
  let computed =
    tokio::task::spawn_blocking(move || -> Result<(String, Vec<Sample>), &'static str> {
      let elements =
        Elements::from_tle(Some(name), l1.as_bytes(), l2.as_bytes()).map_err(|_| "bad TLE")?;
      if now.abs_diff(elements.datetime.and_utc().timestamp()) > 7 * 86_400 {
        return Err("TLE too old");
      }
      let constants = Constants::from_elements(&elements).map_err(|_| "bad TLE")?;
      let samples = sample_track(&elements, &constants, lat, lon, start, secs, step)
        .ok_or("propagation failed")?;
      Ok((elements.object_name.clone().unwrap_or_default(), samples))
    })
    .await;
  let (name, samples) = match computed {
    Ok(Ok(v)) => v,
    Ok(Err(msg)) => return (StatusCode::UNPROCESSABLE_ENTITY, msg).into_response(),
    Err(e) => {
      error!("sat track compute failed: {e:#}");
      return (StatusCode::INTERNAL_SERVER_ERROR, "compute failed").into_response();
    }
  };

  let payload = TrackPayload {
    norad: q.norad,
    name,
    step: q.step,
    samples,
    stale,
  };
  match serde_json::to_string(&payload) {
    Ok(json) => {
      if let Ok(mut g) = cache.results.lock() {
        crate::cache::evict_oldest(&mut *g, MAX_RESULT_ENTRIES);
        g.insert(key, (Instant::now(), (json.clone(), stale)));
      }
      json_response(json, stale)
    }
    Err(e) => {
      error!("serialize sat-track failed: {e:#}");
      (StatusCode::INTERNAL_SERVER_ERROR, "serialize failed").into_response()
    }
  }
}

#[cfg(test)]
mod tests {
  use std::time::Instant;

  use super::*;

  #[test]
  fn sampling_keeps_fractional_tle_epoch() {
    let (e, c) = iss_model();
    let epoch = e.datetime.and_utc();
    let exact = epoch.timestamp_micros() as f64 / 1e6;
    let station = geodetic_to_ecef(BEIJING.0, BEIJING.1, 0.0);
    let basis = enu_basis(BEIJING.0, BEIJING.1);
    for offset in (0..3600).step_by(100) {
      let t = epoch.timestamp() + offset;
      let p = c
        .propagate(MinutesSinceEpoch((t as f64 - exact) / 60.0))
        .unwrap();
      let expected = observe(station, basis, t as f64, p.position, p.velocity);
      let sampled = sample_track(&e, &c, BEIJING.0, BEIJING.1, t, 1, 1).unwrap();
      assert!((sampled[0].range_rate_km_s - expected.range_rate_km_s).abs() < 0.00006);
    }
  }

  /// Celestrak 业余卫星组 2026-10-15 取得的真实 TLE（历元 2026 年第 286.82 天）。
  const ISS: (&str, &str, &str) = (
    "ISS (ZARYA)",
    "1 25544U 98067A   26286.82178372  .00006592  00000+0  12831-3 0  9991",
    "2 25544  51.6325  73.0089 0006710 259.3258 100.6975 15.48855783590140",
  );
  const SO50: (&str, &str, &str) = (
    "SAUDISAT 1C (SO-50)",
    "1 27607U 02058C   26286.89489756  .00000950  00000+0  12934-3 0  9992",
    "2 27607  64.5507 121.6463 0070201 240.5175 118.8917 14.83256710282068",
  );

  fn tle(t: (&str, &str, &str)) -> (String, String, String) {
    (t.0.to_owned(), t.1.to_owned(), t.2.to_owned())
  }

  fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
  }

  fn iss_model() -> (Elements, Constants) {
    let e = Elements::from_tle(Some(ISS.0.to_owned()), ISS.1.as_bytes(), ISS.2.as_bytes()).unwrap();
    let c = Constants::from_elements(&e).unwrap();
    (e, c)
  }

  const BEIJING: (f64, f64) = (39.9, 116.4);

  // 以下三组对拍值由独立的 Python 脚本用「惯性系」方法算出：把站点转到 TEME 系、
  // 取站点惯性速度 ω×r_s，再算相对距离与相对速度 —— 与实现里「把卫星转到 ECEF」的路径不同。
  #[test]
  fn observation_matches_the_inertial_frame_method() {
    let station = geodetic_to_ecef(BEIJING.0, BEIJING.1, 0.0);
    let basis = enu_basis(BEIJING.0, BEIJING.1);
    let (r, v) = ([4200.0, 3800.0, 3900.0], [-3.1, 4.2, 5.0]);
    for (unix, az, el, range, rr) in [
      (
        1_760_029_400.0,
        116.256_252_018,
        17.551_383_175,
        1_302.161_091_711,
        3.228_760_675,
      ),
      (
        1_760_029_460.0,
        116.890_858_496,
        17.982_639_029,
        1_283.901_966_136,
        3.194_681_588,
      ),
      // 6 小时后：卫星在地平线下，仰角为负也要对。
      (
        1_760_051_000.0,
        292.604_073_020,
        -26.728_624_855,
        6_740.910_998_878,
        -3.172_640_091,
      ),
    ] {
      let l = observe(station, basis, unix, r, v);
      assert!(near(l.az, az, 1e-6), "az {} vs {az}", l.az);
      assert!(near(l.el, el, 1e-6), "el {} vs {el}", l.el);
      assert!(
        near(l.range_km, range, 1e-6),
        "range {} vs {range}",
        l.range_km
      );
      assert!(
        near(l.range_rate_km_s, rr, 1e-9),
        "rr {} vs {rr}",
        l.range_rate_km_s
      );
    }
  }

  #[test]
  fn ignoring_the_earth_rotation_term_would_be_caught() {
    // 守护测试：若把 `ω × r` 这一项漏掉，径向速度会差零点几 km/s（折合 437.8 MHz 上百 kHz 量级以内的几百 Hz~kHz），
    // 对拍值的容差（1e-9 km/s）远小于这个量，所以上面的测试会红。这里直接量一下这一项有多大。
    let station = geodetic_to_ecef(BEIJING.0, BEIJING.1, 0.0);
    let basis = enu_basis(BEIJING.0, BEIJING.1);
    let (r, v) = ([4200.0, 3800.0, 3900.0], [-3.1, 4.2, 5.0]);
    let with = observe(station, basis, 1_760_029_400.0, r, v).range_rate_km_s;
    // 漏掉 ω×r 的写法：直接用旋转后的速度。
    let theta = gmst_radians(1_760_029_400.0 / 86_400.0 + 2_440_587.5);
    let (c, s) = (theta.cos(), theta.sin());
    let rv = [v[0] * c + v[1] * s, -v[0] * s + v[1] * c, v[2]];
    let re = [r[0] * c + r[1] * s, -r[0] * s + r[1] * c, r[2]];
    let d = [re[0] - station[0], re[1] - station[1], re[2] - station[2]];
    let rng = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let without = (d[0] * rv[0] + d[1] * rv[1] + d[2] * rv[2]) / rng;
    assert!((with - without).abs() > 0.1, "{with} vs {without}");
  }

  #[test]
  fn range_rate_is_the_derivative_of_range_for_a_real_tle() {
    // 用真实 TLE 传播：径向速度必须等于距离对时间的数值导数（中心差分，步长 0.1 s）。
    // 任何一处坐标系或自转项的错误都会在这里暴露（漏 ω×r 会差 0.3 km/s 以上）。
    //
    // 容差为什么是 2e-4 而不是更紧：实测过，即使完全在 TEME 系里、不经过本文件的任何代码，
    // SGP4 给出的速度与它自己位置的数值导数之间也有约 3e-5 km/s（29 mm/s）的固有差异，
    // 本测试的残差（约 7e-5 km/s，折合 437.8 MHz 约 0.1 Hz）正是由它带来的。
    // 2e-4 km/s 仍比「漏掉自转项」的误差小三个数量级，足以抓住真正的错误。
    let (e, c) = iss_model();
    let epoch = e.datetime.and_utc().timestamp_micros() as f64 / 1e6;
    let station = geodetic_to_ecef(BEIJING.0, BEIJING.1, 0.0);
    let basis = enu_basis(BEIJING.0, BEIJING.1);
    let at = |t: f64| {
      let p = c.propagate(MinutesSinceEpoch((t - epoch) / 60.0)).unwrap();
      observe(station, basis, t, p.position, p.velocity)
    };
    let mut worst = 0.0f64;
    let mut checked = 0;
    let mut t = epoch + 600.0;
    while t < epoch + 6.0 * 3600.0 {
      let (lo, hi, mid) = (at(t - 0.1), at(t + 0.1), at(t));
      let numeric = (hi.range_km - lo.range_km) / 0.2;
      worst = worst.max((numeric - mid.range_rate_km_s).abs());
      checked += 1;
      t += 37.0;
    }
    assert!(checked > 400, "{checked}");
    assert!(worst < 2e-4, "最大偏差 {worst} km/s");
  }

  /// 在 `epoch` 之后 24 小时里找一次最大仰角不低于 `min_max_el` 的过境，返回 `(aos, los)`（仰角 ≥ 0 的区间）。
  fn find_pass(e: &Elements, c: &Constants, min_max_el: f64) -> (f64, f64) {
    let epoch = e.datetime.and_utc().timestamp_micros() as f64 / 1e6;
    let station = geodetic_to_ecef(BEIJING.0, BEIJING.1, 0.0);
    let basis = enu_basis(BEIJING.0, BEIJING.1);
    let el = |t: f64| {
      let p = c.propagate(MinutesSinceEpoch((t - epoch) / 60.0)).unwrap();
      observe(station, basis, t, p.position, p.velocity).el
    };
    let mut t = epoch;
    while t < epoch + 86_400.0 {
      if el(t) >= 0.0 {
        let aos = t;
        let mut peak = f64::MIN;
        while el(t) >= 0.0 {
          peak = peak.max(el(t));
          t += 5.0;
        }
        if peak >= min_max_el {
          return (aos, t);
        }
      }
      t += 20.0;
    }
    panic!("24 小时内没有最大仰角 ≥ {min_max_el}° 的过境");
  }

  #[test]
  fn an_iss_pass_has_physically_sensible_doppler() {
    let (e, c) = iss_model();
    let (aos, los) = find_pass(&e, &c, 30.0);
    let samples = sample_track(
      &e,
      &c,
      BEIJING.0,
      BEIJING.1,
      aos as i64 - 60,
      (los - aos) as u32 + 120,
      5,
    )
    .expect("传播不应失败");
    assert!(samples.len() > 100, "{}", samples.len());
    let step_ok = samples.windows(2).all(|w| w[1].t - w[0].t == 5);
    assert!(step_ok);

    // 过境期间（仰角 ≥ 0）：先靠近后远离，径向速度由负变正，且过零点与最大仰角时刻只差几秒。
    let up: Vec<&Sample> = samples.iter().filter(|s| s.el >= 0.0).collect();
    let (first, last) = (up.first().unwrap(), up.last().unwrap());
    assert!(
      first.range_rate_km_s < -5.0,
      "刚升起应当在快速靠近：{}",
      first.range_rate_km_s
    );
    assert!(
      last.range_rate_km_s > 5.0,
      "即将落下应当在快速远离：{}",
      last.range_rate_km_s
    );
    let peak = up.iter().max_by(|a, b| a.el.total_cmp(&b.el)).unwrap();
    assert!(peak.el >= 30.0);
    let zero = up
      .windows(2)
      .find(|w| w[0].range_rate_km_s < 0.0 && w[1].range_rate_km_s >= 0.0)
      .expect("径向速度应当过零");
    assert!(
      (zero[0].t - peak.t).abs() <= 20,
      "过零 {} 与最大仰角 {} 相差过大",
      zero[0].t,
      peak.t
    );

    // 量级：ISS 地平线附近 |ṙ| 约 6–7.7 km/s，437.8 MHz 上多普勒约 ±10 kHz。
    let max_rr = up
      .iter()
      .map(|s| s.range_rate_km_s.abs())
      .fold(0.0, f64::max);
    assert!((5.5..7.8).contains(&max_rr), "{max_rr}");
    let max_shift_khz = max_rr / 299_792.458 * 437.8e6 / 1000.0;
    assert!((8.0..12.0).contains(&max_shift_khz), "{max_shift_khz} kHz");
    // 距离：天顶附近约 400–500 km，地平线约 2000+ km。
    assert!(up.iter().all(|s| (380.0..2800.0).contains(&s.range_km)));
    let min_range = up.iter().map(|s| s.range_km).fold(f64::MAX, f64::min);
    assert!(min_range < 800.0, "{min_range}");
  }

  #[test]
  fn the_track_round_trips_through_the_client_type() {
    // 服务端序列化、客户端（core）反序列化并通过 Track::new 的校验：两端的字段与约束必须对得上。
    let (e, c) = iss_model();
    let start = e.datetime.and_utc().timestamp() + 1000;
    let samples = sample_track(&e, &c, BEIJING.0, BEIJING.1, start, 600, 5).unwrap();
    assert_eq!(samples.len(), 121);
    let json = serde_json::to_string(&TrackPayload {
      norad: 25_544,
      name: "ISS".into(),
      step: 5,
      samples,
      stale: false,
    })
    .unwrap();
    #[derive(Deserialize)]
    struct Back {
      samples: Vec<Sample>,
    }
    let back: Back = serde_json::from_str(&json).unwrap();
    let track = ham_web_core::sat_doppler::Track::new(back.samples).expect("轨迹应当合法");
    assert_eq!((track.start(), track.end()), (start, start + 600));
    // 小数位被压缩：整条轨迹的 JSON 不该膨胀（121 个采样 < 16 KB）。
    assert!(json.len() < 16_000, "{}", json.len());
  }

  fn tle_at(t: (&str, &str, &str), at: i64) -> passes::TleLine {
    let days = at.div_euclid(86_400);
    let date = ham_web_core::study_plan::format_day(days);
    let year: u32 = date[..4].parse().unwrap();
    let first = ham_web_core::study_plan::day_number(&format!("{year}-01-01")).unwrap();
    let epoch = format!("{:02}{:03}.50000000", year % 100, days - first + 1);
    let (name, mut line, second) = tle(t);
    line.replace_range(18..32, &epoch);
    let checksum: u32 = line.as_bytes()[..68]
      .iter()
      .map(|b| match b {
        b'0'..=b'9' => u32::from(b - b'0'),
        b'-' => 1,
        _ => 0,
      })
      .sum();
    line.replace_range(68..69, &(checksum % 10).to_string());
    (name, line, second)
  }

  fn seeded_cache() -> Arc<passes::Cache> {
    let cache = Arc::new(passes::Cache::default());
    cache.tle.lock().unwrap().insert(
      SatGroup::Amateur,
      (
        Instant::now(),
        vec![tle_at(ISS, now()), tle_at(SO50, now())],
      ),
    );
    cache
  }

  fn query(norad: u64, lat: f64, lon: f64, start: i64, secs: u32, step: u32) -> TrackQuery {
    TrackQuery {
      norad,
      lat,
      lon,
      start,
      secs,
      step,
    }
  }

  fn now() -> i64 {
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_secs() as i64
  }

  async fn read(resp: Response) -> (StatusCode, String) {
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
      .await
      .unwrap();
    (status, String::from_utf8_lossy(&bytes).into_owned())
  }

  async fn run(q: TrackQuery) -> (StatusCode, String) {
    read(handler(Query(q), seeded_cache(), Arc::new(Cache::default())).await).await
  }

  #[tokio::test]
  async fn freshly_cached_old_orbit_is_rejected() {
    let t = now();
    let cache = seeded_cache();
    cache.tle.lock().unwrap().insert(
      SatGroup::Amateur,
      (Instant::now(), vec![tle_at(ISS, t - 8 * 86_400)]),
    );
    let (status, body) = read(
      handler(
        Query(query(25_544, 39.9, 116.4, t, 60, 5)),
        cache,
        Arc::new(Cache::default()),
      )
      .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body, "TLE too old");
  }

  #[test]
  fn the_cache_key_quantises_the_observer_position() {
    let t = 1_760_000_000;
    // 同一观测点附近（< 0.01°）算同一个 key：重复加载应当命中。
    assert_eq!(
      track_key(&query(25_544, 39.901, 116.404, t, 60, 5)),
      track_key(&query(25_544, 39.899, 116.401, t, 60, 5))
    );
    // 差得多的位置 / 参数不同则不能混用。
    assert_ne!(
      track_key(&query(25_544, 39.95, 116.404, t, 60, 5)),
      track_key(&query(25_544, 39.901, 116.404, t, 60, 5))
    );
    assert_ne!(
      track_key(&query(25_544, 39.901, 116.404, t, 60, 5)),
      track_key(&query(25_544, 39.901, 116.404, t, 120, 5))
    );
  }

  #[tokio::test]
  async fn repeated_identical_requests_are_served_from_the_result_cache() {
    let t = now();
    let tles = seeded_cache();
    let cache = Arc::new(Cache::default());
    let q = || query(27_607, 39.9, 116.4, t, 600, 10);
    let (s1, b1) = read(handler(Query(q()), tles.clone(), cache.clone()).await).await;
    assert_eq!(s1, StatusCode::OK, "{b1}");
    assert_eq!(cache.results.lock().unwrap().len(), 1);
    // 清掉 TLE 缓存：命中结果缓存就不该再碰 TLE（碰了就会去回源并可能 503）。
    tles.tle.lock().unwrap().clear();
    let (s2, b2) = read(handler(Query(q()), tles, cache).await).await;
    assert_eq!(s2, StatusCode::OK, "{b2}");
    assert_eq!(b1, b2);
  }

  #[tokio::test]
  async fn the_handler_validates_its_parameters() {
    let t = now();
    for (q, why) in [
      (query(25_544, 91.0, 0.0, t, 60, 5), "纬度越界"),
      (query(25_544, 0.0, 181.0, t, 60, 5), "经度越界"),
      (query(25_544, f64::NAN, 0.0, t, 60, 5), "纬度 NaN"),
      (query(25_544, 0.0, 0.0, t, 0, 5), "secs 为 0"),
      (
        query(25_544, 0.0, 0.0, t, MAX_TRACK_SECS + 1, 5),
        "secs 过大",
      ),
      (query(25_544, 0.0, 0.0, t, 60, 0), "step 为 0"),
      (
        query(25_544, 0.0, 0.0, t, 60, MAX_STEP_SECS + 1),
        "step 过大",
      ),
      (
        query(25_544, 0.0, 0.0, t - MAX_PAST_SECS - 10, 60, 5),
        "起点太早",
      ),
      (
        query(25_544, 0.0, 0.0, t + MAX_FUTURE_SECS + 10, 60, 5),
        "起点太晚",
      ),
    ] {
      let (status, _) = run(q).await;
      assert_eq!(status, StatusCode::BAD_REQUEST, "{why}");
    }
    // 参数都对但卫星不在 TLE 组里：404，而不是 200 空轨迹。
    let (status, body) = run(query(99_999, 39.9, 116.4, t, 60, 5)).await;
    assert_eq!(
      (status, body.as_str()),
      (StatusCode::NOT_FOUND, "unknown satellite")
    );
  }

  #[tokio::test]
  async fn the_handler_returns_a_fixed_step_track_for_the_requested_satellite() {
    let t = now();
    let (status, body) = run(query(27_607, 39.9, 116.4, t, 600, 10)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["norad"], 27_607);
    assert_eq!(v["name"], "SAUDISAT 1C (SO-50)");
    assert_eq!(v["step"], 10);
    let samples = v["samples"].as_array().unwrap();
    assert_eq!(samples.len(), 61); // 0..=600 步长 10
    assert_eq!(samples[0]["t"], t);
    assert_eq!(samples[60]["t"], t + 600);
    // 与同一颗星的 ISS 区分开：SO-50 的轨道倾角 64.6°，轨道高度约 700 km，所以距离不会低于 ~600 km。
    let min_range = samples
      .iter()
      .map(|s| s["range_km"].as_f64().unwrap())
      .fold(f64::MAX, f64::min);
    assert!(min_range > 550.0, "{min_range}");
    // 默认步长：不传 step 时用 DEFAULT_STEP_SECS。
    let q: TrackQuery =
      serde_json::from_str(r#"{"norad":1,"lat":0,"lon":0,"start":0,"secs":10}"#).unwrap();
    assert_eq!(q.step, DEFAULT_STEP_SECS);
  }
}

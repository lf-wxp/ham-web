//! 卫星过境预报代理：拉取 Celestrak 业余卫星 TLE，用 SGP4 计算未来 24 小时过境。
//!
//! TLE 缓存 6 小时；过境计算在 `spawn_blocking` 中执行，避免阻塞事件循环。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use sgp4::{Constants, Elements, MinutesSinceEpoch};
use tracing::{error, warn};

use crate::cache::Inflight;

/// 卫星群组：业余卫星（默认）与气象卫星（NOAA APT）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SatGroup {
  #[default]
  Amateur,
  Weather,
}

impl SatGroup {
  /// 对应 Celestrak GP 组 TLE 地址。
  const fn url(self) -> &'static str {
    match self {
      Self::Amateur => "https://celestrak.org/NORAD/elements/gp.php?GROUP=amateur&FORMAT=tle",
      Self::Weather => "https://celestrak.org/NORAD/elements/gp.php?GROUP=weather&FORMAT=tle",
    }
  }
}

impl std::str::FromStr for SatGroup {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "weather" => Ok(Self::Weather),
      _ => Ok(Self::Amateur),
    }
  }
}
/// TLE 缓存时长。
const TLE_CACHE_TTL: Duration = Duration::from_secs(6 * 3600);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;
/// 采样步长（秒）。
const STEP_SECS: f64 = 30.0;
/// 过境结果缓存时长（结果短时间内不变，缓存可显著降低 CPU 压力）。
const RESULT_CACHE_TTL: Duration = Duration::from_secs(300);
/// 结果缓存条目数上限。
const MAX_RESULT_ENTRIES: usize = 1_000;

/// 一行 TLE 数据：`(卫星名, 第一行, 第二行)`。
pub(crate) type TleLine = (String, String, String);
/// TLE 缓存值：`(缓存时间, TLE 列表)`。
type TleCacheEntry = (Instant, Vec<TleLine>);

/// 取回的一组 TLE，以及它是不是「回源失败时的过期兜底」。
pub(crate) struct Tles {
  pub(crate) lines: Vec<TleLine>,
  /// `true` 表示上游故障、用的是超过 TTL 的旧 TLE（响应要标 `X-Cache: stale`）。
  pub(crate) stale: bool,
}

/// 查询参数。
#[derive(Deserialize)]
pub struct PassQuery {
  lat: f64,
  lon: f64,
  #[serde(default = "default_min_elev")]
  min_elev: f64,
  /// 卫星群组：`amateur`（默认）或 `weather`。
  #[serde(default)]
  group: String,
}

const fn default_min_elev() -> f64 {
  10.0
}

/// 单条过境记录。
#[derive(Serialize, Clone)]
struct Pass {
  name: String,
  norad: u64,
  /// 升交点（开始可见）Unix 秒。
  aos: i64,
  /// 降交点（结束可见）Unix 秒。
  los: i64,
  /// 最大仰角（度）。
  max_elev: f64,
  /// 最大仰角时刻 Unix 秒。
  max_elev_time: i64,
  /// 最大仰角时的方位角（度，从北顺时针）。
  azimuth: f64,
}

/// 返回给前端的 JSON。
#[derive(Serialize)]
struct PassesPayload {
  source: &'static str,
  lat: f64,
  lon: f64,
  count: usize,
  passes: Vec<Pass>,
}

/// 解析 Celestrak 三行式 TLE 文本为 `(名称, 第一行, 第二行)` 列表。
fn parse_tle_group(text: &str) -> Vec<TleLine> {
  let mut out = Vec::new();
  let mut lines = text
    .lines()
    .map(str::trim)
    .filter(|l| !l.is_empty())
    .peekable();
  while let Some(first) = lines.next() {
    if first.starts_with("1 ") {
      if let Some(second) = lines.next()
        && second.starts_with("2 ")
      {
        out.push((String::new(), first.to_owned(), second.to_owned()));
      }
    } else if let (Some(l1), Some(l2)) = (lines.next(), lines.next())
      && l1.starts_with("1 ")
      && l2.starts_with("2 ")
    {
      out.push((first.to_owned(), l1.to_owned(), l2.to_owned()));
    }
  }
  out
}

/// 拉取 Celestrak TLE。
fn fetch_tle(group: SatGroup) -> anyhow::Result<Vec<TleLine>> {
  let mut res = crate::util::http_agent()
    .get(group.url())
    .call()
    .context("fetch Celestrak failed")?;
  let text = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read Celestrak body failed")?;
  Ok(parse_tle_group(&text))
}

/// 格林尼治平恒星时（弧度），IAU 简化近似。
pub(crate) fn gmst_radians(jd: f64) -> f64 {
  let d = jd - 2451545.0;
  (280.46061837 + 360.98564736629 * d)
    .rem_euclid(360.0)
    .to_radians()
}

/// 地理坐标 → ECEF（WGS84，单位 km）。
pub(crate) fn geodetic_to_ecef(lat_deg: f64, lon_deg: f64, alt_km: f64) -> [f64; 3] {
  const A: f64 = 6378.137;
  const F: f64 = 1.0 / 298.257223563;
  let e2 = F * (2.0 - F);
  let lat = lat_deg.to_radians();
  let lon = lon_deg.to_radians();
  let n = A / (1.0 - e2 * lat.sin() * lat.sin()).sqrt();
  [
    (n + alt_km) * lat.cos() * lon.cos(),
    (n + alt_km) * lat.cos() * lon.sin(),
    (n * (1.0 - e2) + alt_km) * lat.sin(),
  ]
}

/// 地面站 ENU 基向量（东、北、上）。
pub(crate) fn enu_basis(lat_deg: f64, lon_deg: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
  let lat = lat_deg.to_radians();
  let lon = lon_deg.to_radians();
  let east = [-lon.sin(), lon.cos(), 0.0];
  let north = [-lat.sin() * lon.cos(), -lat.sin() * lon.sin(), lat.cos()];
  let up = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
  (east, north, up)
}

/// 计算某时刻卫星相对地面站的仰角与方位角（度）。
fn look(
  constants: &Constants,
  epoch_unix: f64,
  station_ecef: [f64; 3],
  basis: ([f64; 3], [f64; 3], [f64; 3]),
  unix_secs: f64,
) -> Option<(f64, f64)> {
  let minutes = (unix_secs - epoch_unix) / 60.0;
  let pred = constants.propagate(MinutesSinceEpoch(minutes)).ok()?;
  let sat_teme = pred.position;

  // TEME → ECEF 近似：绕 z 轴旋转 -GMST。
  let jd = unix_secs / 86400.0 + 2440587.5;
  let theta = gmst_radians(jd);
  let (ct, st) = (theta.cos(), theta.sin());
  let sat_ecef = [
    sat_teme[0] * ct + sat_teme[1] * st,
    -sat_teme[0] * st + sat_teme[1] * ct,
    sat_teme[2],
  ];

  let d = [
    sat_ecef[0] - station_ecef[0],
    sat_ecef[1] - station_ecef[1],
    sat_ecef[2] - station_ecef[2],
  ];
  let (east, north, up) = basis;
  let e = d[0] * east[0] + d[1] * east[1] + d[2] * east[2];
  let n = d[0] * north[0] + d[1] * north[1] + d[2] * north[2];
  let u = d[0] * up[0] + d[1] * up[1] + d[2] * up[2];

  let elev = u.atan2((e * e + n * n).sqrt()).to_degrees();
  let az = e.atan2(n).to_degrees().rem_euclid(360.0);
  Some((elev, az))
}

/// 计算单颗卫星在 `[start, end]` 内的过境。
fn compute_passes(
  elements: &Elements,
  constants: &Constants,
  lat: f64,
  lon: f64,
  min_elev: f64,
  start: i64,
  end: i64,
) -> Vec<Pass> {
  let epoch = elements.datetime.and_utc();
  let epoch_unix = epoch.timestamp() as f64 + f64::from(epoch.timestamp_subsec_nanos()) / 1e9;
  let station_ecef = geodetic_to_ecef(lat, lon, 0.0);
  let basis = enu_basis(lat, lon);

  let mut out = Vec::new();
  let mut t = start as f64;
  let mut aos: Option<f64> = None;
  let mut max_elev = -90.0f64;
  let mut max_t = t;
  let mut max_az = 0.0f64;

  while t <= end as f64 {
    if let Some((elev, az)) = look(constants, epoch_unix, station_ecef, basis, t)
      && elev >= min_elev
    {
      if aos.is_none() {
        aos = Some(t);
        max_elev = -90.0;
      }
      if elev > max_elev {
        max_elev = elev;
        max_t = t;
        max_az = az;
      }
    } else if let Some(a) = aos.take() {
      out.push(Pass {
        name: elements.object_name.clone().unwrap_or_default(),
        norad: elements.norad_id,
        aos: a as i64,
        los: t as i64,
        max_elev,
        max_elev_time: max_t as i64,
        azimuth: max_az,
      });
    }
    t += STEP_SECS;
  }

  if let Some(a) = aos {
    out.push(Pass {
      name: elements.object_name.clone().unwrap_or_default(),
      norad: elements.norad_id,
      aos: a as i64,
      los: end,
      max_elev,
      max_elev_time: max_t as i64,
      azimuth: max_az,
    });
  }
  out
}

/// 过境结果缓存 key：经纬度量化到约 0.5 度、仰角量化到整数，避免缓存无限增长。
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct PassKey {
  lat: i32,
  lon: i32,
  min_elev: i32,
  group: SatGroup,
}

/// 量化经纬度与仰角为缓存 key。
fn result_key(lat: f64, lon: f64, min_elev: f64, group: SatGroup) -> PassKey {
  PassKey {
    lat: (lat * 2.0).round() as i32,
    lon: (lon * 2.0).round() as i32,
    min_elev: min_elev.round() as i32,
    group,
  }
}

/// 缓存：TLE（6 小时）+ 过境结果（5 分钟）。
///
/// 两份 `Inflight` 是**分开**的：`tle_inflight` 管 TLE 回源，`inflight` 管过境计算。
/// 后者持有期间会调 `tles()` 去等前者，两级锁不会互相等待，故无死锁。
type PassCacheEntry = (Instant, (Vec<Pass>, bool));

#[derive(Default)]
pub struct Cache {
  pub(crate) tle: Mutex<HashMap<SatGroup, TleCacheEntry>>,
  results: Mutex<HashMap<PassKey, PassCacheEntry>>,
  inflight: Inflight,
  /// TLE 回源的单飞保护：`/api/passes` 与 `/api/sat-track` 共用同一份缓存，
  /// 单飞必须落在 `tles()` 里（只放在 `/api/passes` 的 handler 里挡不住后者）。
  tle_inflight: Inflight,
}

/// `GET /api/passes` 处理器。
pub async fn handler(Query(q): Query<PassQuery>, cache: Arc<Cache>) -> Response {
  if !q.lat.is_finite()
    || !q.lon.is_finite()
    || !(-90.0..=90.0).contains(&q.lat)
    || !(-180.0..=180.0).contains(&q.lon)
  {
    return (StatusCode::BAD_REQUEST, "invalid lat/lon").into_response();
  }
  let min_elev = q.min_elev.clamp(0.0, 90.0);
  let group = q.group.parse::<SatGroup>().unwrap_or_default();
  let key = result_key(q.lat, q.lon, min_elev, group);

  loop {
    // 命中结果缓存直接返回。
    if let Some((t, (passes, stale))) = cache.results.lock().ok().and_then(|g| g.get(&key).cloned())
      && t.elapsed() < RESULT_CACHE_TTL
    {
      let mut res = passes_payload(q.lat, q.lon, &passes);
      if stale {
        crate::cache::mark_stale(&mut res);
      }
      return res;
    }

    // 抢计算权；抢不到则等待，避免并发请求在冷启动/过期瞬间拿到无谓的 503。
    if let Some(_guard) = cache.inflight.acquire() {
      let result = compute(cache.clone(), q.lat, q.lon, min_elev, group).await;

      // `_guard` 存活到缓存写入完成后释放，保证等待者被唤醒时能读到新缓存。
      return match result {
        Ok((passes, stale)) => {
          if let Ok(mut g) = cache.results.lock() {
            crate::cache::evict_oldest(&mut *g, MAX_RESULT_ENTRIES);
            g.insert(key, (Instant::now(), (passes.clone(), stale)));
          }
          let mut res = passes_payload(q.lat, q.lon, &passes);
          if stale {
            crate::cache::mark_stale(&mut res);
          }
          res
        }
        Err(resp) => resp,
      };
    }

    cache.inflight.wait().await;
  }
}

/// 拉取（或复用）TLE：命中 6 小时缓存直接返回；过期时单飞回源，失败则降级到旧的 TLE。
///
/// 单飞在这里而不是调用方的 handler 里：`/api/passes` 与 `/api/sat-track` 共用本缓存，
/// TTL 到期的一瞬间两边都会有并发请求进来，各自回源会一起打 Celestrak（上游挂起时
/// 每个请求占一个阻塞线程最长 15 s，正是 `util.rs` 里写的级联场景）。
///
/// TLE 的可用期以「天」计，远超 6 小时的缓存 TTL：回源失败时宁可用过期的轨道根数，
/// 也不让过境 / 多普勒整块不可用（与 [`crate::cache::JsonCache`] 的 stale 兜底同口径）。
#[allow(clippy::result_large_err)]
pub(crate) async fn tles(cache: &Cache, group: SatGroup) -> Result<Tles, Response> {
  loop {
    let cached: Option<TleCacheEntry> = cache.tle.lock().ok().and_then(|g| g.get(&group).cloned());
    if let Some((_, lines)) = cached.as_ref().filter(|(t, _)| t.elapsed() < TLE_CACHE_TTL) {
      return Ok(Tles {
        lines: lines.clone(),
        stale: false,
      });
    }

    // 抢回源权；抢不到就等这一轮刷新结束再看新缓存（与过境计算同一套等待语义）。
    if let Some(_guard) = cache.tle_inflight.acquire() {
      match tokio::task::spawn_blocking(move || fetch_tle(group)).await {
        Ok(Ok(lines)) => {
          if let Ok(mut lock) = cache.tle.lock() {
            lock.insert(group, (Instant::now(), lines.clone()));
          }
          return Ok(Tles {
            lines,
            stale: false,
          });
        }
        Ok(Err(e)) => error!("fetch TLE failed: {e:#}"),
        Err(e) => error!("spawn_blocking failed: {e:#}"),
      }
      return match cached {
        Some((cached_at, lines)) if cached_at.elapsed() <= Duration::from_secs(7 * 86_400) => {
          warn!(?group, "upstream TLE fetch failed, serving stale TLE");
          Ok(Tles { lines, stale: true })
        }
        _ => Err((StatusCode::SERVICE_UNAVAILABLE, "TLE unavailable").into_response()),
      };
    }

    cache.tle_inflight.wait().await;
  }
}

/// 拉取（或复用）TLE 并计算过境；失败返回错误响应。
///
/// 第二个返回值是「TLE 是否为过期兜底」，供 handler 打 `X-Cache: stale`。
#[allow(clippy::result_large_err)]
async fn compute(
  cache: Arc<Cache>,
  lat: f64,
  lon: f64,
  min_elev: f64,
  group: SatGroup,
) -> Result<(Vec<Pass>, bool), Response> {
  // 1) 获取 TLE。
  let Tles { lines, stale } = tles(&cache, group).await?;

  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or(0);
  let horizon = 24 * 3600;

  // 2) 计算过境（CPU 密集，放 spawn_blocking）。
  let passes = tokio::task::spawn_blocking(move || {
    let mut all = Vec::new();
    for (name, l1, l2) in &lines {
      let Ok(elements) = Elements::from_tle(Some(name.clone()), l1.as_bytes(), l2.as_bytes())
      else {
        continue;
      };
      let Ok(constants) = Constants::from_elements(&elements) else {
        continue;
      };
      all.extend(compute_passes(
        &elements,
        &constants,
        lat,
        lon,
        min_elev,
        now,
        now + horizon,
      ));
    }
    all.sort_by_key(|p| p.aos);
    all
  })
  .await
  .map_err(|e| {
    error!("pass compute failed: {e:#}");
    (StatusCode::INTERNAL_SERVER_ERROR, "compute failed").into_response()
  })?;
  Ok((passes, stale))
}

/// 序列化过境结果为 JSON 响应。
fn passes_payload(lat: f64, lon: f64, passes: &[Pass]) -> Response {
  let payload = PassesPayload {
    source: "Celestrak / SGP4",
    lat,
    lon,
    count: passes.len(),
    passes: passes.to_vec(),
  };
  match serde_json::to_string(&payload) {
    Ok(j) => ([(CONTENT_TYPE, "application/json")], j).into_response(),
    Err(e) => {
      error!("serialize passes failed: {e:#}");
      (StatusCode::INTERNAL_SERVER_ERROR, "serialize failed").into_response()
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn cached_passes_preserve_stale_header() {
    let cache = Arc::new(Cache::default());
    let key = result_key(39.9, 116.4, 10.0, SatGroup::Amateur);
    cache
      .results
      .lock()
      .unwrap()
      .insert(key, (Instant::now(), (Vec::new(), true)));
    let response = handler(
      Query(PassQuery {
        lat: 39.9,
        lon: 116.4,
        min_elev: 10.0,
        group: "amateur".into(),
      }),
      cache,
    )
    .await;
    assert_eq!(response.headers().get("x-cache").unwrap(), "stale");
  }

  #[test]
  fn parses_tle_group() {
    let text = "\nISS (ZARYA)\n1 25544U 98067A 20194.88612269 .00002218 00000-0 31515-4 0 9992\n2 25544 51.6461 221.2784 0001413 89.1723 280.4612 15.49507896236008\n";
    let g = parse_tle_group(text);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].0, "ISS (ZARYA)");
    assert!(g[0].1.starts_with("1 "));
    assert!(g[0].2.starts_with("2 "));
  }

  #[test]
  fn ecef_and_gmst() {
    // 地面站 ECEF 的模长应接近地球半径。
    let p = geodetic_to_ecef(39.9, 116.4, 0.0);
    let r = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    assert!((r - 6378.0).abs() < 30.0);
    // GMST 应在 [0, 2π) 内。
    let g = gmst_radians(2451545.0);
    assert!((0.0..std::f64::consts::TAU).contains(&g));
  }
}

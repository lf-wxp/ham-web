//! SOTA / POTA 查询代理：按编号查询山峰（SOTA）或公园（POTA）的详情。
//!
//! 数据静态、变化缓慢，按编号缓存 30 天。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::cache::Inflight;

/// 缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(30 * 24 * 3600);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;
/// 缓存条目上限。
const MAX_ENTRIES: usize = 20_000;

/// SOTA 查询参数。
#[derive(Deserialize)]
pub struct SotaQuery {
  #[serde(rename = "ref")]
  summit: String,
}

/// POTA 查询参数。
#[derive(Deserialize)]
pub struct PotaQuery {
  #[serde(rename = "ref")]
  park: String,
}

/// 山峰详情（返回给前端）。
#[derive(Serialize, Clone)]
struct SummitInfo {
  reference: String,
  name: String,
  region: String,
  altitude_m: i32,
  points: i32,
  latitude: f64,
  longitude: f64,
}

/// 公园详情（返回给前端）。
#[derive(Serialize, Clone)]
struct ParkInfo {
  reference: String,
  name: String,
  entity: String,
  grid: String,
  latitude: f64,
  longitude: f64,
}

/// SOTA 上游响应（仅取需要的字段）。
#[derive(Deserialize)]
struct SotaUpstream {
  #[serde(default, rename = "summitCode")]
  summit_code: String,
  #[serde(default, rename = "summitName")]
  summit_name: String,
  #[serde(default, rename = "regionName")]
  region_name: String,
  #[serde(default, rename = "altM")]
  alt_m: f64,
  #[serde(default)]
  points: i32,
  #[serde(default)]
  latitude: f64,
  #[serde(default)]
  longitude: f64,
}

/// POTA 上游响应（仅取需要的字段）。
#[derive(Deserialize)]
struct PotaUpstream {
  #[serde(default)]
  reference: String,
  #[serde(default)]
  name: String,
  #[serde(default, rename = "entityName")]
  entity_name: String,
  #[serde(default)]
  grid6: String,
  #[serde(default)]
  latitude: f64,
  #[serde(default)]
  longitude: f64,
}

/// 拉取并解析 SOTA 山峰。
fn fetch_summit(r: &str) -> anyhow::Result<String> {
  let url = format!("https://api2.sota.org.uk/api/summits/{r}");
  let mut res = ureq::get(&url).call().context("sota fetch failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read sota body failed")?;
  let up: SotaUpstream = serde_json::from_str(&body).context("parse sota failed")?;
  let info = SummitInfo {
    reference: up.summit_code,
    name: up.summit_name,
    region: up.region_name,
    altitude_m: up.alt_m.round() as i32,
    points: up.points,
    latitude: up.latitude,
    longitude: up.longitude,
  };
  serde_json::to_string(&info).context("serialize summit failed")
}

/// 拉取并解析 POTA 公园。
fn fetch_park(r: &str) -> anyhow::Result<String> {
  let url = format!("https://api.pota.app/park/{r}");
  let mut res = ureq::get(&url).call().context("pota fetch failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read pota body failed")?;
  let up: PotaUpstream = serde_json::from_str(&body).context("parse pota failed")?;
  let info = ParkInfo {
    reference: up.reference,
    name: up.name,
    entity: up.entity_name,
    grid: up.grid6,
    latitude: up.latitude,
    longitude: up.longitude,
  };
  serde_json::to_string(&info).context("serialize park failed")
}

/// 缓存。
#[derive(Default)]
pub struct Cache {
  inner: Mutex<HashMap<String, (Instant, String)>>,
  inflight: Inflight,
}

impl Cache {
  async fn get_or_fetch<F>(&self, key: String, fetch: F) -> Response
  where
    F: Fn() -> anyhow::Result<String> + Send + Sync + Clone + 'static,
  {
    loop {
      if let Some((t, j)) = self.inner.lock().ok().and_then(|g| g.get(&key).cloned())
        && t.elapsed() < CACHE_TTL
      {
        return ([(CONTENT_TYPE, "application/json")], j).into_response();
      }

      // 抢刷新权；抢不到则等待，避免并发请求在缓存过期瞬间同时打上游 SOTA/POTA。
      if let Some(_guard) = self.inflight.acquire() {
        let result = tokio::task::spawn_blocking(fetch.clone()).await;

        // `_guard` 存活到缓存写入完成后释放，保证等待者被唤醒时能读到新缓存。
        return match result {
          Ok(Ok(j)) => {
            if let Ok(mut g) = self.inner.lock() {
              crate::cache::evict_oldest(&mut *g, MAX_ENTRIES);
              g.insert(key, (Instant::now(), j.clone()));
            }
            ([(CONTENT_TYPE, "application/json")], j).into_response()
          }
          Ok(Err(_)) => (StatusCode::NOT_FOUND, "not found").into_response(),
          Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "lookup unavailable").into_response(),
        };
      }

      self.inflight.wait().await;
    }
  }
}

/// `GET /api/sota` 处理器。
pub async fn sota_handler(Query(q): Query<SotaQuery>, cache: Arc<Cache>) -> Response {
  let r = q.summit.trim().to_uppercase();
  if r.len() < 3 || !crate::util::is_safe_token(&r, &['/', '-']) {
    return (StatusCode::BAD_REQUEST, "invalid ref").into_response();
  }
  let key = format!("sota:{r}");
  cache.get_or_fetch(key, move || fetch_summit(&r)).await
}

/// `GET /api/pota` 处理器。
pub async fn pota_handler(Query(q): Query<PotaQuery>, cache: Arc<Cache>) -> Response {
  let r = q.park.trim().to_uppercase();
  if r.len() < 3 || !crate::util::is_safe_token(&r, &['/', '-']) {
    return (StatusCode::BAD_REQUEST, "invalid ref").into_response();
  }
  let key = format!("pota:{r}");
  cache.get_or_fetch(key, move || fetch_park(&r)).await
}

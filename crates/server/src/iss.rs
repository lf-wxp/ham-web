//! 国际空间站（ISS）实时位置代理：拉取 wheretheiss.at（无浏览器 CORS 限制）。
//!
//! 缓存 30 秒。

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::response::Response;
use serde::Serialize;

use crate::cache::JsonCache;

/// wheretheiss.at 的 ISS（NORAD 25544）数据源。
const ISS_URL: &str = "https://api.wheretheiss.at/v1/satellites/25544";
/// 服务端缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(30);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;

/// 返回给前端的 JSON 结构。
#[derive(Serialize)]
struct IssPayload {
  latitude: f64,
  longitude: f64,
  altitude: f64,
  velocity: f64,
  visibility: String,
  timestamp: i64,
}

/// 拉取并解析 ISS 位置（同步，在 `spawn_blocking` 中执行）。
fn fetch_and_parse() -> anyhow::Result<IssPayload> {
  let mut res = crate::util::http_agent()
    .get(ISS_URL)
    .call()
    .context("fetch ISS failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read ISS body failed")?;

  let v: serde_json::Value = serde_json::from_str(&body).context("parse ISS json failed")?;

  Ok(IssPayload {
    latitude: v["latitude"].as_f64().unwrap_or(0.0),
    longitude: v["longitude"].as_f64().unwrap_or(0.0),
    altitude: v["altitude"].as_f64().unwrap_or(0.0),
    velocity: v["velocity"].as_f64().unwrap_or(0.0),
    visibility: v["visibility"].as_str().unwrap_or("").to_owned(),
    timestamp: v["timestamp"].as_i64().unwrap_or(0),
  })
}

/// ISS 位置缓存。
pub type Cache = JsonCache;

/// `GET /api/iss` 处理器。
pub async fn handler(cache: Arc<Cache>) -> Response {
  cache
    .get(CACHE_TTL, fetch_and_parse, "ISS data unavailable")
    .await
}

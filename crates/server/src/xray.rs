//! GOES X 射线通量代理：拉取 SWPC 6 小时实时通量（太阳耀斑监测）。
//!
//! 缓存 5 分钟，返回主波段（0.1–0.8nm，软 X 射线）时间序列、最新通量与耀斑级别（A/B/C/M/X）。

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::response::Response;
use serde::{Deserialize, Serialize};

use crate::cache::JsonCache;

/// NOAA SWPC GOES 主星 X 射线通量（6 小时，每分钟一个点）。
const XRAY_URL: &str = "https://services.swpc.noaa.gov/json/goes/primary/xrays-6-hour.json";
/// 服务端缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(300);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;
/// 耀斑分级所用波段（软 X 射线）。
const LONG_BAND: &str = "0.1-0.8nm";

/// 上游单条记录。
#[derive(Deserialize)]
struct RawPoint {
  time_tag: String,
  flux: f64,
  energy: String,
}

/// 返回给前端的曲线点。
#[derive(Serialize)]
struct XrayPoint {
  time: String,
  flux: f64,
}

/// 返回给前端的 JSON 结构。
#[derive(Serialize)]
struct XrayPayload {
  source: &'static str,
  updated: Option<String>,
  flux: Option<f64>,
  flare_class: &'static str,
  series: Vec<XrayPoint>,
}

/// 按 0.1–0.8nm 波段通量给出耀斑级别（NOAA 分级）。
fn flare_class(flux: f64) -> &'static str {
  if flux >= 1e-4 {
    "X"
  } else if flux >= 1e-5 {
    "M"
  } else if flux >= 1e-6 {
    "C"
  } else if flux >= 1e-7 {
    "B"
  } else {
    "A"
  }
}

/// 拉取并解析（同步，在 `spawn_blocking` 中执行）。
fn fetch_and_parse() -> anyhow::Result<XrayPayload> {
  let mut res = crate::util::http_agent()
    .get(XRAY_URL)
    .call()
    .context("fetch GOES xray failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read GOES xray body failed")?;
  let raw: Vec<RawPoint> = serde_json::from_str(&body).context("parse GOES xray failed")?;

  let series: Vec<XrayPoint> = raw
    .into_iter()
    .filter(|p| p.energy == LONG_BAND)
    .map(|p| XrayPoint {
      time: p.time_tag,
      flux: p.flux,
    })
    .collect();

  let last = series.last();
  Ok(XrayPayload {
    source: "NOAA SWPC GOES",
    updated: last.map(|p| p.time.clone()),
    flux: last.map(|p| p.flux),
    flare_class: last.map_or("", |p| flare_class(p.flux)),
    series,
  })
}

/// X 射线通量缓存。
pub type Cache = JsonCache;

/// `GET /api/xray` 处理器。
pub async fn handler(cache: Arc<Cache>) -> Response {
  cache
    .get(CACHE_TTL, fetch_and_parse, "xray flux unavailable")
    .await
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn flare_classes() {
    assert_eq!(flare_class(5e-8), "A");
    assert_eq!(flare_class(1e-7), "B");
    assert_eq!(flare_class(5e-7), "B");
    assert_eq!(flare_class(1e-6), "C");
    assert_eq!(flare_class(5e-6), "C");
    assert_eq!(flare_class(1e-5), "M");
    assert_eq!(flare_class(1e-4), "X");
  }
}

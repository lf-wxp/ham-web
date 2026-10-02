//! 点对点 HF 传播预测：直接调用 `ham-web-core` 的简化 VOACAP 引擎（无外部依赖）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::Query;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use ham_web_core::voacap;

/// 缓存时长：预测仅随月份 / 太阳黑子数变化，缓存 6 小时足够。
const CACHE_TTL: Duration = Duration::from_secs(6 * 3600);
/// 缓存条目上限。
const MAX_ENTRIES: usize = 10_000;

/// 查询参数。
#[derive(Deserialize)]
pub struct VoacapQuery {
  /// 发射端网格（4 或 6 位 Maidenhead）。
  tx: String,
  /// 接收端网格。
  rx: String,
  /// 月份（1–12），默认 10。
  #[serde(default)]
  month: Option<u32>,
  /// 太阳黑子数，默认 100。
  #[serde(default)]
  ssn: Option<f64>,
  /// UTC 时刻（0–24，可为小数）。省略时按路径日照最佳情况估算。
  #[serde(default)]
  hour: Option<f64>,
}

/// 缓存 key 与条目。
type CacheKey = (String, String, u32, i32, Option<i32>);
type CacheEntry = (Instant, voacap::Prediction);

/// 缓存。
#[derive(Default)]
pub struct Cache {
  inner: Mutex<HashMap<CacheKey, CacheEntry>>,
}

/// 缓存 key：网格大写规范化，SSN 量化到 0.1 精度，时刻量化到半小时。
fn key(q: &VoacapQuery) -> CacheKey {
  let month = q.month.unwrap_or(10).clamp(1, 12);
  let ssn = (q.ssn.unwrap_or(100.0).clamp(0.0, 400.0) * 10.0).round() as i32;
  let hour = q.hour.map(|h| (h.clamp(0.0, 24.0) * 2.0).round() as i32);
  (
    q.tx.trim().to_uppercase(),
    q.rx.trim().to_uppercase(),
    month,
    ssn,
    hour,
  )
}

/// `GET /api/voacap` 处理器。
pub async fn handler(Query(q): Query<VoacapQuery>, cache: Arc<Cache>) -> Response {
  let month = q.month.unwrap_or(10);
  let ssn = q.ssn.unwrap_or(100.0);
  if !(1..=12).contains(&month) {
    return (StatusCode::BAD_REQUEST, "invalid month").into_response();
  }
  if q.hour.is_some_and(|h| !(0.0..=24.0).contains(&h)) {
    return (StatusCode::BAD_REQUEST, "invalid hour").into_response();
  }

  let k = key(&q);
  if let Some((t, r)) = cache.inner.lock().ok().and_then(|g| g.get(&k).cloned())
    && t.elapsed() < CACHE_TTL
  {
    return json(&r);
  }

  match voacap::predict_at(&q.tx, &q.rx, month, ssn, q.hour) {
    Some(p) => {
      if let Ok(mut g) = cache.inner.lock() {
        crate::cache::evict_oldest(&mut *g, MAX_ENTRIES);
        g.insert(k, (Instant::now(), p.clone()));
      }
      json(&p)
    }
    None => (StatusCode::BAD_REQUEST, "invalid grid").into_response(),
  }
}

fn json(p: &voacap::Prediction) -> Response {
  match serde_json::to_string(p) {
    Ok(j) => ([(CONTENT_TYPE, "application/json")], j).into_response(),
    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "serialize failed").into_response(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn key_normalizes_grid() {
    let q = VoacapQuery {
      tx: "om89".into(),
      rx: " io91 ".into(),
      month: Some(10),
      ssn: Some(100.0),
      hour: None,
    };
    assert_eq!(key(&q), ("OM89".into(), "IO91".into(), 10, 1000, None));
  }

  #[test]
  fn key_quantizes_hour() {
    let q = VoacapQuery {
      tx: "OM89".into(),
      rx: "IO91".into(),
      month: Some(10),
      ssn: Some(100.0),
      hour: Some(12.4),
    };
    // 12.4 h → 量化到半小时精度
    assert_eq!(key(&q).4, Some(25));
  }
}

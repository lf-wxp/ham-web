//! 中继台数据库代理：按国家查询 RepeaterBook 收录的中继台。

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
use crate::util::urlencode_query;

/// 缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 3600);
/// 上游响应体上限。
const MAX_BODY: u64 = 16 << 20;
/// 缓存条目上限。
const MAX_ENTRIES: usize = 100;
/// 单次最多返回的中继台数。
const MAX_RESULTS: usize = 500;
/// RepeaterBook 要求标识调用方的 User-Agent。
const USER_AGENT: &str = "ham-exam-web (https://ham.onlyxp.me)";

/// 查询参数。
#[derive(Deserialize)]
pub struct RepeaterQuery {
  #[serde(default = "default_country")]
  country: String,
}

fn default_country() -> String {
  "China".to_owned()
}

/// 返回给前端的一条中继台。
#[derive(Serialize, Clone)]
struct Repeater {
  callsign: String,
  frequency_mhz: String,
  offset_mhz: String,
  pl: String,
  city: String,
  state: String,
  r#use: String,
  status: String,
  lat: f64,
  lon: f64,
}

/// 返回结构。
#[derive(Serialize)]
struct Payload {
  country: String,
  count: usize,
  results: Vec<Repeater>,
}

/// RepeaterBook 上游字段（字段名含空格，用 rename 匹配）。
#[derive(Deserialize)]
struct RbResult {
  #[serde(default, rename = "Callsign")]
  callsign: String,
  #[serde(default, rename = "Frequency")]
  frequency: String,
  #[serde(default, rename = "Input Freq")]
  input_freq: String,
  #[serde(default, rename = "PL")]
  pl: String,
  #[serde(default, rename = "Nearest City")]
  nearest_city: String,
  #[serde(default, rename = "State")]
  state: String,
  #[serde(default, rename = "Use")]
  r#use: String,
  #[serde(default, rename = "Operational Status")]
  status: String,
  #[serde(default, rename = "Lat")]
  lat: String,
  #[serde(default, rename = "Long")]
  lon: String,
}

#[derive(Deserialize)]
struct RbResponse {
  #[serde(default)]
  results: Vec<RbResult>,
}

fn parse_f(s: &str) -> f64 {
  s.trim().parse().unwrap_or(0.0)
}

/// 按国家选择 RepeaterBook 端点：北美用 `export.php`，其余用 `exportROW.php`。
fn endpoint_for(country: &str) -> &'static str {
  let c = country.to_ascii_lowercase();
  if c.contains("united states") || c.contains("canada") || c.contains("mexico") {
    "https://www.repeaterbook.com/api/export.php"
  } else {
    "https://www.repeaterbook.com/api/exportROW.php"
  }
}

/// 拉取并解析 RepeaterBook。
fn fetch_and_parse(country: &str) -> anyhow::Result<Payload> {
  // country 可能含空格（如 "United States"），必须 percent-encode 后再拼接。
  let url = format!(
    "{}?country={}",
    endpoint_for(country),
    urlencode_query(country)
  );
  let mut res = ureq::get(&url)
    .header("User-Agent", USER_AGENT)
    .call()
    .context("repeaterbook fetch failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read repeaterbook body failed")?;
  let up: RbResponse = serde_json::from_str(&body).context("parse repeaterbook failed")?;

  let results: Vec<Repeater> = up
    .results
    .into_iter()
    .filter(|r| !r.callsign.is_empty())
    .take(MAX_RESULTS)
    .map(|r| {
      let freq = parse_f(&r.frequency);
      let input = parse_f(&r.input_freq);
      let offset = if freq > 0.0 && input > 0.0 {
        input - freq
      } else {
        0.0
      };
      Repeater {
        callsign: r.callsign,
        frequency_mhz: format!("{freq:.4}"),
        offset_mhz: format!("{offset:+.4}"),
        pl: r.pl,
        city: r.nearest_city,
        state: r.state,
        r#use: r.r#use,
        status: r.status,
        lat: parse_f(&r.lat),
        lon: parse_f(&r.lon),
      }
    })
    .collect();
  let count = results.len();
  Ok(Payload {
    country: country.to_owned(),
    count,
    results,
  })
}

/// 缓存。
#[derive(Default)]
pub struct Cache {
  inner: Mutex<HashMap<String, (Instant, String)>>,
  inflight: Inflight,
}

/// `GET /api/repeaters` 处理器。
pub async fn handler(Query(q): Query<RepeaterQuery>, cache: Arc<Cache>) -> Response {
  let country = q.country.trim().to_owned();
  if country.is_empty() {
    return (StatusCode::BAD_REQUEST, "missing country").into_response();
  }
  let key = format!("rb:{country}");

  loop {
    if let Some((t, j)) = cache.inner.lock().ok().and_then(|g| g.get(&key).cloned())
      && t.elapsed() < CACHE_TTL
    {
      return ([(CONTENT_TYPE, "application/json")], j).into_response();
    }

    // 抢刷新权；抢不到则等待，避免并发请求在缓存过期瞬间同时打上游 RepeaterBook。
    if let Some(_guard) = cache.inflight.acquire() {
      let country_clone = country.clone();
      let result = tokio::task::spawn_blocking(move || {
        fetch_and_parse(&country_clone).and_then(|p| serde_json::to_string(&p).context("serialize"))
      })
      .await;

      // `_guard` 存活到缓存写入完成后释放，保证等待者被唤醒时能读到新缓存。
      return match result {
        Ok(Ok(j)) => {
          if let Ok(mut g) = cache.inner.lock() {
            crate::cache::evict_oldest(&mut *g, MAX_ENTRIES);
            g.insert(key, (Instant::now(), j.clone()));
          }
          ([(CONTENT_TYPE, "application/json")], j).into_response()
        }
        Ok(Err(_)) => (StatusCode::SERVICE_UNAVAILABLE, "repeaters unavailable").into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "repeaters unavailable").into_response(),
      };
    }

    cache.inflight.wait().await;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn endpoint_selects_region() {
    assert_eq!(
      endpoint_for("China"),
      "https://www.repeaterbook.com/api/exportROW.php"
    );
    assert_eq!(
      endpoint_for("United States"),
      "https://www.repeaterbook.com/api/export.php"
    );
    assert_eq!(
      endpoint_for("Canada"),
      "https://www.repeaterbook.com/api/export.php"
    );
  }

  #[test]
  fn parses_rb_result() {
    let j = r#"{"results":[{"Callsign":"BR1BJ","Frequency":"145.65000","Input Freq":"145.05000","PL":"88.5","Nearest City":"Beijing","State":"Beijing","Use":"OPEN","Operational Status":"On-air","Lat":"39.9","Long":"116.4"}]}"#;
    let up: RbResponse = serde_json::from_str(j).unwrap();
    assert_eq!(up.results.len(), 1);
    assert_eq!(up.results[0].callsign, "BR1BJ");
    assert_eq!(up.results[0].nearest_city, "Beijing");
  }
}

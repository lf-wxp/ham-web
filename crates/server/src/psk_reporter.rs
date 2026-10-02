//! PSK Reporter 代理：按发送呼号查询「谁收到了我发的信号」（数字模式实时接收报告）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::cache::Inflight;

/// 缓存时长：实时报告，1 分钟。
const CACHE_TTL: Duration = Duration::from_secs(60);
/// 上游响应体上限。
const MAX_BODY: u64 = 8 << 20;
/// 缓存条目上限。
const MAX_ENTRIES: usize = 1000;

/// 查询参数。
#[derive(Deserialize)]
pub struct PskQuery {
  /// 发送方呼号（我自己的呼号，查询谁收到了我）。
  callsign: String,
}

/// 返回给前端的一条接收报告。
#[derive(Serialize, Clone)]
struct PskReport {
  receiver: String,
  freq_khz: f64,
  band: String,
  mode: String,
  snr: i32,
  flow_start_seconds: i64,
  receiver_locator: String,
}

/// 上游返回结构（仅取需要的字段）。
#[derive(Deserialize)]
struct Upstream {
  #[serde(default, rename = "receptionReports")]
  reception_reports: Vec<UpstreamReport>,
}

#[derive(Deserialize)]
struct UpstreamReport {
  #[serde(default, rename = "receiverCallsign")]
  receiver_callsign: String,
  #[serde(default)]
  frequency: f64,
  #[serde(default)]
  mode: String,
  #[serde(default, rename = "sNR")]
  snr: i32,
  #[serde(default, rename = "flowStartSeconds")]
  flow_start_seconds: i64,
  #[serde(default, rename = "receiverLocator")]
  receiver_locator: String,
}

/// 剥离 JSONP 包装（`cb({...})` → `{...}`）。
fn unwrap_jsonp(s: &str) -> &str {
  let s = s.trim();
  if let (Some(start), Some(end)) = (s.find('('), s.rfind(')'))
    && end > start
  {
    return &s[start + 1..end];
  }
  s
}

/// 拉取并解析 PSK Reporter。
fn fetch_and_parse(call: &str) -> anyhow::Result<Vec<PskReport>> {
  let url = format!("https://pskreporter.info/pskqueryapi?senderCallsign={call}");
  let mut res = crate::util::http_agent()
    .get(&url)
    .call()
    .context("fetch pskreporter failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read pskreporter body failed")?;
  let json = unwrap_jsonp(&body);
  let up: Upstream = serde_json::from_str(json).context("parse pskreporter failed")?;

  let mut reports: Vec<PskReport> = up
    .reception_reports
    .into_iter()
    .filter(|r| !r.receiver_callsign.is_empty())
    .map(|r| PskReport {
      receiver: r.receiver_callsign,
      freq_khz: r.frequency / 1000.0,
      band: ham_web_core::frequencies::band_of(r.frequency / 1_000_000.0).to_owned(),
      mode: r.mode,
      snr: r.snr,
      flow_start_seconds: r.flow_start_seconds,
      receiver_locator: r.receiver_locator,
    })
    .collect();
  // 最新在前，最多 100 条。
  reports.sort_by_key(|r| std::cmp::Reverse(r.flow_start_seconds));
  reports.truncate(100);
  Ok(reports)
}

/// 缓存。
#[derive(Default)]
pub struct Cache {
  inner: Mutex<HashMap<String, (Instant, Vec<PskReport>)>>,
  inflight: Inflight,
}

/// `GET /api/psk-reporter` 处理器。
pub async fn handler(Query(q): Query<PskQuery>, cache: Arc<Cache>) -> Response {
  let call = q.callsign.trim().to_uppercase();
  if call.is_empty() || !crate::util::is_safe_token(&call, &['/']) {
    return (StatusCode::BAD_REQUEST, "invalid callsign").into_response();
  }

  loop {
    if let Some((t, r)) = cache.inner.lock().ok().and_then(|g| g.get(&call).cloned())
      && t.elapsed() < CACHE_TTL
    {
      return json(&r);
    }

    if let Some(_guard) = cache.inflight.acquire() {
      let call_clone = call.clone();
      let fetched = tokio::task::spawn_blocking(move || fetch_and_parse(&call_clone)).await;

      // `_guard` 存活到缓存写入完成后释放，保证等待者被唤醒时能读到新缓存。
      return match fetched {
        Ok(Ok(r)) => {
          if let Ok(mut g) = cache.inner.lock() {
            crate::cache::evict_oldest(&mut *g, MAX_ENTRIES);
            g.insert(call, (Instant::now(), r.clone()));
          }
          json(&r)
        }
        Ok(Err(e)) => {
          error!("fetch pskreporter failed: {e:#}");
          (StatusCode::SERVICE_UNAVAILABLE, "psk reporter unavailable").into_response()
        }
        Err(e) => {
          error!("spawn_blocking failed: {e:#}");
          (StatusCode::SERVICE_UNAVAILABLE, "psk reporter unavailable").into_response()
        }
      };
    }

    cache.inflight.wait().await;
  }
}

fn json(r: &[PskReport]) -> Response {
  #[derive(Serialize)]
  struct Payload<'a> {
    reports: &'a [PskReport],
  }
  match serde_json::to_string(&Payload { reports: r }) {
    Ok(j) => ([(CONTENT_TYPE, "application/json")], j).into_response(),
    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "serialize failed").into_response(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn unwraps_jsonp() {
    assert_eq!(unwrap_jsonp("cb({\"a\":1});"), "{\"a\":1}");
    assert_eq!(unwrap_jsonp("{\"a\":1}"), "{\"a\":1}");
  }

  #[test]
  fn parses_reports() {
    let j = r#"{"receptionReports":[{"receiverCallsign":"XX1XX","frequency":14074000,"mode":"FT8","sNR":-12,"flowStartSeconds":1670000000,"receiverLocator":"JN18eu"}]}"#;
    let up: Upstream = serde_json::from_str(j).unwrap();
    assert_eq!(up.reception_reports.len(), 1);
    assert_eq!(up.reception_reports[0].mode, "FT8");
  }
}

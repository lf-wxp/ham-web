//! 呼号查询代理：按呼号查询操作员姓名、网格与 QTH，供通联日志录入自动补全。
//!
//! 数据源按优先级：
//! 1. Callook（`callook.info`，免费 JSON，覆盖美国 / 加拿大呼号，无需鉴权）；
//! 2. HamQTH（需设置 `HAMQTH_USER` / `HAMQTH_PASS` 环境变量，覆盖国际呼号）。
//!
//! 结果按呼号缓存 7 天（呼号资料变化缓慢）。

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
use crate::util::urlencode_query;

/// 缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 3600);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;
/// 缓存条目上限。
const MAX_ENTRIES: usize = 10_000;

/// 查询参数。
#[derive(Deserialize)]
pub struct CallsignQuery {
  /// 要查询的呼号。
  callsign: String,
}

/// 返回给前端的呼号资料。
#[derive(Serialize, Clone)]
struct CallsignInfo {
  callsign: String,
  name: String,
  grid: String,
  qth: String,
  country: String,
  source: String,
}

/// Callook 响应（仅取需要的字段）。
#[derive(Deserialize)]
struct Callook {
  #[serde(default)]
  status: String,
  #[serde(default)]
  current: Option<CallookCurrent>,
}

#[derive(Default, Deserialize)]
struct CallookCurrent {
  #[serde(default)]
  name: String,
  #[serde(default)]
  address: CallookAddress,
  #[serde(default)]
  location: CallookLocation,
}

#[derive(Default, Deserialize)]
struct CallookAddress {
  #[serde(default)]
  line1: String,
  #[serde(default)]
  line2: String,
}

#[derive(Default, Deserialize)]
struct CallookLocation {
  #[serde(default)]
  gridsquare: String,
}

/// 从 Callook 查询（美国 / 加拿大，免费）。
fn from_callook(call: &str) -> anyhow::Result<CallsignInfo> {
  let url = format!("https://callook.info/{call}/json");
  let mut res = ureq::get(&url).call().context("callook fetch failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read callook body failed")?;
  let c: Callook = serde_json::from_str(&body).context("parse callook failed")?;
  if c.status != "VALID" {
    anyhow::bail!("callook status: {}", c.status);
  }
  let cur = c.current.unwrap_or_default();
  let qth = [cur.address.line1, cur.address.line2]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(", ");
  Ok(CallsignInfo {
    callsign: call.to_owned(),
    name: cur.name,
    grid: cur.location.gridsquare,
    qth,
    country: "United States".to_owned(),
    source: "Callook".to_owned(),
  })
}

/// 提取 HamQTH XML 的 `<name>value</name>`。
fn tag(xml: &str, name: &str) -> Option<String> {
  let open = format!("<{name}>");
  let close = format!("</{name}>");
  let start = xml.find(&open)? + open.len();
  let end = xml[start..].find(&close)? + start;
  Some(xml[start..end].trim().to_owned())
}

/// 从 HamQTH 查询（国际，需账号）。
fn from_hamqth(call: &str) -> anyhow::Result<CallsignInfo> {
  let user = std::env::var("HAMQTH_USER").context("HAMQTH_USER not set")?;
  let pass = std::env::var("HAMQTH_PASS").context("HAMQTH_PASS not set")?;
  // HamQTH 官方协议即要求以查询参数传凭据；这里至少保证参数被正确 percent-encode，
  // 避免密码含 `&`/`=`/空格 等字符时破坏 URL 结构或注入额外参数。
  let url = format!(
    "https://www.hamqth.com/xml.php?u={}&p={}&callsign={}",
    urlencode_query(&user),
    urlencode_query(&pass),
    urlencode_query(call)
  );
  let mut res = ureq::get(&url).call().context("hamqth fetch failed")?;
  let xml = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read hamqth body failed")?;
  if xml.contains("<error>") {
    let msg = tag(&xml, "error").unwrap_or_else(|| "unknown".to_owned());
    anyhow::bail!("hamqth error: {msg}");
  }
  let first = tag(&xml, "first_name").unwrap_or_default();
  let last = tag(&xml, "last_name").unwrap_or_default();
  let name = [first, last]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(" ");
  Ok(CallsignInfo {
    callsign: call.to_owned(),
    name: if name.is_empty() {
      tag(&xml, "nick").unwrap_or_default()
    } else {
      name
    },
    grid: tag(&xml, "grid").unwrap_or_default(),
    qth: tag(&xml, "qth").unwrap_or_default(),
    country: tag(&xml, "country").unwrap_or_default(),
    source: "HamQTH".to_owned(),
  })
}

/// 拉取并解析呼号资料：优先 Callook，失败再试 HamQTH。
fn fetch_and_parse(call: &str) -> anyhow::Result<CallsignInfo> {
  match from_callook(call) {
    Ok(info) => Ok(info),
    Err(e) => {
      error!("callook lookup {call} failed: {e:#}, trying HamQTH");
      from_hamqth(call)
    }
  }
}

/// 缓存。
#[derive(Default)]
pub struct Cache {
  inner: Mutex<HashMap<String, (Instant, CallsignInfo)>>,
  inflight: Inflight,
}

/// `GET /api/callsign` 处理器。
pub async fn handler(Query(q): Query<CallsignQuery>, cache: Arc<Cache>) -> Response {
  let call = q.callsign.trim().to_uppercase();
  if call.len() < 3 || !crate::util::is_safe_token(&call, &['/']) {
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
          error!("callsign lookup {call} failed: {e:#}");
          (StatusCode::SERVICE_UNAVAILABLE, "callsign unavailable").into_response()
        }
        Err(e) => {
          error!("spawn_blocking failed: {e:#}");
          (StatusCode::SERVICE_UNAVAILABLE, "callsign unavailable").into_response()
        }
      };
    }

    cache.inflight.wait().await;
  }
}

fn json(r: &CallsignInfo) -> Response {
  match serde_json::to_string(r) {
    Ok(j) => ([(CONTENT_TYPE, "application/json")], j).into_response(),
    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "serialize failed").into_response(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extracts_hamqth_tags() {
    let xml = "<HamQTH><search><callsign>W1AW</callsign><first_name>ARRL</first_name><grid>FN31pr</grid><qth>Newington</qth><country>United States</country></search></HamQTH>";
    assert_eq!(tag(xml, "grid").as_deref(), Some("FN31pr"));
    assert_eq!(tag(xml, "first_name").as_deref(), Some("ARRL"));
    assert_eq!(tag(xml, "qth").as_deref(), Some("Newington"));
    assert_eq!(tag(xml, "missing"), None);
  }

  #[test]
  fn callook_invalid_status_fails() {
    let body = r#"{"status":"INVALID"}"#;
    let c: Callook = serde_json::from_str(body).unwrap();
    assert_ne!(c.status, "VALID");
  }
}

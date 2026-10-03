//! 呼号查询代理：按呼号查询操作员姓名、网格与 QTH，供通联日志录入自动补全。
//!
//! 数据源按优先级：
//! 1. Callook（`callook.info`，免费 JSON，覆盖美国 / 加拿大呼号，无需鉴权）；
//! 2. HamQTH（需设置 `HAMQTH_USER` / `HAMQTH_PASS` 环境变量，覆盖国际呼号）；
//! 3. **本地 DXCC 前缀库**（内置、离线）：上游都没有资料时只给出国家 / 地区，
//!    不猜姓名、QTH 与网格 —— 凭空生成的网格会污染日志与奖章统计。
//!
//! 响应状态码：`200` 查到资料（`source` 标明来源，含 `DXCC` 回退）；`400` 呼号非法；
//! `404` 前缀无法识别（多半是拼写错误）；`503` 仅用于服务内部异常
//! （如阻塞线程池故障），不再表示「没有资料」。
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
  let mut res = crate::util::http_agent()
    .get(&url)
    .call()
    .context("callook fetch failed")?;
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

/// 反转 XML 中最常见的 5 个预定义实体（HamQTH 返回的字段可能把 `&`、`<` 等转义）。
fn unescape_xml(s: &str) -> String {
  s.replace("&amp;", "&")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&quot;", "\"")
    .replace("&apos;", "'")
}

/// 提取 HamQTH XML 的 `<name>value</name>`。
fn tag(xml: &str, name: &str) -> Option<String> {
  let open = format!("<{name}>");
  let close = format!("</{name}>");
  let start = xml.find(&open)? + open.len();
  let end = xml[start..].find(&close)? + start;
  Some(unescape_xml(xml[start..end].trim()))
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
  let mut res = crate::util::http_agent()
    .get(&url)
    .call()
    .context("hamqth fetch failed")?;
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

/// 上游都没有资料时的本地回退：用内置 DXCC 前缀库识别国家 / 地区。
///
/// 只填 `country`（实体中文名），其余字段留空。**不要**用实体中心点去合成
/// `grid`：那既不是台站真实网格，还会被日志与奖章统计当成有效数据。
fn dxcc_fallback(call: &str) -> Option<CallsignInfo> {
  let entity = ham_web_core::dxcc::lookup(call)?;
  Some(CallsignInfo {
    callsign: call.to_owned(),
    name: String::new(),
    grid: String::new(),
    qth: String::new(),
    country: entity.name.to_owned(),
    source: "DXCC".to_owned(),
  })
}

/// 写入缓存（含回退结果），并按容量上限淘汰最旧条目。
fn cache_put(cache: &Cache, call: &str, info: CallsignInfo) {
  if let Ok(mut g) = cache.inner.lock() {
    crate::cache::evict_oldest(&mut *g, MAX_ENTRIES);
    g.insert(call.to_owned(), (Instant::now(), info));
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
          cache_put(&cache, &call, r.clone());
          json(&r)
        }
        Ok(Err(e)) => {
          error!("callsign lookup {call} failed: {e:#}");
          // 上游查不到（地区不覆盖 / 未配置账号 / 网络异常）时退回本地 DXCC：
          // 至少给出国家 / 地区，避免整条查询因一个地区缺资料而不可用。
          match dxcc_fallback(&call) {
            Some(info) => {
              cache_put(&cache, &call, info.clone());
              json(&info)
            }
            None => (StatusCode::NOT_FOUND, "callsign not found").into_response(),
          }
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
  fn tag_unescapes_xml_entities() {
    let xml =
      "<HamQTH><search><qth>Dolgeville &amp; NY</qth><name>A&amp;B</name></search></HamQTH>";
    assert_eq!(tag(xml, "qth").as_deref(), Some("Dolgeville & NY"));
    assert_eq!(tag(xml, "name").as_deref(), Some("A&B"));
  }

  #[test]
  fn callook_invalid_status_fails() {
    let body = r#"{"status":"INVALID"}"#;
    let c: Callook = serde_json::from_str(body).unwrap();
    assert_ne!(c.status, "VALID");
  }

  #[test]
  fn dxcc_fallback_gives_country_only() {
    // BV9P 为东沙群岛：Callook 不覆盖（返回 INVALID），HamQTH 未配置时会走到这里。
    let info = dxcc_fallback("BV9PAA").expect("应识别出东沙群岛");
    assert_eq!(info.country, "东沙群岛");
    assert_eq!(info.source, "DXCC");
    assert_eq!(info.callsign, "BV9PAA");
    // 姓名 / QTH / 网格必须留空：绝不能凭空合成，否则会污染日志与奖章统计。
    assert!(info.name.is_empty() && info.grid.is_empty() && info.qth.is_empty());
  }

  #[test]
  fn dxcc_fallback_handles_unknown_and_slash_suffix() {
    assert!(dxcc_fallback("").is_none());
    // 无法归属任何实体（海上移动）→ 交由上层返回 404。
    assert!(dxcc_fallback("BG4XXX/MM").is_none());
    // 常见修饰后缀不应影响识别。
    assert_eq!(
      dxcc_fallback("BG4XXX/P").map(|i| i.country),
      Some("中国".to_owned())
    );
  }
}

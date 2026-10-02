//! NOAA 空间天气警报代理：拉取 SWPC alerts.json（无浏览器 CORS 限制）。
//!
//! 缓存 5 分钟，返回最近的警报（产品 ID、发布时间、级别、正文），级别从正文提取。

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::response::Response;
use serde::Serialize;

use crate::cache::JsonCache;

/// NOAA SWPC 空间天气警报数据源。
const ALERTS_URL: &str = "https://services.swpc.noaa.gov/products/alerts.json";
/// 服务端缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(300);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;
/// 返回的最大警报数。
const MAX_ALERTS: usize = 20;

/// 一条警报。
#[derive(Serialize, Clone)]
struct Alert {
  product_id: String,
  issue_time: String,
  level: String,
  message: String,
}

/// 从正文提取首个 G / S / R 级别（如 `G1`、`S2`）。
fn extract_level(msg: &str) -> String {
  for tok in msg.split_whitespace() {
    let t: String = tok.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if (t.starts_with('G') || t.starts_with('S') || t.starts_with('R'))
      && t.len() >= 2
      && t[1..].chars().all(|c| c.is_ascii_digit())
    {
      return t;
    }
  }
  String::new()
}

/// 拉取并解析 SWPC（同步，在 `spawn_blocking` 中执行）。
fn fetch_and_parse() -> anyhow::Result<Vec<Alert>> {
  let mut res = crate::util::http_agent()
    .get(ALERTS_URL)
    .call()
    .context("fetch NOAA alerts failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read NOAA alerts body failed")?;

  let v: serde_json::Value = serde_json::from_str(&body).context("parse NOAA alerts failed")?;
  let arr = v.as_array().context("alerts not array")?;

  let mut alerts: Vec<Alert> = arr
    .iter()
    .filter_map(|item| {
      let message = item.get("message")?.as_str()?.to_owned();
      if message.trim().is_empty() {
        return None;
      }
      Some(Alert {
        product_id: item
          .get("product_id")
          .and_then(|x| x.as_str())
          .unwrap_or("")
          .to_owned(),
        issue_time: item
          .get("issue_datetime")
          .and_then(|x| x.as_str())
          .unwrap_or("")
          .to_owned(),
        level: extract_level(&message),
        message,
      })
    })
    .collect();
  alerts.truncate(MAX_ALERTS);
  Ok(alerts)
}

/// 空间天气警报缓存。
pub type Cache = JsonCache;

/// `GET /api/alerts` 处理器。
pub async fn handler(cache: Arc<Cache>) -> Response {
  cache
    .get(
      CACHE_TTL,
      fetch_and_parse,
      "space weather alerts unavailable",
    )
    .await
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extracts_levels() {
    assert_eq!(extract_level("NOAA Scale: G1 - Minor"), "G1");
    assert_eq!(
      extract_level("Geomagnetic Storm Category G3 Predicted"),
      "G3"
    );
    assert_eq!(extract_level("10MeV Proton Event S1 - Minor"), "S1");
    assert_eq!(extract_level("Type II Radio Emission"), "");
  }
}

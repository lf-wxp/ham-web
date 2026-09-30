//! DX 实时热点（DX Cluster）代理：拉取 DXWatch 全球 DX spot（无浏览器 CORS 限制）。
//!
//! spot 更新频繁，缓存 60 秒。返回报告者、频率、DX 台、备注、时间与年龄。

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::response::Response;
use serde::Serialize;

use crate::cache::JsonCache;

/// DXWatch DX spot 数据源。
const DXWATCH_URL: &str = "https://www.dxwatch.com/dxsd1/s.php?s=0&r=50";
/// 服务端缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(60);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;
/// 返回的最大 spot 数。
const MAX_SPOTS: usize = 50;

/// 一条 DX spot。
#[derive(Serialize, Clone)]
struct Spot {
  spotter: String,
  freq_khz: u32,
  dx: String,
  country: String,
  comment: String,
  time: String,
  age: u32,
}

/// 从 `ci` 缓存对象读取呼号对应的国家/地区名。
fn country_of(ci: Option<&serde_json::Map<String, serde_json::Value>>, call: &str) -> String {
  ci.and_then(|c| c.get(call))
    .and_then(|val| val.as_array())
    .and_then(|arr| arr.get(1))
    .and_then(|x| x.as_str())
    .unwrap_or("")
    .to_owned()
}

/// 拉取并解析 DXWatch（同步，在 `spawn_blocking` 中执行）。
fn fetch_and_parse() -> anyhow::Result<Vec<Spot>> {
  let mut res = ureq::get(DXWATCH_URL)
    .call()
    .context("fetch DXWatch failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read DXWatch body failed")?;

  let v: serde_json::Value = serde_json::from_str(&body).context("parse DXWatch json failed")?;
  let s = v.get("s").and_then(|x| x.as_object()).context("no spots")?;
  let ci = v.get("ci").and_then(|x| x.as_object());

  let mut spots: Vec<Spot> = s
    .values()
    .filter_map(|val| {
      let arr = val.as_array()?;
      if arr.len() < 6 {
        return None;
      }
      let dx = arr[2].as_str().unwrap_or("").to_owned();
      if dx.is_empty() {
        return None;
      }
      Some(Spot {
        spotter: arr[0].as_str().unwrap_or("").to_owned(),
        freq_khz: arr[1].as_u64().unwrap_or(0) as u32,
        country: country_of(ci, &dx),
        dx,
        comment: arr[3].as_str().unwrap_or("").to_owned(),
        time: arr[4].as_str().unwrap_or("").to_owned(),
        age: arr[5].as_u64().unwrap_or(0) as u32,
      })
    })
    .collect();
  spots.sort_by_key(|s| s.age);
  spots.truncate(MAX_SPOTS);
  Ok(spots)
}

/// DX spot 缓存。
pub type Cache = JsonCache;

/// `GET /api/spots` 处理器。
pub async fn handler(cache: Arc<Cache>) -> Response {
  cache
    .get(CACHE_TTL, fetch_and_parse, "dx spots unavailable")
    .await
}

#[cfg(test)]
mod tests {
  #[test]
  fn parses_dxwatch_spots() {
    let json = r#"{"s":{"1":["OH6NUW",7015,"VE7CA","","0549z 30 Sep",340,12,1]}}"#;
    let v: serde_json::Value = serde_json::from_str(json).unwrap();
    let s = v.get("s").and_then(|x| x.as_object()).unwrap();
    let arr = s.values().next().unwrap().as_array().unwrap();
    assert_eq!(arr[0].as_str().unwrap(), "OH6NUW");
    assert_eq!(arr[1].as_u64().unwrap(), 7015);
    assert_eq!(arr[2].as_str().unwrap(), "VE7CA");
  }
}

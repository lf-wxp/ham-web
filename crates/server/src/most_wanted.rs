//! DXCC Most Wanted 榜单代理：服务端拉取 Club Log 实时榜单（无浏览器 CORS 限制）。
//!
//! 结果缓存 6 小时（榜单变化缓慢），返回前 30 名最稀有实体的排名与 ADIF 编号。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::response::Response;
use serde::Serialize;

use crate::cache::JsonCache;

/// Club Log Most Wanted 数据源（`api=1` 返回 `{"rank": "adif"}` 映射）。
const CLUBLOG_URL: &str = "https://clublog.org/mostwanted.php?api=1";
/// 服务端缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(6 * 3600);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;
/// 返回的榜单条目数。
const TOP_N: usize = 30;

/// 一条榜单记录。
#[derive(Serialize, Clone)]
struct WantedEntry {
  rank: u32,
  adif: u32,
}

/// 返回给前端的 JSON 结构。
#[derive(Serialize)]
struct WantedPayload {
  source: &'static str,
  count: usize,
  entries: Vec<WantedEntry>,
}

/// 拉取并解析 Club Log（同步，在 `spawn_blocking` 中执行）。
fn fetch_and_parse() -> anyhow::Result<WantedPayload> {
  let mut res = crate::util::http_agent()
    .get(CLUBLOG_URL)
    .call()
    .context("fetch Club Log failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read Club Log body failed")?;

  let map: HashMap<String, String> =
    serde_json::from_str(&body).context("parse Club Log json failed")?;
  let mut pairs: Vec<(u32, u32)> = map
    .into_iter()
    .filter_map(|(r, a)| Some((r.parse::<u32>().ok()?, a.parse::<u32>().ok()?)))
    .collect();
  pairs.sort_by_key(|(rank, _)| *rank);
  let entries: Vec<WantedEntry> = pairs
    .into_iter()
    .take(TOP_N)
    .map(|(rank, adif)| WantedEntry { rank, adif })
    .collect();

  Ok(WantedPayload {
    source: "Club Log",
    count: entries.len(),
    entries,
  })
}

/// Most Wanted 缓存。
pub type Cache = JsonCache;

/// `GET /api/most-wanted` 处理器。
pub async fn handler(cache: Arc<Cache>) -> Response {
  cache
    .get(CACHE_TTL, fetch_and_parse, "most wanted data unavailable")
    .await
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_rank_to_adif_map() {
    let json = r#"{"1":"344","2":"123","340":"291"}"#;
    let map: HashMap<String, String> = serde_json::from_str(json).unwrap();
    let mut pairs: Vec<(u32, u32)> = map
      .into_iter()
      .filter_map(|(r, a)| Some((r.parse::<u32>().ok()?, a.parse::<u32>().ok()?)))
      .collect();
    pairs.sort_by_key(|(rank, _)| *rank);
    assert_eq!(pairs[0], (1, 344));
    assert_eq!(pairs[1], (2, 123));
    assert_eq!(pairs[2], (340, 291));
  }
}

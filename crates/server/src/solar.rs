//! 太阳活动数据代理：服务端拉取 HamQSL（无浏览器 CORS 限制），解析为 JSON 供前端同源访问。
//!
//! 结果缓存 5 分钟，避免频繁请求上游，同时降低对 HamQSL 的访问压力。

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::response::Response;
use regex::Regex;
use serde::Serialize;

use crate::cache::JsonCache;

/// HamQSL 太阳活动数据源（N0NBH 维护）。
const HAMQSL_URL: &str = "https://www.hamqsl.com/solarxml.php";
/// 服务端缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(300);
/// 上游响应体上限。
const MAX_BODY: u64 = 1 << 20;

/// 各波段传播条件。
#[derive(Serialize)]
struct BandCondition {
  band: String,
  time: String,
  condition: String,
}

/// 返回给前端的 JSON 结构。
#[derive(Serialize)]
struct SolarPayload {
  source: &'static str,
  updated: Option<String>,
  solar_flux: Option<i32>,
  a_index: Option<i32>,
  k_index: Option<i32>,
  sunspots: Option<i32>,
  xray: Option<String>,
  conditions: Vec<BandCondition>,
}

/// 提取 `<name>value</name>`。
fn tag(xml: &str, name: &str) -> Option<String> {
  let open = format!("<{name}>");
  let close = format!("</{name}>");
  let start = xml.find(&open)? + open.len();
  let end = xml[start..].find(&close)? + start;
  Some(xml[start..end].trim().to_owned())
}

fn parse_int(xml: &str, name: &str) -> Option<i32> {
  tag(xml, name).and_then(|s| s.trim().parse().ok())
}

/// 解析 `<band name=".." time="..">..</band>` 序列。
fn parse_conditions(xml: &str) -> Vec<BandCondition> {
  let re =
    Regex::new(r#"<band name="([^"]+)" time="([^"]+)">([^<]+)</band>"#).expect("valid regex");
  re.captures_iter(xml)
    .map(|c| BandCondition {
      band: c[1].to_owned(),
      time: c[2].to_owned(),
      condition: c[3].to_owned(),
    })
    .collect()
}

/// 拉取并解析 HamQSL（同步，在 `spawn_blocking` 中执行）。
fn fetch_and_parse() -> anyhow::Result<SolarPayload> {
  let mut res = crate::util::http_agent()
    .get(HAMQSL_URL)
    .call()
    .context("fetch HamQSL failed")?;
  let xml = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read HamQSL body failed")?;

  Ok(SolarPayload {
    source: "N0NBH / HamQSL",
    updated: tag(&xml, "updated"),
    solar_flux: parse_int(&xml, "solarflux"),
    a_index: parse_int(&xml, "aindex"),
    k_index: parse_int(&xml, "kindex"),
    sunspots: parse_int(&xml, "sunspots"),
    xray: tag(&xml, "xray"),
    conditions: parse_conditions(&xml),
  })
}

/// 太阳活动缓存。
pub type Cache = JsonCache;

/// `GET /api/solar` 处理器。
pub async fn handler(cache: Arc<Cache>) -> Response {
  cache
    .get(CACHE_TTL, fetch_and_parse, "solar data unavailable")
    .await
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extracts_tags() {
    let xml = "<solar><solarflux>92</solarflux><aindex> 4 </aindex><xray>B3.2</xray></solar>";
    assert_eq!(tag(xml, "solarflux").as_deref(), Some("92"));
    assert_eq!(parse_int(xml, "aindex"), Some(4));
    assert_eq!(tag(xml, "xray").as_deref(), Some("B3.2"));
    assert_eq!(tag(xml, "missing"), None);
  }

  #[test]
  fn parses_band_conditions() {
    let xml = r#"<calculatedconditions><band name="80m-40m" time="day">Fair</band><band name="20m-17m" time="night">Good</band></calculatedconditions>"#;
    let conds = parse_conditions(xml);
    assert_eq!(conds.len(), 2);
    assert_eq!(conds[0].band, "80m-40m");
    assert_eq!(conds[0].time, "day");
    assert_eq!(conds[0].condition, "Fair");
  }
}

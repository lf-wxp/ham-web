//! NOAA SWPC Kp 三日预报代理：约十天、每 3 小时一条，标明实测 / 估计 / 预报。
//!
//! 缓存 10 分钟（上游实测值每 3 小时更新、预报每天更新几次，更勤没有意义）。
//! 只做取数与校验，G 级与按天整理由前端用 `ham_web_core::kp_forecast` 算。

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, bail};
use axum::response::Response;
use ham_web_core::kp_forecast::{KpForecast, KpKind, KpPoint, clean};
use serde::Deserialize;
use tracing::warn;

use crate::cache::JsonCache;

const URL: &str = "https://services.swpc.noaa.gov/products/noaa-planetary-k-index-forecast.json";
const CACHE_TTL: Duration = Duration::from_secs(600);
/// 上游响应体上限（实际约 7 KB）。
const MAX_BODY: u64 = 1 << 20;

/// 上游单条记录；`noaa_scale` 只在实测行里有，我们自己按 Kp 算，所以不读。
#[derive(Deserialize)]
struct Raw {
  time_tag: String,
  kp: f64,
  observed: String,
}

/// 解析并校验上游文本。
///
/// 认不得的 `observed` 取值、越界的 Kp、不是 3 小时时段起点的时间都丢掉并记一条警告
/// （上游改了格式时能在日志里看到，而不是页面悄悄少几格）；**一条可用的都没有就报错**，
/// 让缓存层回退到上一份好数据，而不是把空数组当成「预报为空」缓存十分钟。
fn parse(body: &str) -> anyhow::Result<KpForecast> {
  let raw: Vec<Raw> = serde_json::from_str(body).context("parse Kp forecast failed")?;
  let total = raw.len();
  let points: Vec<KpPoint> = raw
    .into_iter()
    .filter_map(|r| {
      Some(KpPoint {
        time: r.time_tag,
        kp: r.kp,
        kind: KpKind::from_upstream(&r.observed)?,
      })
    })
    .collect();
  let (points, _) = clean(points);
  if points.is_empty() {
    bail!("Kp forecast has no usable rows ({total} rows upstream)");
  }
  if points.len() < total {
    warn!(
      kept = points.len(),
      total, "Kp forecast: some upstream rows were dropped"
    );
  }
  Ok(KpForecast {
    source: "NOAA SWPC".to_owned(),
    updated: points
      .iter()
      .rev()
      .find(|p| p.kind == KpKind::Observed)
      .map(|p| p.time.clone()),
    points,
  })
}

fn fetch_and_parse() -> anyhow::Result<KpForecast> {
  let mut res = crate::util::http_agent()
    .get(URL)
    .call()
    .context("fetch Kp forecast failed")?;
  let body = res
    .body_mut()
    .with_config()
    .limit(MAX_BODY)
    .read_to_string()
    .context("read Kp forecast body failed")?;
  parse(&body)
}

/// Kp 预报缓存。
pub type Cache = JsonCache;

/// `GET /api/kp-forecast` 处理器。
pub async fn handler(cache: Arc<Cache>) -> Response {
  cache
    .get(CACHE_TTL, fetch_and_parse, "Kp forecast unavailable")
    .await
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 2026-10-15 取回的上游原文里的几行（字段与取值一行不改）：含带 `noaa_scale` 的实测行、
  /// 估计行与预报行。
  const REAL: &str = r#"[
    {"time_tag":"2026-10-10T15:00:00","kp":7.0,"observed":"observed","noaa_scale":"G3"},
    {"time_tag":"2026-10-10T21:00:00","kp":5.33,"observed":"observed","noaa_scale":"G1"},
    {"time_tag":"2026-10-15T06:00:00","kp":1.33,"observed":"observed","noaa_scale":null},
    {"time_tag":"2026-10-15T09:00:00","kp":2.33,"observed":"estimated","noaa_scale":null},
    {"time_tag":"2026-10-16T00:00:00","kp":2.33,"observed":"predicted","noaa_scale":null}
  ]"#;

  #[test]
  fn parses_the_real_upstream_shape() {
    let f = parse(REAL).unwrap();
    assert_eq!(f.source, "NOAA SWPC");
    assert_eq!(f.points.len(), 5);
    // `updated` 是最近一条**实测**，不是最后一条（估计 / 预报都不算）。
    assert_eq!(f.updated.as_deref(), Some("2026-10-15T06:00:00"));
    let kinds: Vec<_> = f.points.iter().map(|p| p.kind).collect();
    assert_eq!(
      kinds,
      [
        KpKind::Observed,
        KpKind::Observed,
        KpKind::Observed,
        KpKind::Estimated,
        KpKind::Predicted
      ]
    );
    assert_eq!((f.points[0].kp, f.points[1].kp), (7.0, 5.33));
  }

  #[test]
  fn drops_what_it_cannot_trust_but_keeps_the_rest() {
    let body = r#"[
      {"time_tag":"2026-10-15T00:00:00","kp":1.0,"observed":"observed","noaa_scale":null},
      {"time_tag":"2026-10-15T03:00:00","kp":2.0,"observed":"forecast","noaa_scale":null},
      {"time_tag":"2026-10-15T06:00:00","kp":42.0,"observed":"observed","noaa_scale":null},
      {"time_tag":"2026-10-15T07:00:00","kp":2.0,"observed":"predicted","noaa_scale":null},
      {"time_tag":"2026-10-15T09:00:00","kp":3.0,"observed":"predicted","noaa_scale":null}
    ]"#;
    let f = parse(body).unwrap();
    let times: Vec<_> = f.points.iter().map(|p| p.time.as_str()).collect();
    assert_eq!(times, ["2026-10-15T00:00:00", "2026-10-15T09:00:00"]);
  }

  #[test]
  fn nothing_usable_is_an_error_so_the_cache_falls_back_to_the_last_good_copy() {
    assert!(parse("[]").is_err());
    assert!(parse("not json").is_err());
    assert!(parse(r#"{"error":"maintenance"}"#).is_err());
    // 字段缺失 / 类型不对：整份解析失败，同样不当成「空预报」。
    assert!(parse(r#"[{"time_tag":"2026-10-15T00:00:00","observed":"observed"}]"#).is_err());
    assert!(
      parse(r#"[{"time_tag":"2026-10-15T00:00:00","kp":"2.0","observed":"observed"}]"#).is_err()
    );
    // 行都在，但没有一行可用。
    assert!(
      parse(r#"[{"time_tag":"2026-10-15T00:00:00","kp":1.0,"observed":"mystery"}]"#).is_err()
    );
  }

  #[test]
  fn updated_is_none_when_no_row_is_observed() {
    let body = r#"[{"time_tag":"2026-10-16T00:00:00","kp":1.0,"observed":"predicted"}]"#;
    assert_eq!(parse(body).unwrap().updated, None);
  }

  #[test]
  fn a_duplicated_slot_keeps_the_later_row_and_the_output_is_sorted() {
    let body = r#"[
      {"time_tag":"2026-10-15T03:00:00","kp":2.0,"observed":"predicted"},
      {"time_tag":"2026-10-15T00:00:00","kp":1.0,"observed":"observed"},
      {"time_tag":"2026-10-15T03:00:00","kp":3.0,"observed":"estimated"}
    ]"#;
    let f = parse(body).unwrap();
    let got: Vec<_> = f.points.iter().map(|p| (p.time.as_str(), p.kp)).collect();
    assert_eq!(
      got,
      [("2026-10-15T00:00:00", 1.0), ("2026-10-15T03:00:00", 3.0)]
    );
  }
}

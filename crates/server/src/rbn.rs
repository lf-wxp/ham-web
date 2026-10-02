//! RBN（Reverse Beacon Network）代理：连接官方 telnet 流，抓取指定呼号被 CW/RTTY 信标台
//! 收到的实时报告。RBN 无稳定的 HTTP 历史查询接口（`dxsd1.php` 已改为 HTML 页面），
//! 故此处用官方 telnet 接口（telnet.reversebeacon.net:7000）短暂监听并过滤。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::error;

use crate::cache::Inflight;

/// RBN telnet 端点。
const HOST: &str = "telnet.reversebeacon.net:7000";
/// 单次监听时长。
const READ_TIMEOUT: Duration = Duration::from_secs(4);
/// 缓存时长。
const CACHE_TTL: Duration = Duration::from_secs(30);
/// 缓存条目上限。
const MAX_ENTRIES: usize = 1000;
/// 单次最多返回的 spot 数。
const MAX_SPOTS: usize = 100;

/// 查询参数。
#[derive(Deserialize)]
pub struct RbnQuery {
  /// 被 spot 的呼号（我自己的呼号，查询谁收到了我）。
  callsign: String,
}

/// 一条信标报告。
#[derive(Serialize, Clone)]
struct RbnSpot {
  spotter: String,
  freq_khz: f64,
  band: String,
  mode: String,
  snr: i32,
  wpm: u32,
}

/// 解析一条 DX cluster / RBN spot：`DX de K1TTT-#:  14025.0  W1AW  CW 15 dB 25 wpm CQ`。
fn parse_spot(line: &str, target: &str) -> Option<RbnSpot> {
  let rest = line.trim().strip_prefix("DX de ")?;
  let (spotter, rest) = rest.split_once(':')?;
  let spotter = spotter.trim().trim_end_matches("-#").to_owned();
  let mut fields = rest.split_whitespace();
  let freq_khz = fields.next()?.parse::<f64>().ok()?;
  let dx = fields.next()?;
  if !dx.eq_ignore_ascii_case(target) {
    return None;
  }
  let mode = fields.next().unwrap_or("CW").to_owned();
  let snr = fields
    .next()
    .and_then(|s| s.parse::<i32>().ok())
    .unwrap_or(0);
  // 跳过 "dB"，取 "NN wpm"。
  let _db = fields.next();
  let wpm = fields
    .next()
    .and_then(|s| s.parse::<u32>().ok())
    .unwrap_or(0);
  Some(RbnSpot {
    spotter,
    freq_khz,
    band: ham_web_core::frequencies::band_of(freq_khz / 1000.0).to_owned(),
    mode,
    snr,
    wpm,
  })
}

/// 连接 telnet 流，读取若干秒，过滤目标呼号。
async fn fetch_spots(target: &str) -> anyhow::Result<Vec<RbnSpot>> {
  let stream = tokio::net::TcpStream::connect(HOST)
    .await
    .context("connect rbn failed")?;
  let reader = BufReader::new(stream);
  let mut lines = reader.lines();
  let mut spots = Vec::new();
  let deadline = tokio::time::Instant::now() + READ_TIMEOUT;
  loop {
    if spots.len() >= MAX_SPOTS {
      break;
    }
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    if remaining.is_zero() {
      break;
    }
    match tokio::time::timeout(remaining, lines.next_line()).await {
      Ok(Ok(Some(line))) => {
        if let Some(s) = parse_spot(&line, target) {
          spots.push(s);
        }
      }
      _ => break,
    }
  }
  Ok(spots)
}

/// 缓存。
#[derive(Default)]
pub struct Cache {
  inner: Mutex<HashMap<String, (Instant, Vec<RbnSpot>)>>,
  inflight: Inflight,
}

/// `GET /api/rbn` 处理器。
pub async fn handler(Query(q): Query<RbnQuery>, cache: Arc<Cache>) -> Response {
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
      // `fetch_spots` 本身是异步的（tokio::net::TcpStream），直接 await 即可，
      // 无需 spawn_blocking + block_on（那会白白占用 blocking 线程池线程）。
      let fetched = fetch_spots(&call).await;

      // `_guard` 存活到缓存写入完成后释放，保证等待者被唤醒时能读到新缓存。
      return match fetched {
        Ok(r) => {
          if let Ok(mut g) = cache.inner.lock() {
            crate::cache::evict_oldest(&mut *g, MAX_ENTRIES);
            g.insert(call, (Instant::now(), r.clone()));
          }
          json(&r)
        }
        Err(e) => {
          error!("fetch rbn failed: {e:#}");
          (StatusCode::SERVICE_UNAVAILABLE, "rbn unavailable").into_response()
        }
      };
    }

    cache.inflight.wait().await;
  }
}

fn json(r: &[RbnSpot]) -> Response {
  #[derive(Serialize)]
  struct Payload<'a> {
    spots: &'a [RbnSpot],
  }
  match serde_json::to_string(&Payload { spots: r }) {
    Ok(j) => ([(CONTENT_TYPE, "application/json")], j).into_response(),
    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "serialize failed").into_response(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_rbn_spot() {
    let line = "DX de K1TTT-#:  14025.0  W1AW         CW 15 dB  25 wpm  CQ";
    let s = parse_spot(line, "W1AW").expect("should parse");
    assert_eq!(s.spotter, "K1TTT");
    assert_eq!(s.freq_khz, 14025.0);
    assert_eq!(s.mode, "CW");
    assert_eq!(s.snr, 15);
    assert_eq!(s.wpm, 25);
    assert_eq!(s.band, "20m");
  }

  #[test]
  fn filters_other_callsigns() {
    let line = "DX de K1TTT-#:  14025.0  W1AW         CW 15 dB  25 wpm  CQ";
    assert!(parse_spot(line, "JA1X").is_none());
  }
}

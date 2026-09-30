//! 通用 JSON 响应缓存：TTL + 单飞（singleflight）+ stale-while-revalidate + 错误日志。
//!
//! 缓存过期时只有一个请求回源刷新，其余并发请求**等待**刷新完成后读取新缓存，
//! 既避免「缓存击穿」对上游（HamQSL / DXWatch / NOAA 等）形成请求风暴，
//! 也不会让并发请求在冷启动 / 缓存过期瞬间拿到无谓的 503。

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tokio::sync::Notify;
use tracing::error;

/// 构造 `application/json` 响应。
pub fn json_ok(body: String) -> Response {
  ([(CONTENT_TYPE, "application/json")], body).into_response()
}

/// 构造纯文本错误响应。
pub fn json_err(status: StatusCode, msg: &'static str) -> Response {
  (status, msg).into_response()
}

/// 带单飞与 stale 兜底的 JSON 字符串缓存。
#[derive(Default)]
pub struct JsonCache {
  inner: Mutex<Option<(Instant, String)>>,
  inflight: AtomicBool,
  notify: Notify,
}

impl JsonCache {
  /// 命中新鲜缓存直接返回；否则用 `fetch` 拉取（`spawn_blocking`）后序列化并缓存。
  ///
  /// 并发请求在回源期间会等待（`notify`）而非立即失败；若回源失败，
  /// 等待者醒来后会抢锁自行重试一次。
  pub async fn get<T, F>(&self, ttl: Duration, fetch: F, unavailable: &'static str) -> Response
  where
    T: Serialize + Send + 'static,
    F: Fn() -> anyhow::Result<T> + Send + Sync + Clone + 'static,
  {
    loop {
      if let Some(json) = self.fresh_json(ttl) {
        return json_ok(json);
      }

      if self
        .inflight
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
      {
        let result = tokio::task::spawn_blocking(fetch.clone()).await;
        self.inflight.store(false, Ordering::Release);
        self.notify.notify_waiters();

        return match result {
          Ok(Ok(payload)) => match serde_json::to_string(&payload) {
            Ok(json) => {
              if let Ok(mut g) = self.inner.lock() {
                *g = Some((Instant::now(), json.clone()));
              }
              json_ok(json)
            }
            Err(e) => {
              error!("serialize payload failed: {e:#}");
              json_err(StatusCode::INTERNAL_SERVER_ERROR, "serialize failed")
            }
          },
          Ok(Err(e)) => {
            error!("upstream fetch failed: {e:#}");
            json_err(StatusCode::SERVICE_UNAVAILABLE, unavailable)
          }
          Err(e) => {
            error!("spawn_blocking failed: {e:#}");
            json_err(StatusCode::SERVICE_UNAVAILABLE, unavailable)
          }
        };
      }

      // 已有请求在刷新：等待通知后重试（读取新缓存，或抢锁自行回源）。
      self.notify.notified().await;
    }
  }

  /// 返回仍在 TTL 内的缓存值。
  fn fresh_json(&self, ttl: Duration) -> Option<String> {
    self
      .inner
      .lock()
      .ok()
      .and_then(|g| g.clone())
      .filter(|(t, _)| t.elapsed() < ttl)
      .map(|(_, json)| json)
  }
}

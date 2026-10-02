//! 通用 JSON 响应缓存：TTL + 单飞（singleflight）+ 上游故障时 stale 兜底 + 错误日志。
//!
//! 缓存过期时只有一个请求回源刷新，其余并发请求**等待**刷新完成后读取新缓存，
//! 既避免「缓存击穿」对上游（HamQSL / DXWatch / NOAA 等）形成请求风暴，
//! 也不会让并发请求在冷启动 / 缓存过期瞬间拿到无谓的 503。
//!
//! 回源失败（上游故障 / 网络异常）时，若有历史缓存则返回该 stale 数据并标记
//! `X-Cache: stale`，避免实时数据页在上游抖动时整页「暂不可用」；下次请求仍会
//! 重新尝试回源（stale-while-revalidate 语义）。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tokio::sync::watch;
use tracing::error;

/// 构造 `application/json` 响应。
pub fn json_ok(body: String) -> Response {
  ([(CONTENT_TYPE, "application/json")], body).into_response()
}

/// 构造纯文本错误响应。
pub fn json_err(status: StatusCode, msg: &'static str) -> Response {
  (status, msg).into_response()
}

/// 单飞（singleflight）保护：确保同一时刻只有一个请求回源刷新缓存，其余并发请求等待。
///
/// `acquire` 返回的守卫通过 `Drop` 释放，保证即使请求 future 被取消（例如客户端
/// 断开连接导致 handler 被丢弃）也不会留下「永久 in-flight」状态，从而避免后续所有
/// 请求永久阻塞在 `wait` 上。
///
/// 等待语义用 `watch` 广播（而非 `Notify::notify_waiters`）：`watch` 的订阅者能看到
/// 「订阅时刻之后」的所有更新。由于每个等待者在 `subscribe` 时都会先取到当前值，
/// 即使刷新方在等待者订阅之前就已完成（`done == true`），`wait_for` 也会立即返回，
/// 从根本上消除「等待者注册晚于通知发出」导致的丢失唤醒竞态。
pub struct Inflight {
  inflight: AtomicBool,
  done_tx: watch::Sender<bool>,
  /// 锚点订阅者：保证 `done_tx.send` 始终有接收方（`watch` 无订阅者时 `send` 会失败）。
  _done_rx: watch::Receiver<bool>,
}

impl Default for Inflight {
  fn default() -> Self {
    let (done_tx, _done_rx) = watch::channel(false);
    Self {
      inflight: AtomicBool::new(false),
      done_tx,
      _done_rx,
    }
  }
}

impl Inflight {
  /// 尝试抢占刷新权；成功返回守卫（`Drop` 时自动释放并唤醒等待者），失败返回 `None`。
  pub fn acquire(&self) -> Option<InflightGuard<'_>> {
    if self
      .inflight
      .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
      .is_ok()
    {
      // 新一轮刷新开始：重置完成标记，让后续等待者从本轮开始监听。
      let _ = self.done_tx.send(false);
      Some(InflightGuard { inflight: self })
    } else {
      None
    }
  }

  /// 等待刷新完成（由守卫 `Drop` 时的 `send(true)` 唤醒）。
  pub async fn wait(&self) {
    let mut rx = self.done_tx.subscribe();
    let _ = rx.wait_for(|&done| done).await;
  }
}

/// 持有刷新权的守卫；`Drop` 时释放并唤醒等待者。
pub struct InflightGuard<'a> {
  inflight: &'a Inflight,
}

impl Drop for InflightGuard<'_> {
  fn drop(&mut self) {
    self.inflight.inflight.store(false, Ordering::Release);
    let _ = self.inflight.done_tx.send(true);
  }
}

/// 带单飞与 stale 兜底的 JSON 字符串缓存。
#[derive(Default)]
pub struct JsonCache {
  inner: Mutex<Option<(Instant, String)>>,
  inflight: Inflight,
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

      if let Some(_guard) = self.inflight.acquire() {
        let result = tokio::task::spawn_blocking(fetch.clone()).await;

        // 注意：`_guard` 需存活到缓存写入完成之后再释放，保证等待者被唤醒时能读到
        // 新缓存而非旧缓存。因此这里不提前 drop，让它随函数返回时自然释放。
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
            self.stale_or_unavailable(unavailable)
          }
          Err(e) => {
            error!("spawn_blocking failed: {e:#}");
            self.stale_or_unavailable(unavailable)
          }
        };
      }

      // 已有请求在刷新：等待通知后重试（读取新缓存，或抢锁自行回源）。
      self.inflight.wait().await;
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

  /// 返回最近一次成功回源的缓存值（不限 TTL），用于上游故障时的 stale 兜底。
  fn stale_json(&self) -> Option<String> {
    self
      .inner
      .lock()
      .ok()
      .and_then(|g| g.clone())
      .map(|(_, json)| json)
  }

  /// 上游拉取失败时：有历史缓存则返回 stale 数据（标记 `X-Cache: stale`），否则 503。
  fn stale_or_unavailable(&self, unavailable: &'static str) -> Response {
    if let Some(json) = self.stale_json() {
      let mut res = json_ok(json);
      res.headers_mut().insert(
        axum::http::header::HeaderName::from_static("x-cache"),
        axum::http::HeaderValue::from_static("stale"),
      );
      res
    } else {
      json_err(StatusCode::SERVICE_UNAVAILABLE, unavailable)
    }
  }
}

/// 缓存条目数达到 `max` 时，淘汰最旧的一半条目（而非整表清空）。
///
/// 整表清空会在缓存上限附近引发「清空 → 瞬时击穿上游」的抖动；保留较新的半区
/// 能显著平滑这一过程。`value` 中的 `Instant` 即插入时间，用于判定新旧。
pub fn evict_oldest<K: Eq + std::hash::Hash, T>(map: &mut HashMap<K, (Instant, T)>, max: usize) {
  if map.len() < max {
    return;
  }
  let mut entries: Vec<(K, (Instant, T))> = map.drain().collect();
  entries.sort_by_key(|(_, (t, _))| *t);
  let keep = entries.len() / 2;
  map.extend(entries.into_iter().skip(keep));
}

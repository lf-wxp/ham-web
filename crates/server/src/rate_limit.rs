//! 基于客户端身份（IP 或 API key）的固定窗口限流：防 `/api/*` 代理接口被滥用
//! （耗尽上游配额 / 拖垮服务）。
//!
//! IP 识别：直连场景取 TCP 连接的 [`ConnectInfo`]（`X-Forwarded-For` 由客户端伪造，
//! 不能信）；连接地址落在回环 / 私有段（典型反代部署，对端是代理）时才采用
//! `X-Forwarded-For` / `X-Real-IP` 的**首个**条目（nginx `proxy_set_header` 的覆盖语义）。
//! 两者都缺失时放行（如仅提供 /api 代理的开发联调模式）。

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request};

/// 分桶身份：按 IP 或按任意字符串（API key）。
///
/// key 桶的存在理由：反代部署下 `ConnectInfo` 是代理地址，所有 keyed 请求若按 IP 分桶
/// 会挤在同一个桶里 —— 一个客户端打满配额，别的 key 跟着被拒。
#[derive(Clone, Hash, PartialEq, Eq)]
enum Bucket {
  Ip(IpAddr),
  Key(String),
}

/// 固定窗口限流器：窗口内请求数超过 `limit` 即拒绝（429）。
#[derive(Clone)]
pub struct RateLimiter {
  inner: Arc<Mutex<HashMap<Bucket, Vec<Instant>>>>,
  limit: usize,
  window: Duration,
}

impl RateLimiter {
  pub fn new(limit: usize, window: Duration) -> Self {
    Self {
      inner: Arc::new(Mutex::new(HashMap::new())),
      limit,
      window,
    }
  }

  /// 临界区里 panic 会毒化锁；限流器不是账本，沿用里面那份数据好过让之后的每个请求都 panic。
  fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Bucket, Vec<Instant>>> {
    self.inner.lock().unwrap_or_else(|e| e.into_inner())
  }

  /// 条目过多时全量清理过期时间戳与空条目，避免大量不同来源扫描导致内存膨胀。
  fn shrink_if_large(&self, map: &mut HashMap<Bucket, Vec<Instant>>, now: Instant) {
    if map.len() >= 10_000 {
      map.retain(|_, w| {
        w.retain(|t| now.duration_since(*t) < self.window);
        !w.is_empty()
      });
    }
  }

  fn allow_bucket(&self, bucket: Bucket) -> bool {
    let now = Instant::now();
    let mut map = self.lock();
    self.shrink_if_large(&mut map, now);
    let entry = map.entry(bucket).or_default();
    entry.retain(|t| now.duration_since(*t) < self.window);
    if entry.len() >= self.limit {
      return false;
    }
    entry.push(now);
    true
  }

  fn allow_ip(&self, ip: IpAddr) -> bool {
    self.allow_bucket(Bucket::Ip(ip))
  }

  /// 按字符串身份分桶：不同 key 各自独立计数。
  pub fn allow_by_key(&self, key: &str) -> bool {
    self.allow_bucket(Bucket::Key(key.to_owned()))
  }

  // 下面三个是「只数失败」的用法（登录猜口令）：先 `is_blocked_by_key` 看一眼，
  // 失败了 `record_by_key`，成功了 `clear_by_key`。与 `allow_by_key`「每次请求都计一次」不同，
  // 正确的请求不消耗额度，别人也无法靠反复访问把本人挤出额度。

  /// 该身份在窗口内的记录是否已达上限（只看不记）。
  pub fn is_blocked_by_key(&self, key: &str) -> bool {
    let now = Instant::now();
    let mut map = self.lock();
    let bucket = Bucket::Key(key.to_owned());
    let Some(window) = map.get_mut(&bucket) else {
      return false;
    };
    window.retain(|t| now.duration_since(*t) < self.window);
    if window.is_empty() {
      map.remove(&bucket);
      return false;
    }
    window.len() >= self.limit
  }

  /// 给该身份记一次（不检查上限）。
  pub fn record_by_key(&self, key: &str) {
    let now = Instant::now();
    let mut map = self.lock();
    self.shrink_if_large(&mut map, now);
    let entry = map.entry(Bucket::Key(key.to_owned())).or_default();
    entry.retain(|t| now.duration_since(*t) < self.window);
    entry.push(now);
  }

  /// 清掉该身份的记录。
  pub fn clear_by_key(&self, key: &str) {
    self.lock().remove(&Bucket::Key(key.to_owned()));
  }
}

/// `X-Forwarded-For` / `X-Real-IP` 的首个地址（代理覆盖写入的客户端真实 IP）。
fn forwarded_ip(req: &Request) -> Option<IpAddr> {
  for header in ["x-forwarded-for", "x-real-ip"] {
    if let Some(v) = req.headers().get(header).and_then(|v| v.to_str().ok())
      && let Some(first) = v.split(',').next().map(str::trim).filter(|s| !s.is_empty())
      && let Ok(ip) = first.parse::<IpAddr>()
    {
      return Some(ip);
    }
  }
  None
}

/// 是否落在回环 / 私有 / 链路本地段：这些段里的对端只可能是本机进程或内网代理。
fn is_local_or_private(ip: IpAddr) -> bool {
  match ip {
    IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
    IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
  }
}

/// 从连接信息或转发头解析客户端 IP。
pub fn client_ip(conn: Option<ConnectInfo<SocketAddr>>, req: &Request) -> Option<IpAddr> {
  let forwarded = forwarded_ip(req);
  match conn {
    // 直连公网客户端：TCP 对端就是真相，XFF 可能是伪造的，不采用。
    Some(ConnectInfo(addr)) if !is_local_or_private(addr.ip()) => Some(addr.ip()),
    // 对端是回环/内网（反代或本机联调）：真实客户端在转发头里；没有转发头才退回对端。
    Some(ConnectInfo(addr)) => forwarded.or(Some(addr.ip())),
    None => forwarded,
  }
}

/// 判断本次请求是否放行；无法识别 IP 时放行（避免误伤）。
pub fn allow(limiter: &RateLimiter, conn: Option<ConnectInfo<SocketAddr>>, req: &Request) -> bool {
  match client_ip(conn, req) {
    Some(ip) => limiter.allow_ip(ip),
    None => true,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn counting_only_failures_never_penalises_a_clean_key() {
    let l = RateLimiter::new(3, Duration::from_secs(60));
    // 只看不记：反复看也不会把额度耗掉。
    for _ in 0..10 {
      assert!(!l.is_blocked_by_key("alice"));
    }
    for _ in 0..3 {
      assert!(!l.is_blocked_by_key("alice"));
      l.record_by_key("alice");
    }
    assert!(l.is_blocked_by_key("alice"));
    // 别的身份不受牵连。
    assert!(!l.is_blocked_by_key("bob"));
  }

  #[test]
  fn clearing_a_key_lifts_the_block() {
    let l = RateLimiter::new(1, Duration::from_secs(60));
    l.record_by_key("alice");
    assert!(l.is_blocked_by_key("alice"));
    l.clear_by_key("alice");
    assert!(!l.is_blocked_by_key("alice"));
  }

  #[test]
  fn records_expire_with_the_window() {
    let l = RateLimiter::new(1, Duration::from_millis(30));
    l.record_by_key("alice");
    assert!(l.is_blocked_by_key("alice"));
    std::thread::sleep(Duration::from_millis(60));
    assert!(!l.is_blocked_by_key("alice"));
  }
}

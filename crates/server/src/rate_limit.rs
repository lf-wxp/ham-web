//! 基于客户端 IP 的固定窗口限流：防 `/api/*` 代理接口被滥用（耗尽上游配额 / 拖垮服务）。
//!
//! 客户端 IP 优先取 TCP 连接的 [`ConnectInfo`]，反代场景回退到 `X-Forwarded-For` /
//! `X-Real-IP` 的首个地址；两者都缺失时放行（如仅提供 /api 代理的开发联调模式）。

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request};

/// 固定窗口限流器：窗口内请求数超过 `limit` 即拒绝（429）。
#[derive(Clone)]
pub struct RateLimiter {
  inner: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
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

  fn allow_ip(&self, ip: IpAddr) -> bool {
    let now = Instant::now();
    let mut map = self.inner.lock().expect("rate limiter poisoned");
    // 条目过多时全量清理过期时间戳与空条目，避免大量不同 IP 扫描导致内存膨胀。
    if map.len() >= 10_000 {
      map.retain(|_, w| {
        w.retain(|t| now.duration_since(*t) < self.window);
        !w.is_empty()
      });
    }
    let entry = map.entry(ip).or_default();
    entry.retain(|t| now.duration_since(*t) < self.window);
    if entry.len() >= self.limit {
      return false;
    }
    entry.push(now);
    true
  }
}

/// 从连接信息或转发头解析客户端 IP。
fn client_ip(conn: Option<ConnectInfo<SocketAddr>>, req: &Request) -> Option<IpAddr> {
  if let Some(ConnectInfo(addr)) = conn {
    return Some(addr.ip());
  }
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

/// 判断本次请求是否放行；无法识别 IP 时放行（避免误伤）。
pub fn allow(limiter: &RateLimiter, conn: Option<ConnectInfo<SocketAddr>>, req: &Request) -> bool {
  match client_ip(conn, req) {
    Some(ip) => limiter.allow_ip(ip),
    None => true,
  }
}

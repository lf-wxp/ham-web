//! 通用小工具：查询参数 percent-encode、共享 HTTP Agent（带超时）。

use std::sync::OnceLock;
use std::time::Duration;

/// 建立连接（含 DNS 解析与 TLS 握手）的超时。
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// 单次请求的端到端超时：从 DNS 解析到读完响应体，覆盖其余所有超时。
const DEFAULT_GLOBAL_TIMEOUT: Duration = Duration::from_secs(15);

/// 出网上游请求共用的 `ureq::Agent`。
///
/// 直接调用 `ureq::get(..)` 使用的是不带任何超时的默认 agent：上游一旦挂起，
/// 该请求会一直占住一个 `spawn_blocking` 线程（默认池 512），并发稍多就会耗尽
/// 阻塞线程池，使**所有** `/api/*` 一起不可用。这里统一配置超时以避免级联故障。
///
/// 超时可用环境变量覆盖：`UPSTREAM_CONNECT_TIMEOUT_SECS` / `UPSTREAM_TIMEOUT_SECS`。
pub fn http_agent() -> &'static ureq::Agent {
  static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
  AGENT.get_or_init(|| {
    let connect = env_duration("UPSTREAM_CONNECT_TIMEOUT_SECS", DEFAULT_CONNECT_TIMEOUT);
    let global = env_duration("UPSTREAM_TIMEOUT_SECS", DEFAULT_GLOBAL_TIMEOUT);
    let config = ureq::Agent::config_builder()
      .timeout_connect(Some(connect))
      .timeout_global(Some(global))
      .build();
    ureq::Agent::from(config)
  })
}

/// 读取秒数环境变量，缺失或非法时回退到默认值（非法值不静默当作 0，避免「零超时」）。
fn env_duration(key: &str, default: Duration) -> Duration {
  std::env::var(key)
    .ok()
    .and_then(|s| s.parse::<u64>().ok())
    .filter(|&s| s > 0)
    .map_or(default, Duration::from_secs)
}

/// 对查询参数值做 percent-encode（`application/x-www-form-urlencoded`，空格编码为 `+`）。
///
/// 用于拼接外部上游 API 的查询串，避免参数值中的空格、`&`、`=` 等字符破坏 URL 结构。
pub fn urlencode_query(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for b in s.bytes() {
    match b {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
        out.push(char::from(b));
      }
      b' ' => out.push('+'),
      _ => {
        use std::fmt::Write;
        let _ = write!(out, "%{b:02X}");
      }
    }
  }
  out
}

/// 校验外部查询 token 只含 ASCII 字母数字与允许的额外字符。
///
/// 呼号 / 编号等参数会直接拼进上游 URL 路径或查询串，若允许 `%`、`#`、`?`、空格等
/// 字符，可能改变 URL 语义（路径穿越 / fragment 注入）。本函数用于在进入缓存与回源
/// 之前对这类参数做白名单校验。
pub fn is_safe_token(s: &str, extra: &[char]) -> bool {
  s.chars()
    .all(|c| c.is_ascii_alphanumeric() || extra.contains(&c))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn encodes_spaces_and_specials() {
    assert_eq!(urlencode_query("United States"), "United+States");
    assert_eq!(urlencode_query("a&b=c"), "a%26b%3Dc");
    assert_eq!(urlencode_query("W1AW/QRP"), "W1AW%2FQRP");
    assert_eq!(urlencode_query("abc123-_.~"), "abc123-_.~");
  }

  #[test]
  fn http_agent_is_shared_and_configured() {
    // 同一进程内多次调用应复用同一个 agent（连接池与超时配置只需构建一次）。
    let a = http_agent();
    let b = http_agent();
    assert!(std::ptr::eq(a, b), "http_agent 应返回同一个静态实例");
  }
}

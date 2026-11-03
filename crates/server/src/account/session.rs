//! 会话令牌与 Cookie。
//!
//! 令牌是 32 字节随机数（base64url）：只在登录响应的 `Set-Cookie` 里出现一次，库里存它的 SHA-256。
//! Cookie 是 `HttpOnly`（页面脚本读不到，XSS 偷不走）+ `SameSite=Strict`（跨站请求不带，
//! 同时就是 CSRF 防线；本站的 `fetch` 都是同源的，不受影响）。

use axum::http::HeaderMap;
use axum::http::header::COOKIE;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ham_web_core::account::SESSION_COOKIE;
use sha2::{Digest, Sha256};

/// 生成一个新令牌。
pub fn new_token() -> String {
  let mut raw = [0u8; 32];
  rand::fill(&mut raw);
  URL_SAFE_NO_PAD.encode(raw)
}

/// 令牌的库内形式。
pub fn hash_token(token: &str) -> [u8; 32] {
  Sha256::digest(token.as_bytes()).into()
}

/// 从请求头的 `Cookie` 里取出会话令牌。
pub fn token_from_headers(headers: &HeaderMap) -> Option<String> {
  headers
    .get_all(COOKIE)
    .iter()
    .filter_map(|v| v.to_str().ok())
    .flat_map(|line| line.split(';'))
    .filter_map(|pair| pair.trim().split_once('='))
    .find(|(name, _)| *name == SESSION_COOKIE)
    .map(|(_, value)| value.trim().to_owned())
    .filter(|v| !v.is_empty())
}

/// 登录时下发的 `Set-Cookie`。
pub fn set_cookie(token: &str, max_age_secs: i64, secure: bool) -> String {
  format!(
    "{SESSION_COOKIE}={token}; Path=/; Max-Age={max_age_secs}; HttpOnly; SameSite=Strict{}",
    if secure { "; Secure" } else { "" }
  )
}

/// 退出登录时让浏览器丢掉 Cookie。
pub fn clear_cookie(secure: bool) -> String {
  set_cookie("", 0, secure)
}

#[cfg(test)]
mod tests {
  use axum::http::HeaderValue;

  use super::*;

  #[test]
  fn tokens_are_long_unique_and_url_safe() {
    let a = new_token();
    assert_eq!(a.len(), 43);
    assert_ne!(a, new_token());
    assert!(
      a.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    );
  }

  #[test]
  fn the_stored_form_is_a_stable_digest_not_the_token() {
    assert_eq!(hash_token("abc"), hash_token("abc"));
    assert_ne!(hash_token("abc"), hash_token("abd"));
    assert_ne!(&hash_token("abc")[..], b"abc");
  }

  fn headers(cookie: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(COOKIE, HeaderValue::from_str(cookie).unwrap());
    h
  }

  #[test]
  fn the_token_is_found_among_other_cookies() {
    assert_eq!(
      token_from_headers(&headers("theme=dark; ham_session=abc123; x=y")).as_deref(),
      Some("abc123")
    );
    assert_eq!(
      token_from_headers(&headers("ham_session=abc")).as_deref(),
      Some("abc")
    );
    // 名字必须整个相等：别家的同后缀 cookie 不算。
    assert_eq!(token_from_headers(&headers("x_ham_session=evil")), None);
    assert_eq!(token_from_headers(&headers("ham_session=")), None);
    assert_eq!(token_from_headers(&HeaderMap::new()), None);
  }

  #[test]
  fn the_cookie_is_http_only_strict_and_secure_when_asked() {
    let c = set_cookie("tok", 60, false);
    assert!(c.starts_with("ham_session=tok;"));
    assert!(c.contains("HttpOnly") && c.contains("SameSite=Strict") && c.contains("Max-Age=60"));
    assert!(!c.contains("Secure"));
    assert!(set_cookie("tok", 60, true).ends_with("; Secure"));
    let cleared = clear_cookie(true);
    assert!(cleared.starts_with("ham_session=;") && cleared.contains("Max-Age=0"));
  }
}

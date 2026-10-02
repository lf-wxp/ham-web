//! 通用小工具：查询参数 percent-encode。

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
}

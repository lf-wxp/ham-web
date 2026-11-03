//! CSS `<time>` 字面量（`180ms` / `0.24s`）→ 毫秒。
//!
//! 弹层关闭后要「留在 DOM 里播完退场动画再卸载」，卸载的延迟必须等于样式表里的时长 token，
//! 否则改了 token 却忘了改 Rust 里的常量，退场就会被截断（偏小）或多留一帧空壳（偏大）。
//! 前端读 `getComputedStyle(:root).getPropertyValue("--md-dur")` 得到的就是这种字符串。

/// 解析一个 CSS `<time>`：`180ms`、`0.24s`、` .5s `（两端空白、省略前导 0 都认）。
///
/// 缺单位、负数、非数字、非有限值一律返回 `None` —— 调用方应当退回自己的兜底值，
/// 而不是把 `NaN` 变成 0 毫秒（那会让弹层「关闭即卸载」，退场动画整个消失）。
#[must_use]
pub fn parse_ms(raw: &str) -> Option<u64> {
  let text = raw.trim();
  // 先试 `ms` 再试 `s`：`180ms` 同时以 `s` 结尾。
  let (number, per_unit) = match text.strip_suffix("ms") {
    Some(n) => (n, 1.0),
    None => (text.strip_suffix('s')?, 1000.0),
  };
  let value: f64 = number.trim().parse().ok()?;
  if !value.is_finite() || value < 0.0 {
    return None;
  }
  // 上面已排除负数与非有限值；超大值饱和到 `u64::MAX`，不会回绕。
  #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
  let ms = (value * per_unit).round() as u64;
  Some(ms)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn milliseconds_and_seconds() {
    assert_eq!(parse_ms("180ms"), Some(180));
    assert_eq!(parse_ms("0.24s"), Some(240));
    assert_eq!(parse_ms("1s"), Some(1000));
    assert_eq!(parse_ms("120ms"), Some(120));
  }

  #[test]
  fn tolerates_whitespace_and_missing_leading_zero() {
    // `getPropertyValue` 返回的自定义属性值常带前导空格。
    assert_eq!(parse_ms(" 180ms"), Some(180));
    assert_eq!(parse_ms("\t.5s\n"), Some(500));
    assert_eq!(parse_ms("0.08s"), Some(80));
  }

  #[test]
  fn rounds_instead_of_truncating() {
    // 0.12s 在浮点里是 0.12000000000000001 一类的数：截断会得到 119。
    assert_eq!(parse_ms("0.12s"), Some(120));
    assert_eq!(parse_ms("0.48s"), Some(480));
    assert_eq!(parse_ms("0.5ms"), Some(1));
  }

  #[test]
  fn rejects_what_is_not_a_time() {
    // 缺单位：`180` 是数字不是时长，不能猜成毫秒。
    assert_eq!(parse_ms("180"), None);
    assert_eq!(parse_ms(""), None);
    assert_eq!(parse_ms("ms"), None);
    assert_eq!(parse_ms("s"), None);
    assert_eq!(parse_ms("fast"), None);
    assert_eq!(parse_ms("var(--md-dur)"), None);
    // 单位只认小写的 `ms` / `s`（CSS 单位大小写不敏感，但 token 里一直是小写）。
    assert_eq!(parse_ms("1m"), None);
  }

  #[test]
  fn rejects_negative_and_non_finite() {
    assert_eq!(parse_ms("-1s"), None);
    assert_eq!(parse_ms("-0.5ms"), None);
    assert_eq!(parse_ms("NaNs"), None);
    assert_eq!(parse_ms("infs"), None);
    assert_eq!(parse_ms("infinityms"), None);
  }

  #[test]
  fn huge_values_saturate() {
    assert_eq!(parse_ms("1e30s"), Some(u64::MAX));
    assert_eq!(parse_ms("0s"), Some(0));
  }
}

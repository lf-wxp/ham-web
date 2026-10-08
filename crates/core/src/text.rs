//! 与 JavaScript 语义保持一致的文本工具函数。

/// 判断字符是否属于 JavaScript 正则 `\s` 所匹配的空白字符集合。
///
/// 与 [`char::is_whitespace`] 的差异：JS 额外包含 `U+FEFF`，但不包含 `U+0085`。
/// 为了与历史数据（由 JS 生成的内容指纹）逐字节一致，这里严格复刻 JS 行为。
#[must_use]
pub const fn is_js_whitespace(c: char) -> bool {
  matches!(
    c,
    '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r' | ' ' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
      ..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}'
  )
}

/// 等价于 JS 的 `s.replace(/\s+/g, " ").trim()`。
#[must_use]
pub fn collapse_whitespace(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  let mut pending_space = false;
  for c in s.chars() {
    if is_js_whitespace(c) {
      pending_space = !out.is_empty();
    } else {
      if pending_space {
        out.push(' ');
        pending_space = false;
      }
      out.push(c);
    }
  }
  out
}

/// 等价于 JS 的 `s.trim()`。
#[must_use]
pub fn js_trim(s: &str) -> &str {
  s.trim_matches(is_js_whitespace)
}

/// 把毫秒格式化为 `mm:ss`，超过 1 小时则为 `hh:mm:ss`。
#[must_use]
pub fn format_ms(ms: i64) -> String {
  let total_sec = (ms.max(0)) / 1000;
  let h = total_sec / 3600;
  let m = (total_sec % 3600) / 60;
  let s = total_sec % 60;
  if h > 0 {
    format!("{h:02}:{m:02}:{s:02}")
  } else {
    format!("{m:02}:{s:02}")
  }
}

/// 字符是否属于要判定的 CJK 区段。
#[must_use]
pub const fn is_cjk(c: char) -> bool {
  matches!(
    c,
    '\u{3000}'..='\u{303F}'   // CJK 标点（、。「」…）
      | '\u{3400}'..='\u{4DBF}' // 扩展 A
      | '\u{4E00}'..='\u{9FFF}' // 基本区
      | '\u{F900}'..='\u{FAFF}' // 兼容汉字
      | '\u{FF00}'..='\u{FFEF}' // 全角形式（（），：；？！）
  )
}

/// 字符串里是否含 CJK 字符 —— 用来区分「语义 key」与「直接写中文原文」。
///
/// 只认基本区（`U+4E00–U+9FFF`）会漏掉三类入参：CJK 标点 / 全角符号、扩展 A 区汉字、
/// 兼容区汉字 —— 它们会被当成语义 key 原样返回，界面上直接漏出中文。
///
/// 前端（`catalog` 的中文反向索引）、构建工具（文案扫描）与知识库译文抽取三处
/// 共用这一份实现，避免各自的区段口径漂移。
#[must_use]
pub fn has_cjk(s: &str) -> bool {
  s.chars().any(is_cjk)
}

/// 逐字符转大写（每个字符只取单字符大写结果），保证字符数量不变，便于做位置对齐的高亮。
#[must_use]
pub fn upper_chars(s: &str) -> Vec<char> {
  s.chars()
    .map(|c| {
      let mut up = c.to_uppercase();
      match (up.next(), up.next()) {
        (Some(u), None) => u,
        _ => c,
      }
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn collapse_matches_js_semantics() {
    assert_eq!(collapse_whitespace("  a \n\t b\u{3000}c  "), "a b c");
    assert_eq!(collapse_whitespace(""), "");
    assert_eq!(collapse_whitespace("\u{FEFF}x"), "x");
    assert_eq!(collapse_whitespace("a\u{0085}b"), "a\u{0085}b");
  }

  #[test]
  fn cjk_detection_covers_all_the_ranges_that_matter() {
    // 语义 key 与纯拉丁文本不含 CJK。
    for s in ["common.save", "tools.nec-imported", "Class B", "", "Ω ± ×"] {
      assert!(!has_cjk(s), "{s} 不应判为含 CJK");
    }
    // 基本区、扩展 A、兼容区、CJK 标点、全角符号都要认出来（用转义写，避免字形歧义）。
    for (name, s) in [
      ("基本区", "\u{4E00}"),
      ("扩展A", "\u{3400}"),
      ("兼容区", "\u{F900}"),
      ("CJK标点", "\u{3001}"),
      ("全角形式", "\u{FF08}"),
      ("保存", "保存"),
      ("中文+key", "保存 common.save"),
    ] {
      assert!(has_cjk(s), "{name} {s} 应判为含 CJK");
      assert!(is_cjk(s.chars().next().expect("非空")), "{name}");
    }
  }

  #[test]
  fn format_ms_pads_minutes_and_hours() {
    assert_eq!(format_ms(0), "00:00");
    assert_eq!(format_ms(-5), "00:00");
    assert_eq!(format_ms(61_000), "01:01");
    assert_eq!(format_ms(3_661_000), "01:01:01");
  }
}

//! FT8 / FT4 消息用到的字符表（`charn` / `nchar`）。
//!
//! 转写自 MIT 许可的 ft8_lib `text.c`。六张字符表分别对应消息不同字段的字母表：
//! 呼号、网格、自由文本、报告各有允许的字符集。

/// 字符表种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharTable {
  /// 空格、0-9、A-Z、`+-./?`（42 个）—— 自由文本。
  Full,
  /// 空格、0-9、A-Z（37 个）—— 呼号。
  AlphanumSpace,
  /// 0-9、A-Z（36 个）。
  Alphanum,
  /// 空格、A-Z（27 个）—— 呼号后缀。
  LettersSpace,
  /// 0-9（10 个）。
  Numeric,
  /// 空格、0-9、A-Z、`/`（38 个）—— 非标准呼号。
  AlphanumSpaceSlash,
}

/// 索引 → 字符（`charn`）。
///
/// 表内索引从 0 开始；越界或无法表示的索引返回 `'_'`（与 ft8_lib 一致）。
#[must_use]
pub fn charn(mut c: usize, table: CharTable) -> char {
  if table != CharTable::Alphanum && table != CharTable::Numeric {
    if c == 0 {
      return ' ';
    }
    c -= 1;
  }
  if table != CharTable::LettersSpace {
    if c < 10 {
      return char::from(b'0' + c as u8);
    }
    c -= 10;
  }
  if table != CharTable::Numeric {
    if c < 26 {
      return char::from(b'A' + c as u8);
    }
    c -= 26;
  }
  if table == CharTable::Full {
    if c < 5 {
      return b"+-./?"[c] as char;
    }
  } else if table == CharTable::AlphanumSpaceSlash && c == 0 {
    return '/';
  }
  '_'
}

/// 字符 → 索引（`nchar`），不在表内返回 `None`。
#[must_use]
pub fn nchar(c: char, table: CharTable) -> Option<usize> {
  let mut n = 0;
  if table != CharTable::Alphanum && table != CharTable::Numeric {
    if c == ' ' {
      return Some(n);
    }
    n += 1;
  }
  if table != CharTable::LettersSpace {
    if c.is_ascii_digit() {
      return Some(n + (c as usize - '0' as usize));
    }
    n += 10;
  }
  if table != CharTable::Numeric {
    if c.is_ascii_uppercase() {
      return Some(n + (c as usize - 'A' as usize));
    }
    n += 26;
  }
  if table == CharTable::Full {
    for (i, p) in b"+-./?".iter().enumerate() {
      if c == *p as char {
        return Some(n + i);
      }
    }
  } else if table == CharTable::AlphanumSpaceSlash && c == '/' {
    return Some(n);
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn charn_nchar_roundtrip_all_tables() {
    // 每张表里每个合法字符都能往返。
    for table in [
      CharTable::Full,
      CharTable::AlphanumSpace,
      CharTable::Alphanum,
      CharTable::LettersSpace,
      CharTable::Numeric,
      CharTable::AlphanumSpaceSlash,
    ] {
      // 扫过足够大的索引范围，只要 charn 不返回 '_' 就能 nchar 回来。
      for idx in 0..64 {
        let ch = charn(idx, table);
        if ch != '_' {
          assert_eq!(nchar(ch, table), Some(idx), "{table:?} idx={idx} ch={ch}");
        }
      }
    }
  }

  #[test]
  fn charn_matches_ft8_lib_layout() {
    // 关键索引：FULL 表的空格=0、数字 1..=10、字母 11..=36、标点 37..=41。
    assert_eq!(charn(0, CharTable::Full), ' ');
    assert_eq!(charn(1, CharTable::Full), '0');
    assert_eq!(charn(10, CharTable::Full), '9');
    assert_eq!(charn(11, CharTable::Full), 'A');
    assert_eq!(charn(36, CharTable::Full), 'Z');
    assert_eq!(charn(37, CharTable::Full), '+');
    assert_eq!(charn(41, CharTable::Full), '?');
    // ALPHANUM_SPACE：空格=0、0=1、A=11、Z=36。
    assert_eq!(charn(0, CharTable::AlphanumSpace), ' ');
    assert_eq!(charn(1, CharTable::AlphanumSpace), '0');
    assert_eq!(charn(11, CharTable::AlphanumSpace), 'A');
    // ALPHANUM_SPACE_SLASH：空格=0、0=1、A=11、/=37。
    assert_eq!(charn(37, CharTable::AlphanumSpaceSlash), '/');
    assert_eq!(nchar('/', CharTable::AlphanumSpaceSlash), Some(37));
    // 不可表示返回 '_'。
    assert_eq!(charn(99, CharTable::Numeric), '_');
    assert_eq!(nchar('!', CharTable::Full), None);
  }
}

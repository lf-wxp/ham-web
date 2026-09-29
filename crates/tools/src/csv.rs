//! 极简 CSV 解析器：支持双引号字段、`""` 转义、字段内换行，忽略 `\r`。
//!
//! 行为与旧版 Node.js 数据集脚本中的解析器逐字符一致。

use std::collections::HashMap;

/// 解析 CSV 文本为行列表。
pub fn parse(text: &str) -> Vec<Vec<String>> {
  let mut rows = Vec::new();
  let mut current = Vec::new();
  let mut field = String::new();
  let mut in_quotes = false;
  let mut chars = text.chars().peekable();

  while let Some(ch) = chars.next() {
    if in_quotes {
      if ch == '"' {
        if chars.peek() == Some(&'"') {
          field.push('"');
          chars.next();
        } else {
          in_quotes = false;
        }
      } else {
        field.push(ch);
      }
      continue;
    }
    match ch {
      '"' => in_quotes = true,
      ',' => current.push(std::mem::take(&mut field)),
      '\n' => {
        current.push(std::mem::take(&mut field));
        rows.push(std::mem::take(&mut current));
      }
      '\r' => {}
      _ => field.push(ch),
    }
  }
  current.push(field);
  rows.push(current);
  rows
}

/// 带表头的 CSV 表。
pub struct Table {
  header: HashMap<String, usize>,
  pub rows: Vec<Vec<String>>,
}

impl Table {
  pub fn parse(text: &str) -> Self {
    let mut rows = parse(text);
    let header_row = if rows.is_empty() {
      Vec::new()
    } else {
      rows.remove(0)
    };
    let header = header_row
      .iter()
      .enumerate()
      .map(|(i, h)| (h.trim().to_owned(), i))
      .collect();
    Self { header, rows }
  }

  /// 列下标。
  pub fn col(&self, name: &str) -> Option<usize> {
    self.header.get(name).copied()
  }

  /// 取某行某列（trim 后），缺失返回空串。
  pub fn get(row: &[String], col: Option<usize>) -> &str {
    col.and_then(|i| row.get(i)).map_or("", |s| s.trim())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_quotes_and_newlines() {
    let rows = parse("a,b\r\n\"x,1\",\"he said \"\"hi\"\"\nnext\"\n");
    assert_eq!(rows[0], vec!["a", "b"]);
    assert_eq!(rows[1], vec!["x,1", "he said \"hi\"\nnext"]);
    assert_eq!(rows[2], vec![""]);
  }
}

//! 从 lrpt-encoder 的 `compression.rs` 提取 DC/AC Huffman 表，生成 Rust 表
//! （`src/huffman_tables.rs`）。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-lrpt --example extract_huffman -- /path/to/compression.rs [输出 .rs]
//! ```
//! 仓库内已有生成产物 `src/huffman_tables.rs`，上游更新时重跑。
//!
//! 来源：lrpt-encoder（MIT License）`src/compression.rs` —— 与 SatDump MSU-MR
//! 解码器一致的 LRPT JPEG 亮度 Huffman 表（小值即 JPEG Annex K 标准表）。
//! 提取逻辑无正则依赖（括号配对 + 逐行解析）。

use std::fs;
use std::path::PathBuf;

fn main() {
  let mut args = std::env::args().skip(1);
  let Some(src_arg) = args.next() else {
    eprintln!(
      "用法: cargo run -p ham-web-lrpt --example extract_huffman -- <源 compression.rs> [输出 .rs]"
    );
    std::process::exit(2);
  };
  let src_path = PathBuf::from(src_arg);
  let out_path = args
    .next()
    .map_or_else(|| PathBuf::from("huffman_tables.rs"), PathBuf::from);
  let src = fs::read_to_string(&src_path).expect("读取源文件");
  let dc = parse_table(&src, "dc_huffman_encode");
  let ac = parse_table(&src, "ac_huffman_encode");
  assert_eq!(dc.len(), 12, "DC 表应有 12 条");
  assert!(ac.len() >= 150, "AC 表应有 150+ 条");

  let header = "//! LRPT JPEG 亮度 Huffman 表（DC + AC），逐字转写自 lrpt-encoder\n\
//! （MIT License）`compression.rs`。生成脚本：`crates/lrpt/examples/extract_huffman.rs`。\n\
//!\n\
//! 注意：DC / AC 两张表内部都没有重复符号（0x0A 等符号只是同时出现在两张表里，各自独立）；\n\
//! 未匹配的符号按 EOB 处理（参照实现的 `_` 分支）。\n\n";
  let text = format!(
    "{header}{}\n\n{}\n",
    rust_table("DC_HUFFMAN", &dc),
    rust_table("AC_HUFFMAN", &ac)
  );
  fs::write(&out_path, text).expect("写输出");
  println!(
    "已生成 {out_path:?}：DC {} 条，AC {} 条",
    dc.len(),
    ac.len()
  );
}

/// 提取一个 `fn xxx(category) -> (...) { match ... }` 函数的条目：
/// `(符号, 码字, 码长)`。键可能是十六进制（AC）或十进制（DC）。
fn parse_table(src: &str, fn_name: &str) -> Vec<(u8, u32, u8)> {
  let start = src
    .find(&format!("fn {fn_name}("))
    .unwrap_or_else(|| panic!("找不到 {fn_name}"));
  let open = start + src[start..].find('{').expect("函数体 {");
  let end = matching_brace(src, open + 1); // open 是 '{' 本身，扫描从其后开始
  let mut out = Vec::new();
  for line in src[open + 1..end].lines() {
    // 去掉注释后找 "=> ("。
    let line = match line.find("//") {
      Some(i) => &line[..i],
      None => line,
    };
    let Some(arrow) = line.find("=> (") else {
      continue;
    };
    let sym_part = line[..arrow].trim().trim_end_matches('(').trim();
    if sym_part == "_" {
      continue; // 默认分支不是表条目
    }
    let sym = if let Some(hex) = sym_part.strip_prefix("0x") {
      u8::from_str_radix(hex, 16).expect("十六进制键")
    } else {
      sym_part.parse().expect("十进制键")
    };
    let rest = &line[arrow + 4..];
    let close = rest.find(')').expect("条目右括号");
    let mut parts = rest[..close].split(',');
    let code = parts.next().expect("码字");
    let code = code
      .trim()
      .strip_prefix("0b")
      .expect("二进制码字")
      .chars()
      .fold(0u32, |acc, c| (acc << 1) | u32::from(c == '1'));
    let len: u8 = parts
      .next()
      .expect("码长")
      .trim()
      .parse()
      .expect("码长数值");
    out.push((sym, code, len));
  }
  out
}

/// 括号配对扫描：`from` 处深度为 1，返回配对的 `}` 下标。
fn matching_brace(src: &str, from: usize) -> usize {
  let mut depth = 1usize;
  for (i, ch) in src[from..].char_indices() {
    match ch {
      '{' => depth += 1,
      '}' => {
        depth -= 1;
        if depth == 0 {
          return from + i;
        }
      }
      _ => {}
    }
  }
  panic!("找不到配对的 }}");
}

/// 条目（符号, 码字, 码长）→ Rust 表文本。
fn rust_table(name: &str, entries: &[(u8, u32, u8)]) -> String {
  let mut out =
    vec!["/// 条目（符号, 码字, 码长），逐字转写自 lrpt-encoder `compression.rs`。".to_owned()];
  out.push(format!("pub const {name}: &[(u8, u32, u8)] = &["));
  for (sym, code, len) in entries {
    out.push(format!("  (0x{sym:02X}, 0b{code:b}, {len}),"));
  }
  out.push("];".to_owned());
  out.join("\n")
}

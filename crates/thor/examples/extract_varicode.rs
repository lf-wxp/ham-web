//! 从 fldigi 的 `thorvaricode.cxx` 提取 THOR 的二次信道 Varicode，生成：
//! - `varicode.rs`（本 crate 的表）；
//! - `thor_varicode_data.ts`（e2e 独立 TS 编码器用的数据模块）。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-thor --example extract_varicode -- /path/to/thorvaricode.cxx [输出目录]
//! ```
//! 仓库内已有生成产物（`src/varicode.rs`、`e2e/tests/thor_varicode_data.ts`），
//! 仅当上游 fldigi 表更新时需要重跑。
//!
//! 来源：fldigi（GPL-3.0）`src/thor/thorvaricode.cxx`（© 2008 Dave Freese W1HKJ）。THOR Varicode 是
//! IZ8BLY MFSK Varicode 的扩展：主字符集同 MFSK，二次字符集用 MFSK 码表里没分配的 12 位码字。
//! 提取时与同文件的 `thor_varidecode[]` 逐项比对（字符串按二进制解读必须等于解码表的值），
//! 对不上直接失败。

use std::fs;
use std::path::PathBuf;

fn main() {
  let mut args = std::env::args().skip(1);
  let Some(src_arg) = args.next() else {
    eprintln!(
      "用法: cargo run -p ham-web-thor --example extract_varicode -- <源 thorvaricode.cxx> [输出目录]"
    );
    std::process::exit(2);
  };
  let out_dir = args
    .next()
    .map_or_else(|| PathBuf::from("."), PathBuf::from);
  let src = fs::read(PathBuf::from(src_arg)).expect("读取源文件");
  let src: String = src
    .iter()
    .map(|&b| if b < 0x80 { b as char } else { '?' })
    .collect();

  // 每行一个码字：`"101110000000",   /* 032 - <SPC> */` —— 只取**行首**那对引号里的内容。
  let codes = between(&src, "static const char *thor_varicode[] = {", "};")
    .lines()
    .filter_map(|l| l.trim().strip_prefix('"'))
    .map(|l| l.split('"').next().unwrap_or("").to_owned())
    .collect::<Vec<_>>();
  let decode: Vec<u32> = between(
    &src,
    "static const unsigned int thor_varidecode[] = {",
    "};",
  )
  .split(',')
  .filter_map(|v| v.trim().strip_prefix("0x"))
  .map(|h| u32::from_str_radix(h, 16).expect("十六进制"))
  .collect();
  // 字符 ' '..='z' 共 91 个。
  assert_eq!(codes.len(), 91, "码表应有 91 项（' '..='z'）");
  assert_eq!(decode.len(), 91, "解码表应有 91 项");
  for (i, (c, d)) in codes.iter().zip(&decode).enumerate() {
    assert_eq!(c.len(), 12, "第 {i} 项不是 12 位码字");
    assert!(
      c.bytes().all(|b| b == b'0' || b == b'1'),
      "第 {i} 项不是二进制串"
    );
    assert_eq!(
      u32::from_str_radix(c, 2).unwrap(),
      *d,
      "第 {i} 项与解码表不符"
    );
  }

  let mut rs = String::from(
    "//! THOR 二次信道 Varicode（12 位码字）。\n//!\n//! **生成物**：由 `examples/extract_varicode.rs` 从 fldigi（GPL-3.0）\n//! `src/thor/thorvaricode.cxx` 提取，请勿手改。\n\n/// 字符 `' '`（0x20）至 `'z'`（0x7A）→ 12 位码字（`'1'` / `'0'` 串）。\n///\n/// 这些码字取自 MFSK Varicode 没分配的 12 位码，数值都不小于 `0xB80`，所以接收端能用「码字是否小于\n/// `0xB80`」把主、次信道分开。\npub const SECONDARY: [&str; 91] = [\n",
  );
  for c in &codes {
    rs.push_str(&format!("  \"{c}\",\n"));
  }
  rs.push_str("];\n");
  fs::write(out_dir.join("varicode.rs"), rs).expect("写 varicode.rs");

  let mut ts = String::from(
    "// THOR 二次信道 Varicode（12 位码字，字符 ' '..'z'）。生成物：由 crates/thor/examples/extract_varicode.rs\n// 从 fldigi（GPL-3.0）src/thor/thorvaricode.cxx 提取，请勿手改。\nexport const THOR_SECONDARY_VARICODE: readonly string[] = [\n",
  );
  for c in &codes {
    ts.push_str(&format!("  \"{c}\",\n"));
  }
  ts.push_str("];\n");
  fs::write(out_dir.join("thor_varicode_data.ts"), ts).expect("写 TS 数据");
  println!("已生成 varicode.rs 与 thor_varicode_data.ts（91 码，已与解码表逐项比对）");
}

fn between<'a>(src: &'a str, start: &str, end: &str) -> &'a str {
  let s = src.find(start).expect("找不到表头") + start.len();
  let e = s + src[s..].find(end).expect("找不到表尾");
  &src[s..e]
}

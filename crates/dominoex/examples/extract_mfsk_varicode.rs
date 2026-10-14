//! 从 fldigi 的 `mfskvaricode.cxx` 提取 IZ8BLY MFSK Varicode（DominoEX FEC 用），生成：
//! - `mfsk_varicode.rs`（本 crate 的表）；
//! - `mfsk_varicode_data.ts`（e2e 独立 TS 编码器用的数据模块）。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-dominoex --example extract_mfsk_varicode -- /path/to/mfskvaricode.cxx [输出目录]
//! ```
//! 仓库内已有生成产物（`src/mfsk_varicode.rs`、`e2e/tests/mfsk_varicode_data.ts`），
//! 仅当上游 fldigi 表更新时需要重跑。
//!
//! 来源：fldigi（GPL-3.0）`src/mfsk/mfskvaricode.cxx`，表本身是 IZ8BLY 的 MFSK Varicode
//! （<http://www.qsl.net/zl1bpu/MFSK/Varicode.html>）。提取时与同文件里的 `varidecode[]`
//! 逐项比对（字符串按二进制解读必须等于解码表的值），对不上直接失败。

use std::fs;
use std::path::PathBuf;

fn main() {
  let mut args = std::env::args().skip(1);
  let Some(src_arg) = args.next() else {
    eprintln!(
      "用法: cargo run -p ham-web-dominoex --example extract_mfsk_varicode -- <源 mfskvaricode.cxx> [输出目录]"
    );
    std::process::exit(2);
  };
  let out_dir = args
    .next()
    .map_or_else(|| PathBuf::from("."), PathBuf::from);
  let src = fs::read(PathBuf::from(src_arg)).expect("读取源文件");
  // 源文件注释里有 Latin-1 字符：按字节读，只取 ASCII 的码字与十六进制数。
  let src: String = src
    .iter()
    .map(|&b| if b < 0x80 { b as char } else { '?' })
    .collect();

  // 每行一个码字：`"1110…",  /* 034 - '"' */` —— 注释里也有引号（`'"'`），
  // 所以只取**行首**那对引号里的内容，不能整段按引号切。
  let codes = between(&src, "static const char *varicode[] = {", "};")
    .lines()
    .filter_map(|l| l.trim().strip_prefix('"'))
    .map(|l| l.split('"').next().unwrap_or("").to_owned())
    .collect::<Vec<_>>();
  let decode: Vec<u32> = between(&src, "static const unsigned int varidecode[] = {", "};")
    .split(',')
    .filter_map(|v| v.trim().strip_prefix("0x"))
    .map(|h| u32::from_str_radix(h, 16).expect("十六进制"))
    .collect();
  assert_eq!(codes.len(), 256, "码表应有 256 项");
  assert_eq!(decode.len(), 256, "解码表应有 256 项");
  for (i, (c, d)) in codes.iter().zip(&decode).enumerate() {
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
    "//! IZ8BLY MFSK Varicode（DominoEX FEC 用）。\n//!\n//! **生成物**：由 `examples/extract_mfsk_varicode.rs` 从 fldigi（GPL-3.0）\n//! `src/mfsk/mfskvaricode.cxx` 提取，请勿手改。\n\n/// 字符码 0–255 → 码字（`'1'` / `'0'` 串，首位恒为 1、不含 `001`）。\npub const VARICODE: [&str; 256] = [\n",
  );
  for c in &codes {
    rs.push_str(&format!("  \"{c}\",\n"));
  }
  rs.push_str("];\n");
  fs::write(out_dir.join("mfsk_varicode.rs"), rs).expect("写 mfsk_varicode.rs");

  let mut ts = String::from(
    "// IZ8BLY MFSK Varicode（DominoEX FEC 用）。生成物：由 crates/dominoex/examples/extract_mfsk_varicode.rs\n// 从 fldigi（GPL-3.0）src/mfsk/mfskvaricode.cxx 提取，请勿手改。\nexport const MFSK_VARICODE: readonly string[] = [\n",
  );
  for c in &codes {
    ts.push_str(&format!("  \"{c}\",\n"));
  }
  ts.push_str("];\n");
  fs::write(out_dir.join("mfsk_varicode_data.ts"), ts).expect("写 TS 数据");
  println!("已生成 mfsk_varicode.rs 与 mfsk_varicode_data.ts（256 码，已与解码表逐项比对）");
}

fn between<'a>(src: &'a str, start: &str, end: &str) -> &'a str {
  let s = src.find(start).expect("找不到表头") + start.len();
  let e = s + src[s..].find(end).expect("找不到表尾");
  &src[s..e]
}

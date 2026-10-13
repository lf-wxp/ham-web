//! 从 fldigi 的 `dominovar.cxx` 提取 DominoEX Varicode 表，生成：
//! - `varicode.rs`（本 crate 的表）；
//! - `dominoex_varicode_data.ts`（e2e 独立 TS 编码器用的数据模块）。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-dominoex --example extract_varicode -- /path/to/dominovar.cxx [输出目录]
//! ```
//! 仓库内已有生成产物（`src/varicode.rs`、`e2e/tests/dominoex_varicode_data.ts`），
//! 仅当上游 fldigi 表更新时需要重跑。
//!
//! 来源：fldigi（GPL-3.0）`src/dominoex/dominovar.cxx`，© 2001–2003 Tomi
//! Manninen OH2BNS。提取逻辑无正则依赖（纯手写括号扫描）。

use std::fs;
use std::path::PathBuf;

fn main() {
  let mut args = std::env::args().skip(1);
  let Some(src_arg) = args.next() else {
    eprintln!(
      "用法: cargo run -p ham-web-dominoex --example extract_varicode -- <源 dominovar.cxx> [输出目录]"
    );
    std::process::exit(2);
  };
  let src_path = PathBuf::from(src_arg);
  let out_dir = args
    .next()
    .map_or_else(|| PathBuf::from("."), PathBuf::from);
  let src = fs::read_to_string(&src_path).expect("读取源文件");
  let (rows, vals) = extract(&src);
  fs::write(out_dir.join("varicode.rs"), rust_table(&rows, &vals)).expect("写 varicode.rs");
  fs::write(out_dir.join("dominoex_varicode_data.ts"), ts_table(&rows)).expect("写 TS 数据");
  println!("已生成 varicode.rs 与 dominoex_varicode_data.ts（512 码 / 4096 解码项）");
}

/// 提取 `varicode[][3]` 与 `varidecode[]` 两个表（含往返自检）。
fn extract(src: &str) -> (Vec<[u8; 3]>, Vec<i16>) {
  let mark = "static unsigned char varicode[][3] = {";
  let start = src.find(mark).expect("找不到 varicode 表") + mark.len();
  let end = matching_brace(src, start);
  let mut rows = Vec::with_capacity(512);
  let mut depth = 0usize;
  let mut cur = String::new();
  for ch in src[start..end].chars() {
    match ch {
      '{' => {
        depth += 1;
        cur.clear();
      }
      '}' => {
        depth -= 1;
        if depth == 0 {
          let nums: Vec<u8> = cur
            .split(',')
            .map(|s| s.trim().parse().expect("三元组数值"))
            .collect();
          assert_eq!(nums.len(), 3, "三元组 {cur:?}");
          rows.push([nums[0], nums[1], nums[2]]);
        }
      }
      _ if depth >= 1 => cur.push(ch),
      _ => {}
    }
  }
  assert_eq!(rows.len(), 512, "varicode 应有 512 行");

  let mark2 = "static signed int varidecode[] = {";
  let start2 = src.find(mark2).expect("找不到 varidecode 表") + mark2.len();
  let end2 = matching_brace(src, start2);
  // 最后一个 chunk 是结尾换行（数组尾逗号后无内容），过滤空项。
  let vals: Vec<i16> = src[start2..end2]
    .split(',')
    .filter(|s| !s.trim().is_empty())
    .map(|s| s.trim().parse().expect("varidecode 数值"))
    .collect();
  assert_eq!(vals.len(), 4096, "varidecode 应有 4096 项");

  // 往返自检：除已知碰撞（二次信道 379 与 381 同码）外全部往返。
  for (c, row) in rows.iter().enumerate() {
    if c == 379 {
      continue;
    }
    let nibs: Vec<u8> = row.iter().copied().filter(|&n| n != 0).collect();
    let nibs = if nibs.is_empty() { vec![0] } else { nibs };
    let packed: usize = nibs
      .iter()
      .enumerate()
      .map(|(i, &n)| usize::from(n) << (4 * (nibs.len() - 1 - i)))
      .sum();
    assert_eq!(vals[packed], c as i16, "char {c}: packed {packed:#x}");
  }
  (rows, vals)
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

/// 生成 Rust 表（格式与仓库内现有 `src/varicode.rs` 一致）。
fn rust_table(rows: &[[u8; 3]], vals: &[i16]) -> String {
  let mut out = vec![
    "//! DominoEX Varicode 表（转写自 fldigi `src/dominoex/dominovar.cxx`，GPL-3.0，".to_owned(),
  ];
  out.push("//! © 2001–2003 Tomi Manninen OH2BNS）。生成脚本：`crates/dominoex/examples/extract_varicode.rs`。".to_owned());
  out.push("//!".to_owned());
  out.push(
    "//! 结构：字符（主字母表 0–255；二次信道 / ID 256–511）→ 1–3 个 nibble（尾部 0 = 无续码）。"
      .to_owned(),
  );
  out.push(
    "//! 首个 nibble 的 MSB 恒为 0、续码 MSB 恒为 1 —— 这是 nibble 自同步的依据。".to_owned(),
  );
  out.push("//!".to_owned());
  out.push("//! 已知例外（照录参考实现，如实保留）：二次信道的 379 与 381 共享同一码".to_owned());
  out.push(
    "//! `{5,10,12}`，`VARIDECODE` 里该码解为 381（表内后写者赢）—— 这是 fldigi 表".to_owned(),
  );
  out.push("//! 自身的数据，不「修」它；解码侧 379 永远解不出，属参考实现的既定行为。".to_owned());
  out.push(String::new());
  out.push("/// 字符 → 最多 3 个 nibble（尾部 0 = 没有更多 nibble）。".to_owned());
  out.push("pub const VARICODE: [[u8; 3]; 512] = [".to_owned());
  for i in (0..512).step_by(8) {
    let chunk = rows[i..i + 8]
      .iter()
      .map(|r| format!("[{}, {}, {}]", r[0], r[1], r[2]))
      .collect::<Vec<_>>()
      .join(", ");
    out.push(format!("  {chunk},"));
  }
  out.push("];".to_owned());
  out.push(String::new());
  out.push("/// 反向打包（最新 nibble 在最低 4 位）→ 字符值（0–511；-1 = 非法）。".to_owned());
  out.push("pub const VARIDECODE: [i16; 4096] = [".to_owned());
  for i in (0..4096).step_by(16) {
    let chunk = vals[i..i + 16]
      .iter()
      .map(|v| v.to_string())
      .collect::<Vec<_>>()
      .join(", ");
    out.push(format!("  {chunk},"));
  }
  out.push("];".to_owned());
  out.join("\n") + "\n"
}

/// 生成 e2e 用的 TS 数据模块。
fn ts_table(rows: &[[u8; 3]]) -> String {
  let mut out = vec!["/** DominoEX Varicode 表（转写自 fldigi dominovar.cxx，GPL-3.0；".to_owned()];
  out.push(" * 生成：crates/dominoex/examples/extract_varicode.rs）。 */".to_owned());
  out.push("export const VARICODE: [number, number, number][] = [".to_owned());
  for i in (0..512).step_by(8) {
    let chunk = rows[i..i + 8]
      .iter()
      .map(|r| format!("[{}, {}, {}]", r[0], r[1], r[2]))
      .collect::<Vec<_>>()
      .join(", ");
    out.push(format!("  {chunk},"));
  }
  out.push("];".to_owned());
  out.join("\n") + "\n"
}

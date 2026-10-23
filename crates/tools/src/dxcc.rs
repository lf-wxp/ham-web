//! 由 AD1C 维护的 `cty.csv`（<https://www.country-files.com>）生成内置 DXCC 前缀表
//! `crates/core/data/dxcc.txt`。
//!
//! 输出每行一个实体：`DXCC 编号|主前缀|英文名|大洲|CQ 区|ITU 区|纬度|经度(东经为正)|前缀…`。
//! 只保留前缀规则并去掉逐前缀的分区覆盖；整呼号规则（`=`）仅对没有任何前缀规则的实体保留。
//! WAE 专用实体（主前缀以 `*` 开头）不属于 DXCC，跳过。

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::provenance::{self, Stamp};

pub const DEFAULT_URL: &str = "https://www.country-files.com/bigcty/cty.csv";

/// 去掉 `(cq)`、`[itu]`、`<lat/lon>`、`{cont}`、`~tz~` 等覆盖标记。
fn strip_overrides(token: &str) -> &str {
  token
    .find(['(', '[', '<', '{', '~'])
    .map_or(token, |i| &token[..i])
}

/// 从 cty.csv 里尽力提取上游期号。
///
/// CTY 文件用一条特殊规则标记版本实体，形如 `=VER20260915`（数字是发布日期）；取不到就
/// 返回 `None`——戳里只是少一行，不影响生成。**不靠猜**：宁可没有，也不写一个错的版本号。
fn upstream_version(csv: &str) -> Option<String> {
  csv
    .split_whitespace()
    .filter_map(|t| t.trim_start_matches('=').strip_prefix("VER"))
    .find(|rest| rest.len() == 8 && rest.chars().all(|c| c.is_ascii_digit()))
    .map(|rest| format!("VER{rest}"))
}

fn convert(csv: &str, stamp: &Stamp) -> Result<String> {
  let mut out = stamp.header_block("dxcc");
  let mut count = 0;
  for line in csv.lines() {
    let line = line.trim().trim_end_matches(';');
    if line.is_empty() {
      continue;
    }
    let f: Vec<&str> = line.splitn(10, ',').collect();
    if f.len() < 10 {
      bail!("malformed cty.csv line: {line}");
    }
    if f[0].starts_with('*') {
      continue;
    }
    let lon: f64 = f[7]
      .parse()
      .with_context(|| format!("bad longitude in: {line}"))?;
    let tokens: Vec<&str> = f[9].split_whitespace().collect();
    let mut prefixes: Vec<&str> = tokens
      .iter()
      .filter(|t| !t.starts_with('='))
      .map(|t| strip_overrides(t))
      .filter(|t| !t.is_empty())
      .collect();
    if prefixes.is_empty() {
      prefixes = tokens.iter().map(|t| strip_overrides(t)).collect();
    }
    prefixes.sort_unstable();
    prefixes.dedup();
    writeln!(
      out,
      "{}|{}|{}|{}|{}|{}|{}|{}|{}",
      f[2],
      f[0],
      f[1],
      f[3],
      f[4],
      f[5],
      f[6],
      -lon,
      prefixes.join(" ")
    )?;
    count += 1;
  }
  if count < 300 {
    bail!("only {count} entities parsed, cty.csv looks incomplete");
  }
  Ok(out)
}

pub fn generate(root: &Path, input: Option<&Path>, url: &str) -> Result<()> {
  let (csv, stamp) = match input {
    Some(p) => {
      let bytes = fs::read(p).with_context(|| format!("failed to read {}", p.display()))?;
      let csv =
        String::from_utf8(bytes.clone()).with_context(|| format!("{} 不是 UTF-8", p.display()))?;
      (csv, Stamp::of_local(p, &bytes))
    }
    None => {
      let fetched = provenance::download(url, 8 * 1024 * 1024)?;
      let csv = String::from_utf8(fetched.bytes.clone())
        .with_context(|| format!("{url} 返回的不是 UTF-8"))?;
      let stamp = Stamp::of(url, &fetched.bytes)
        .with_validators(fetched.etag.as_deref(), fetched.last_modified.as_deref());
      (csv, stamp)
    }
  };
  let stamp = match upstream_version(&csv) {
    Some(v) => stamp.with_version(v),
    None => stamp,
  };
  let text = convert(&csv, &stamp)?;
  let path = root.join("crates/core/data/dxcc.txt");
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir)?;
  }
  fs::write(&path, &text).with_context(|| format!("failed to write {}", path.display()))?;
  println!(
    "Wrote {} ({} entities)",
    path.display(),
    text.lines().filter(|l| !l.starts_with('#')).count()
  );
  if let Some(v) = &stamp.version {
    println!("上游版本 {v}，摘要 {}", &stamp.digest[..12]);
  } else {
    println!(
      "上游未提供期号（戳里只有摘要 {}）；`check-data-freshness` 将按摘要比对",
      &stamp.digest[..12]
    );
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn strips_overrides_and_skips_wae() {
    let mut csv = String::from(
      "*TA1,European Turkey,390,EU,20,39,41.02,-28.97,-2.0,TA1C TA1D;\n\
       KH6,Hawaii,110,OC,31,61,21.12,157.48,10.0,AH6 KH6(31)[61] =K6ABC;\n\
       BS7,Scarborough Reef,506,AS,27,50,15.08,-117.72,-8.0,=BS7H =BS77H;\n",
    );
    for _ in 0..300 {
      csv.push_str("1A,Sov Mil Order of Malta,246,EU,15,28,41.90,-12.43,-1.0,1A;\n");
    }
    let out = convert(&csv, &Stamp::of("test", csv.as_bytes())).expect("convert");
    let lines: Vec<&str> = out.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(lines[0], "110|KH6|Hawaii|OC|31|61|21.12|-157.48|AH6 KH6");
    assert_eq!(
      lines[1],
      "506|BS7|Scarborough Reef|AS|27|50|15.08|117.72|=BS77H =BS7H"
    );
    // 头部的来源戳必须齐：地址、摘要、抓取时间。
    assert!(out.starts_with("# 由 `ham-web-tools dxcc` 从 test 生成"));
    assert!(out.contains("# 摘要: sha256:"));
    assert!(out.contains("# 抓取: "));
  }

  #[test]
  fn extracts_version_from_marker() {
    let csv = "110|KH6|Hawaii|OC|31|61|21.12|157.48|AH6 KH6 =VER20260915\n";
    assert_eq!(upstream_version(csv), Some("VER20260915".to_owned()));
  }

  #[test]
  fn missing_or_malformed_version_marker_yields_none() {
    // 本地产物里那条裸 `=VERSION` 占位标记不能被当成期号。
    assert_eq!(upstream_version("460|3D2/r|Rotuma|=VERSION\n"), None);
    // 位数不对、含非数字，都不算。
    assert_eq!(upstream_version("x =VER2026\n"), None);
    assert_eq!(upstream_version("x =VER2026091X\n"), None);
    assert_eq!(upstream_version("110|KH6|Hawaii\n"), None);
  }
}

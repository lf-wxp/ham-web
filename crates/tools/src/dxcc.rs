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

pub const DEFAULT_URL: &str = "https://www.country-files.com/bigcty/cty.csv";

/// 去掉 `(cq)`、`[itu]`、`<lat/lon>`、`{cont}`、`~tz~` 等覆盖标记。
fn strip_overrides(token: &str) -> &str {
  token
    .find(['(', '[', '<', '{', '~'])
    .map_or(token, |i| &token[..i])
}

fn convert(csv: &str) -> Result<String> {
  let mut out = String::from(
    "# 由 `ham-web-tools dxcc` 从 country-files.com 的 cty.csv 生成，请勿手工编辑。\n",
  );
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
  let csv = match input {
    Some(p) => fs::read_to_string(p).with_context(|| format!("failed to read {}", p.display()))?,
    None => {
      let mut res = ureq::get(url)
        .call()
        .with_context(|| format!("failed to download {url}"))?;
      res
        .body_mut()
        .with_config()
        .limit(8 * 1024 * 1024)
        .read_to_string()
        .with_context(|| format!("failed to download {url}"))?
    }
  };
  let text = convert(&csv)?;
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
    let out = convert(&csv).expect("convert");
    let lines: Vec<&str> = out.lines().skip(1).collect();
    assert_eq!(lines[0], "110|KH6|Hawaii|OC|31|61|21.12|-157.48|AH6 KH6");
    assert_eq!(
      lines[1],
      "506|BS7|Scarborough Reef|AS|27|50|15.08|117.72|=BS77H =BS7H"
    );
  }
}

//! 性能预算：校验 release 产物的传输体积（`perf-budget.json`）。
//!
//! 口径是 postbuild 预生成的 `.br`（线上按 `Accept-Encoding` 直接发它），而不是原始
//! 文件 —— 原始 wasm 8 MB、br 后 2 MB，拿原始体积定预算会把「压缩率变差」这类真回归
//! 藏起来，也会让阈值失去直觉。没有 `.br` 时视为没跑 postbuild，直接报错而不是退回
//! 原始体积（退回去会让同一份预算在两种口径下判定）。

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Budget {
  assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
  name: String,
  /// 相对 dist 的路径模式，只支持 `*`（不跨 `/`）。
  pattern: String,
  /// 上次核对时的实测值（只用来显示增量）。
  baseline: u64,
  /// 上限（字节，br 压缩后）。
  max: u64,
}

/// `*` 通配（不跨路径分隔符）。
fn glob_match(pat: &str, s: &str) -> bool {
  let (p, t): (Vec<char>, Vec<char>) = (pat.chars().collect(), s.chars().collect());
  let (mut pi, mut ti) = (0usize, 0usize);
  let (mut star, mut mark) = (None::<usize>, 0usize);
  while ti < t.len() {
    if pi < p.len() && p[pi] != '*' && p[pi] == t[ti] {
      pi += 1;
      ti += 1;
    } else if pi < p.len() && p[pi] == '*' {
      star = Some(pi);
      mark = ti;
      pi += 1;
    } else if let Some(sp) = star
      && t[mark] != '/'
    {
      pi = sp + 1;
      mark += 1;
      ti = mark;
    } else {
      return false;
    }
  }
  while pi < p.len() && p[pi] == '*' {
    pi += 1;
  }
  pi == p.len()
}

/// 一项资源的判定结果。
struct Row {
  name: String,
  file: String,
  size: u64,
  baseline: u64,
  max: u64,
}

fn collect(dist: &Path, budget: &Budget) -> Result<Vec<Row>> {
  let mut files: Vec<String> = Vec::new();
  for e in walkdir::WalkDir::new(dist).max_depth(2) {
    let e = e?;
    if !e.file_type().is_file() {
      continue;
    }
    let rel = e
      .path()
      .strip_prefix(dist)?
      .to_string_lossy()
      .replace('\\', "/");
    files.push(rel);
  }
  files.sort();
  let mut rows = Vec::new();
  for a in &budget.assets {
    let matched: Vec<&String> = files.iter().filter(|f| glob_match(&a.pattern, f)).collect();
    // 模式一个文件都没对上 = 预算表与产物脱节（改了文件名 / 漏了构建步骤），
    // 静默通过就等于这一项没有预算。
    if matched.is_empty() {
      bail!(
        "预算项「{}」的模式 `{}` 在 dist 里没有匹配的文件",
        a.name,
        a.pattern
      );
    }
    for f in matched {
      let br = dist.join(format!("{f}.br"));
      let size = std::fs::metadata(&br)
        .with_context(|| {
          format!("缺少 {f}.br：先跑 `cargo make build-web`（含 postbuild 预压缩）")
        })?
        .len();
      rows.push(Row {
        name: a.name.clone(),
        file: f.clone(),
        size,
        baseline: a.baseline,
        max: a.max,
      });
    }
  }
  Ok(rows)
}

fn kib(n: u64) -> String {
  format!("{:.1} KiB", n as f64 / 1024.0)
}

/// 跑一次预算校验；有超限项时返回错误（退出码非零）。
pub fn run(root: &Path, dist: &Path) -> Result<()> {
  let text =
    std::fs::read_to_string(root.join("perf-budget.json")).context("读取 perf-budget.json")?;
  let budget: Budget = serde_json::from_str(&text).context("解析 perf-budget.json")?;
  let rows = collect(dist, &budget)?;
  let mut over = 0usize;
  println!("性能预算（br 压缩后体积）：");
  for r in &rows {
    let delta = r.size as i64 - r.baseline as i64;
    let ok = r.size <= r.max;
    if !ok {
      over += 1;
    }
    println!(
      "  {} {:<16} {:<44} {:>11} / {:>11}（较基线 {:+.1} KiB）",
      if ok { "✓" } else { "✗" },
      r.name,
      r.file,
      kib(r.size),
      kib(r.max),
      delta as f64 / 1024.0
    );
  }
  if over > 0 {
    bail!(
      "{over} 项超出性能预算：先评估能否拆进 Worker / 按需加载，确需上调时改 perf-budget.json 并在 ROADMAP 记录原因"
    );
  }
  println!("✓ {} 项资源均在预算内", rows.len());
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn star_matches_within_a_path_segment_only() {
    assert!(glob_match(
      "ham-web-app-*_bg.wasm",
      "ham-web-app-37a6_bg.wasm"
    ));
    assert!(glob_match(
      "*-worker/*_bg.wasm",
      "apt-worker/apt_worker_bg.wasm"
    ));
    assert!(glob_match("sw.js", "sw.js"));
    assert!(!glob_match("sw.js", "sw.js.br"));
    // `*` 不跨目录：顶层模式不能误配到子目录里的同名文件。
    assert!(!glob_match("*.js", "apt-worker/apt_worker.js"));
    assert!(!glob_match("ham-web-app-*.js", "ham-web-app-1_bg.wasm"));
    // 主包 JS 的模式不能把 wasm 也吞进来（两者共用前缀）。
    assert!(glob_match("ham-web-app-*.js", "ham-web-app-37a6.js"));
  }

  #[test]
  fn the_committed_budget_parses_and_baselines_fit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join("perf-budget.json")).unwrap();
    let budget: Budget = serde_json::from_str(&text).unwrap();
    assert!(!budget.assets.is_empty());
    for a in &budget.assets {
      // 基线本身就超预算 = 预算表写错了（或者上调了 baseline 忘了上调 max）。
      assert!(a.baseline <= a.max, "{} 的基线超出上限", a.name);
      assert!(!a.pattern.contains("**"), "{} 只支持单层 `*`", a.name);
    }
  }
}

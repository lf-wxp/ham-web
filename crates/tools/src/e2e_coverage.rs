//! e2e 覆盖对账：`e2e/coverage-targets.json` 的每条核心旅程必须**唯一**命中一条 spec 用例。
//!
//! 背景：仓库的规矩是「每个交互页都有专属 e2e」，但这条规矩一直靠人记。改一次用例标题
//! 就会让清单悄悄失效，覆盖率统计却照样报一个好看的数字。这里把「清单 ↔ 实际用例」
//! 做成硬校验：grep 命中不了就红。
//!
//! 命中多条同样要红：新用例沿用旧前缀（如「设计存档……」后面又加了「设计存档……对比」）
//! 时，grep 会同时匹配两条，清单指向哪一条就说不清了。这类问题原先只在
//! `cargo make e2e-coverage`（`e2e/scripts/coverage.mjs`）里才报，等到 CI 的覆盖率步骤
//! 才红；放在这里就能在 `cargo make check` 阶段挡住。
//!
//! 只做**正向**校验（清单里的条目必须存在），不反推「每个路由是不是都有用例」——
//! 后者要靠白名单排除纯展示页，误报率远高于收益，留给人工 review。

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Deserialize)]
struct Targets {
  journeys: Vec<Journey>,
}

#[derive(Deserialize)]
struct Journey {
  group: String,
  name: String,
  grep: String,
}

/// 校验覆盖清单：每条 journey 的 `grep` 必须唯一命中一条用例标题。
pub fn check(root: &Path) -> Result<()> {
  let file = root.join("e2e").join("coverage-targets.json");
  let targets: Targets =
    crate::fsutil::read_json(&file).with_context(|| format!("读取 {} 失败", file.display()))?;
  let titles = spec_titles(root)?;
  let missing = missing_journeys(&targets.journeys, &titles);
  if !missing.is_empty() {
    bail!(
      "有 {} 条核心旅程的 grep 在 e2e/tests 下找不到（用例标题改了却没同步 coverage-targets.json）：\n  {}",
      missing.len(),
      missing.join("\n  ")
    );
  }
  let ambiguous = ambiguous_journeys(&targets.journeys, &titles);
  if !ambiguous.is_empty() {
    bail!(
      "有 {} 条核心旅程的 grep 命中多条用例（请把 grep 收窄到能唯一区分）：\n  {}",
      ambiguous.len(),
      ambiguous.join("\n  ")
    );
  }
  println!(
    "✓ e2e 覆盖清单 {} 条 journey 全部唯一命中",
    targets.journeys.len()
  );
  Ok(())
}

/// 找出 grep 命中不了任何 spec 的条目（按 `组 / 名称（grep）` 格式化）。
fn missing_journeys(journeys: &[Journey], titles: &[String]) -> Vec<String> {
  journeys
    .iter()
    .filter(|journey| matches(&journey.grep, titles).is_empty())
    .map(describe)
    .collect()
}

/// 找出 grep 命中多条用例的条目。命中多条说明 grep 太宽（新用例带了同样的前缀），
/// 清单到底对应哪条用例就不确定了。
fn ambiguous_journeys(journeys: &[Journey], titles: &[String]) -> Vec<String> {
  journeys
    .iter()
    .filter(|journey| matches(&journey.grep, titles).len() > 1)
    .map(describe)
    .collect()
}

/// grep 命中的用例标题。
fn matches<'a>(grep: &str, titles: &'a [String]) -> Vec<&'a str> {
  titles
    .iter()
    .map(String::as_str)
    .filter(|title| title.contains(grep))
    .collect()
}

fn describe(journey: &Journey) -> String {
  format!(
    "{} / {}（grep「{}」）",
    journey.group, journey.name, journey.grep
  )
}

/// 递归收集 `e2e/tests` 下所有 `.ts` 的**用例标题**。
///
/// - 递归：将来把 spec 分到子目录（`tests/log/…`）时不会静默漏掉；
/// - 只留 `test(` / `test.describe(` 所在行：整文件 `contains` 会让注释里出现同样中文
///   也算命中，守卫可以被无意绕过；
/// - 按「一行一条标题」返回：条数要用来判断 grep 是否唯一命中。
fn spec_titles(root: &Path) -> Result<Vec<String>> {
  let dir = root.join("e2e").join("tests");
  let mut out = Vec::new();
  let mut stack = vec![dir];
  while let Some(current) = stack.pop() {
    for entry in
      std::fs::read_dir(&current).with_context(|| format!("读取 {} 失败", current.display()))?
    {
      let path = entry?.path();
      if path.is_dir() {
        stack.push(path);
      } else if path.extension().is_some_and(|ext| ext == "ts") {
        let text = std::fs::read_to_string(&path)
          .with_context(|| format!("读取 {} 失败", path.display()))?;
        out.extend(titles(&text));
      }
    }
  }
  Ok(out)
}

/// 抽出 spec 里的用例标题行。
fn titles(spec: &str) -> Vec<String> {
  spec
    .lines()
    .filter(|line| {
      let line = line.trim_start();
      line.starts_with("test(") || line.starts_with("test.describe(")
    })
    .map(str::to_owned)
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn journey(group: &str, name: &str, grep: &str) -> Journey {
    Journey {
      group: group.to_owned(),
      name: name.to_owned(),
      grep: grep.to_owned(),
    }
  }

  #[test]
  fn reports_only_the_journeys_missing_from_every_spec() {
    let journeys = vec![
      journey("复习", "听题", "听题模式：依次朗读题目"),
      journey("工具", "合规", "合规检查：选类别与频率"),
    ];
    let specs = vec![
      "test(\"听题模式：依次朗读题目\", async () => {})".to_owned(),
      "test(\"别的用例\", async () => {})".to_owned(),
    ];
    let missing = missing_journeys(&journeys, &specs);
    assert_eq!(missing.len(), 1);
    assert!(missing[0].contains("合规检查：选类别与频率"));
    assert!(missing[0].starts_with("工具 / 合规"));
  }

  #[test]
  fn an_empty_spec_list_makes_everything_missing() {
    let journeys = vec![journey("工具", "合规", "合规检查：选类别与频率")];
    assert_eq!(missing_journeys(&journeys, &[]).len(), 1);
  }

  #[test]
  fn comments_do_not_count_as_a_journey() {
    // 注释里抄一遍标题不算覆盖：必须真的有一条 `test("…")`。
    let spec = r#"
// test("听题模式：依次朗读题目", async () => {})
test("别的用例", async () => {});
"#;
    let titles = titles(spec);
    assert!(titles.iter().all(|t| !t.contains("听题模式：依次朗读题目")));
    assert!(titles.iter().any(|t| t.contains("别的用例")));
  }

  #[test]
  fn a_grep_matching_any_spec_is_enough() {
    let journeys = vec![journey("工具", "合规", "合规检查：选类别与频率")];
    let titles = vec!["".to_owned(), "test('合规检查：选类别与频率')".to_owned()];
    assert!(missing_journeys(&journeys, &titles).is_empty());
  }

  #[test]
  fn a_grep_matching_two_titles_is_ambiguous() {
    // 新用例沿用旧前缀时，grep 会同时命中两条，清单指向哪一条就说不清了。
    let journeys = vec![journey("工具", "设计存档", "设计存档")];
    let titles = vec![
      "test(\"设计存档保存、刷新后载回\", async () => {})".to_owned(),
      "test(\"设计存档可以两份并排对比\", async () => {})".to_owned(),
    ];
    assert!(missing_journeys(&journeys, &titles).is_empty());
    let ambiguous = ambiguous_journeys(&journeys, &titles);
    assert_eq!(ambiguous.len(), 1);
    assert!(ambiguous[0].starts_with("工具 / 设计存档"));
  }

  #[test]
  fn a_narrowed_grep_is_unique_again() {
    let journeys = vec![journey("工具", "设计存档", "设计存档保存、刷新后载回")];
    let titles = vec![
      "test(\"设计存档保存、刷新后载回\", async () => {})".to_owned(),
      "test(\"设计存档可以两份并排对比\", async () => {})".to_owned(),
    ];
    assert!(ambiguous_journeys(&journeys, &titles).is_empty());
  }
}

//! 听题模式：把一道题转换成依次朗读 / 停顿的脚本。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::question::QuestionItem;

/// 单段朗读的最大字数：Chrome 的语音合成对过长的句子可能中途停止，按句切分。
pub const MAX_CHUNK: usize = 80;
/// 可选的思考时间（秒）。
pub const THINK_CHOICES: [u32; 4] = [3, 5, 8, 12];
/// 可选的语速。
pub const RATE_CHOICES: [f32; 4] = [0.8, 1.0, 1.2, 1.5];

/// 听题的题源。
///
/// 序列化用小写变体名（`a` / `b` / `c` / `mistakes` / `bookmarks`），是已经写进用户存储的格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
  A,
  B,
  C,
  Mistakes,
  Bookmarks,
}

impl Source {
  pub const ALL: [Self; 5] = [Self::A, Self::B, Self::C, Self::Mistakes, Self::Bookmarks];

  #[must_use]
  pub const fn label(self) -> &'static str {
    match self {
      Self::A => "A 类",
      Self::B => "B 类",
      Self::C => "C 类",
      Self::Mistakes => "错题集",
      Self::Bookmarks => "收藏集",
    }
  }

  /// 序列化里的写法（数据库列用同一个写法）。
  #[must_use]
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::A => "a",
      Self::B => "b",
      Self::C => "c",
      Self::Mistakes => "mistakes",
      Self::Bookmarks => "bookmarks",
    }
  }
}

/// 听题模式的设置（`localStorage` 的 `listen-settings`）。
///
/// 放在 core 是因为账号同步的服务端要把它存进数据库表，前后端必须是同一份定义。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
  pub source: Source,
  pub rate: f32,
  pub think_secs: u32,
  pub explain: bool,
  pub shuffle: bool,
  /// 各来源上次听到的位置（顺序模式）。
  pub positions: BTreeMap<Source, usize>,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      source: Source::A,
      rate: 1.0,
      think_secs: 5,
      explain: false,
      shuffle: false,
      positions: BTreeMap::new(),
    }
  }
}

/// 脚本所处阶段，供界面高亮。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
  Question,
  Think,
  Answer,
  Explain,
}

/// 脚本的一步。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
  Say(Phase, String),
  Pause(Phase, u32),
}

impl Step {
  #[must_use]
  pub const fn phase(&self) -> Phase {
    match self {
      Self::Say(p, _) | Self::Pause(p, _) => *p,
    }
  }
}

/// 把题目文本整理成适合朗读的形式：填空括号读作「空白」，合并空白字符。
#[must_use]
pub fn speakable(text: &str) -> String {
  let mut s = text.replace(['（', '('], "（").replace([')', '）'], "）");
  for blank in ["（ ）", "（）", "（  ）", "____", "___", "__"] {
    s = s.replace(blank, "空白");
  }
  s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 按句末标点切分，单句仍超过 `max` 字时再按逗号与字数硬切。
#[must_use]
pub fn chunks(text: &str, max: usize) -> Vec<String> {
  let mut out = Vec::new();
  let mut cur = String::new();
  let flush = |cur: &mut String, out: &mut Vec<String>| {
    let t = cur.trim();
    if !t.is_empty() {
      out.push(t.to_owned());
    }
    cur.clear();
  };
  for c in text.chars() {
    cur.push(c);
    let len = cur.chars().count();
    let hard_end = matches!(c, '。' | '！' | '？' | '；' | '\n' | '!' | '?' | ';');
    let soft_end = matches!(c, '，' | ',' | '、' | '：' | ':') && len >= max / 2;
    if hard_end || soft_end || len >= max {
      flush(&mut cur, &mut out);
    }
  }
  flush(&mut cur, &mut out);
  out
}

fn say_all(steps: &mut Vec<Step>, phase: Phase, text: &str) {
  steps.extend(
    chunks(&speakable(text), MAX_CHUNK)
      .into_iter()
      .map(|t| Step::Say(phase, t)),
  );
}

/// 第 `number` 题的朗读脚本：题号与题型 → 题干 → 各选项 → 思考停顿 → 正确答案（→ 解析）。
#[must_use]
pub fn script(q: &QuestionItem, number: usize, think_secs: u32, explain: bool) -> Vec<Step> {
  let mut steps = vec![Step::Say(
    Phase::Question,
    format!("第 {number} 题，{}题。", q.kind.label()),
  )];
  say_all(&mut steps, Phase::Question, &q.question);
  for o in &q.options {
    say_all(
      &mut steps,
      Phase::Question,
      &format!("{}，{}。", o.key, o.text.trim()),
    );
  }
  // `think_secs` 来自持久化 / 云端数据（`u32`，没有上限）：乘法用饱和版本，
  // 免得一份被改坏的数据让 dev 构建 panic、release 构建回绕成错误停顿。
  steps.push(Step::Pause(Phase::Think, think_secs.saturating_mul(1000)));
  steps.push(Step::Say(
    Phase::Answer,
    format!("正确答案：{}。", q.answer_keys.join("、")),
  ));
  if explain
    && let Some(text) = q
      .explanation
      .as_deref()
      .map(str::trim)
      .filter(|s| !s.is_empty())
  {
    steps.push(Step::Pause(Phase::Explain, 400));
    say_all(&mut steps, Phase::Explain, &format!("解析：{text}"));
  }
  steps.push(Step::Pause(Phase::Answer, 1200));
  steps
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::question::{Codes, QuestionOption, QuestionType};

  fn q(explanation: Option<&str>) -> QuestionItem {
    QuestionItem {
      id: Some("A-1".into()),
      codes: Codes::default(),
      question: "业余电台的发射功率（ ）\n应当符合规定".into(),
      options: vec![
        QuestionOption {
          key: "A".into(),
          text: "不超过执照核定值 ".into(),
        },
        QuestionOption {
          key: "B".into(),
          text: "越大越好".into(),
        },
      ],
      answer_keys: vec!["A".into()],
      kind: QuestionType::Single,
      pages: None,
      image_url: None,
      explanation: explanation.map(Into::into),
    }
  }

  #[test]
  fn script_reads_question_options_then_answer() {
    let steps = script(&q(None), 3, 5, true);
    assert_eq!(
      steps[0],
      Step::Say(Phase::Question, "第 3 题，单选题。".into())
    );
    assert_eq!(
      steps[1],
      Step::Say(
        Phase::Question,
        "业余电台的发射功率空白 应当符合规定".into()
      )
    );
    assert_eq!(
      steps[2],
      Step::Say(Phase::Question, "A，不超过执照核定值。".into())
    );
    assert!(steps.contains(&Step::Pause(Phase::Think, 5000)));
    assert!(steps.contains(&Step::Say(Phase::Answer, "正确答案：A。".into())));
    assert!(
      !steps
        .iter()
        .any(|s| s.phase() == Phase::Explain && matches!(s, Step::Say(..)))
    );
  }

  #[test]
  fn explanation_only_when_enabled() {
    let item = q(Some("依据《条例》。功率须符合核定值。"));
    let with = script(&item, 1, 3, true);
    let said: Vec<&str> = with
      .iter()
      .filter_map(|s| match s {
        Step::Say(Phase::Explain, t) => Some(t.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(said, ["解析：依据《条例》。", "功率须符合核定值。"]);
    assert!(
      !script(&item, 1, 3, false)
        .iter()
        .any(|s| matches!(s, Step::Say(Phase::Explain, _)))
    );
  }

  #[test]
  fn an_absurd_think_time_does_not_overflow() {
    // 一份被改坏的设置（或别的设备同步过来的怪值）不能把乘法变成 panic 或回绕。
    let steps = script(&q(None), 1, u32::MAX, false);
    assert!(steps.contains(&Step::Pause(Phase::Think, u32::MAX)));
  }

  #[test]
  fn long_text_is_chunked() {
    let long = "甲".repeat(200);
    let parts = chunks(&long, MAX_CHUNK);
    assert_eq!(parts.len(), 3);
    assert!(parts.iter().all(|p| p.chars().count() <= MAX_CHUNK));
    assert_eq!(chunks("短句，没有句号", MAX_CHUNK), ["短句，没有句号"]);
  }

  #[test]
  fn the_stored_settings_shape_is_pinned_and_the_column_spelling_matches_serde() {
    // 写进 localStorage、也进云端备份的格式：字段名和顺序都不能悄悄变。
    let mut s = Settings::default();
    s.positions.insert(Source::Mistakes, 3);
    s.positions.insert(Source::A, 7);
    let json = r#"{"source":"a","rate":1.0,"think_secs":5,"explain":false,"shuffle":false,"positions":{"a":7,"mistakes":3}}"#;
    assert_eq!(serde_json::to_string(&s).unwrap(), json);
    assert_eq!(serde_json::from_str::<Settings>(json).unwrap(), s);
    // 缺字段补默认值（struct 级 `default`）。
    assert_eq!(
      serde_json::from_str::<Settings>("{}").unwrap(),
      Settings::default()
    );
    // 数据库的 CHECK 与 `as_str` 同一个写法，它必须与序列化一致。
    for source in Source::ALL {
      assert_eq!(
        serde_json::to_string(&source).unwrap(),
        format!("\"{}\"", source.as_str())
      );
    }
  }
}

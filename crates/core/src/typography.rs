//! 题库文本的排版归一化。
//!
//! 题面来自上游 CSV，同一道题在不同题库里的空格写法并不一致（「25 瓦」与「25瓦」、
//! 「13.8/Ｎ」与「13.8 / Ｎ」），于是同一道题会得到不同指纹，进而在解析表里留下多份
//! 内容不同、甚至互相矛盾的解析，练习模式的「只看本类新增」也会重复出题。
//! 因此在**生成题库数据时**统一排版，让同一道题的文本逐字节一致。
//!
//! 规则（保守、幂等，只统一格式，不改动措辞）：
//!
//! 1. 全角 ASCII（Ａ-Ｚ、ａ-ｚ、０-９）转半角；
//! 2. 连续空白折叠为一个半角空格，去掉首尾；
//! 3. 中文与 ASCII 字母/数字之间补一个空格（「使用 FT8 模式」「25 瓦」）；
//! 4. 中文与全角标点之间不留空格（「哪些“ITU 分区”」）；
//! 5. 斜杠两侧不留空格（「13.8/Ｎ」）；
//! 6. 最后套用 [`TEXT_FIXES`]，修掉上游把英文单词粘在一起的已知错误。
//!
//! 刻意**不**处理的：中文标点与英文标点的选用、英文内部的逗号空格、公式里 `=` `+` `×`
//! 周围的空格 —— 这些在题库里本就两种写法并存，强行统一会改动大量与本次问题无关的文本。

use crate::question::QuestionItem;
use crate::text::is_js_whitespace;

/// 上游已知的文本错误：英文单词被粘在一起、日期缺空格等。键为错误写法，值为修正写法。
///
/// 这些不是空格排版问题（把 `andstanding by` 拆回 `and standing by` 需要知道词边界），
/// 只能逐条登记。数据重新生成时同样会套用，保证 `cargo make dataset` 结果稳定。
pub const TEXT_FIXES: &[(&str, &str)] = &[
  ("andstanding by", "and standing by"),
  ("standingby", "standing by"),
  ("callingyou", "calling you"),
  ("Zulu,BH1ZZZ", "Zulu, BH1ZZZ"),
  ("Dosvidaniya", "Do svidaniya"),
  ("ForwardError Correction", "Forward Error Correction"),
  ("Feb2012", "Feb 2012"),
  ("Feb2018", "Feb 2018"),
  ("14Feb 2018", "14 Feb 2018"),
  ("L =32.4 +20log(d) + 20log(f)", "L=32.4+20log(d)+20log(f)"),
  ("L =32.4 + 20log(d) +20log(f)", "L=32.4+20log(d)+20log(f)"),
];

/// 汉字区间（含扩展 A 与兼容表意文字）。
fn is_han(c: char) -> bool {
  matches!(c, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{f900}'..='\u{faff}')
}

/// 全角标点：与中文相邻时不留空格。
fn is_full_width_punct(c: char) -> bool {
  "，。、：；！？（）「」『』【】〔〕《》〈〉“”‘’…—～·￥".contains(c)
}

/// 全角 ASCII → 半角。
fn to_half_width(c: char) -> char {
  match c {
    '０'..='９' | 'Ａ'..='Ｚ' | 'ａ'..='ｚ' => {
      char::from_u32(u32::from(c) - 0xFEE0).unwrap_or(c)
    }
    _ => c,
  }
}

/// 只做排版规则化，不套 [`TEXT_FIXES`]（内部与测试用）。
#[must_use]
pub fn normalize_text(s: &str) -> String {
  // 1. 全角 ASCII 转半角
  let half: String = s.chars().map(to_half_width).collect();

  // 2. 空白折叠
  let mut out = String::with_capacity(half.len());
  let mut pending_space = false;
  for c in half.chars() {
    if is_js_whitespace(c) {
      pending_space = !out.is_empty();
    } else {
      if pending_space && !out.is_empty() {
        out.push(' ');
      }
      pending_space = false;
      out.push(c);
    }
  }

  // 3~5. 逐字符重建：原有的空格按规则保留或去掉，中文与 ASCII 相邻时补一个空格
  let mut spaced = String::with_capacity(out.len() + out.len() / 8);
  let mut pending_space = false;
  for c in out.chars() {
    if c == ' ' {
      pending_space = true;
      continue;
    }
    if let Some(prev) = spaced.chars().next_back() {
      if pending_space {
        if no_space_pair(prev, c) {
          // 原空格多余：丢掉
        } else {
          spaced.push(' ');
        }
      } else if need_space_pair(prev, c) {
        spaced.push(' ');
      }
    }
    pending_space = false;
    spaced.push(c);
  }
  spaced
}

/// 中文与 ASCII 字母/数字相邻时需要补空格。
fn need_space_pair(prev: char, next: char) -> bool {
  (is_han(prev) && next.is_ascii_alphanumeric()) || (prev.is_ascii_alphanumeric() && is_han(next))
}

/// 这对字符之间不应有空格。
fn no_space_pair(prev: char, next: char) -> bool {
  (is_han(prev) && is_full_width_punct(next))
    || (is_full_width_punct(prev) && is_han(next))
    || prev == '/'
    || next == '/'
}

/// 套用已知文本修正。
#[must_use]
pub fn apply_known_fixes(s: &str) -> String {
  let mut out = s.to_owned();
  for (from, to) in TEXT_FIXES {
    if out.contains(from) {
      out = out.replace(from, to);
    }
  }
  out
}

/// 题库文本的对外入口：先套已知修正，再做排版规则化。
///
/// 幂等：对已归一化的文本再调用不会产生变化。
#[must_use]
pub fn normalize(s: &str) -> String {
  normalize_text(&apply_known_fixes(s))
}

/// 归一化整道题（题干与选项）；返回是否有改动。
pub fn normalize_question(q: &mut QuestionItem) -> bool {
  let before = clone_texts(q);
  q.question = normalize(&q.question);
  for opt in &mut q.options {
    opt.text = normalize(&opt.text);
  }
  before != clone_texts(q)
}

fn clone_texts(q: &QuestionItem) -> (String, Vec<String>) {
  (
    q.question.clone(),
    q.options.iter().map(|o| o.text.clone()).collect(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn adds_space_between_cjk_and_ascii() {
    assert_eq!(normalize_text("使用 FT8模式"), "使用 FT8 模式");
    assert_eq!(normalize_text("约5瓦"), "约 5 瓦");
    assert_eq!(
      normalize_text("自 2024年 3月 1日起施行"),
      "自 2024 年 3 月 1 日起施行"
    );
    assert_eq!(normalize_text("13.8V车载台"), "13.8V 车载台");
    // 单位与数字保持紧贴
    assert_eq!(normalize_text("50Ω负载"), "50Ω负载");
    assert_eq!(normalize_text("SWR=1.5时"), "SWR=1.5 时");
  }

  #[test]
  fn tightens_full_width_punct_and_slash() {
    assert_eq!(
      normalize_text("以下哪些 “ITU 分区”位于"),
      "以下哪些“ITU 分区”位于"
    );
    assert_eq!(
      normalize_text("0.0768 /N（千瓦小时）"),
      "0.0768/N（千瓦小时）"
    );
    assert_eq!(normalize_text("共 3 题（A）"), "共 3 题（A）");
  }

  #[test]
  fn converts_full_width_ascii() {
    assert_eq!(normalize_text("0.091×Ｎ（安）"), "0.091×N（安）");
    assert_eq!(normalize_text("功率Ｐ"), "功率 P");
  }

  #[test]
  fn keeps_english_punctuation_spacing() {
    assert_eq!(
      normalize_text("CQ CQ CQ, this is BH1ZZZ."),
      "CQ CQ CQ, this is BH1ZZZ."
    );
  }

  #[test]
  fn applies_known_fixes_and_is_idempotent() {
    assert_eq!(
      normalize("calling CQ andstanding by"),
      "calling CQ and standing by"
    );
    assert_eq!(
      normalize("（ForwardError Correction，FEC）"),
      "（Forward Error Correction，FEC）"
    );
    for sample in [
      "使用 FT8模式",
      "0.091×Ｎ（安）",
      "calling CQ andstanding by",
      "以下哪些 “ITU 分区”位于",
    ] {
      let once = normalize(sample);
      assert_eq!(normalize(&once), once, "幂等失败：{sample}");
    }
  }
}

//! CW 解码：把「有音 / 无音」的持续时间序列还原为文字，点长随发报速度自适应。
//!
//! 输入由前端的音调检测给出：每次音调开始或结束时调用 [`Decoder::key`]，
//! 静默期间周期性调用 [`Decoder::idle`]，以便在下一个音到来前就完成字符与单词的切分。

use crate::morse::{DIGITS, LETTERS, PUNCTUATION};

/// 初始点长（毫秒），对应约 20 WPM。
const INITIAL_DIT_MS: f64 = 60.0;
/// 点长允许范围：约 5–50 WPM。
const MIN_DIT_MS: f64 = 24.0;
const MAX_DIT_MS: f64 = 240.0;
/// 新样本在点长估计中的权重。
const ALPHA: f64 = 0.3;
/// 短于该值的音视为毛刺（毫秒）。
const GLITCH_MS: f64 = 12.0;
/// 无法识别的点划序列显示为该字符。
pub const UNKNOWN: char = '*';
/// 输出文本最大长度，超出后丢弃最早的部分。
const MAX_TEXT: usize = 2000;

/// 点划序列 → 字符。
#[must_use]
pub fn char_of(code: &str) -> Option<&'static str> {
  LETTERS
    .iter()
    .chain(DIGITS)
    .chain(PUNCTUATION)
    .find(|m| m.code == code)
    .map(|m| m.ch)
}

/// 自适应 CW 解码器。
#[derive(Debug, Clone)]
pub struct Decoder {
  dit_ms: f64,
  symbol: String,
  text: String,
}

impl Default for Decoder {
  fn default() -> Self {
    Self {
      dit_ms: INITIAL_DIT_MS,
      symbol: String::new(),
      text: String::new(),
    }
  }
}

impl Decoder {
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// 当前点长估计（毫秒）。
  #[must_use]
  pub const fn dit_ms(&self) -> f64 {
    self.dit_ms
  }

  /// 估计的发报速度（PARIS 标准：WPM = 1200 / 点长毫秒）。
  #[must_use]
  pub fn wpm(&self) -> f64 {
    1200.0 / self.dit_ms
  }

  /// 已解码文本。
  #[must_use]
  pub fn text(&self) -> &str {
    &self.text
  }

  /// 当前正在接收、尚未成字的点划。
  #[must_use]
  pub fn pending(&self) -> &str {
    &self.symbol
  }

  pub fn clear(&mut self) {
    self.symbol.clear();
    self.text.clear();
  }

  /// 一段音（`on = true`）或静默（`on = false`）结束，持续 `ms` 毫秒。
  pub fn key(&mut self, on: bool, ms: f64) {
    if on {
      self.mark(ms);
    } else {
      self.idle(ms);
    }
  }

  fn mark(&mut self, ms: f64) {
    if ms < GLITCH_MS {
      return;
    }
    // 介于点与划之间（2 个点长）为分界
    let (sym, sample) = if ms < self.dit_ms * 2.0 {
      ('.', ms)
    } else {
      ('-', ms / 3.0)
    };
    self.symbol.push(sym);
    self.dit_ms = (self.dit_ms * (1.0 - ALPHA) + sample * ALPHA).clamp(MIN_DIT_MS, MAX_DIT_MS);
    // 超长序列多半是噪声，直接作废
    if self.symbol.len() > 8 {
      self.symbol.clear();
      self.push(UNKNOWN);
    }
  }

  /// 已静默 `ms` 毫秒：超过字符间隔（标准 3 个点长）则成字，超过单词间隔（标准 7 个点长）则补空格。
  pub fn idle(&mut self, ms: f64) {
    if ms >= self.dit_ms * 2.5 && !self.symbol.is_empty() {
      let ch = char_of(&self.symbol);
      self.symbol.clear();
      match ch {
        Some(s) => s.chars().for_each(|c| self.push(c)),
        None => self.push(UNKNOWN),
      }
    }
    if ms >= self.dit_ms * 5.0 && !self.text.is_empty() && !self.text.ends_with(' ') {
      self.push(' ');
    }
  }

  fn push(&mut self, c: char) {
    self.text.push(c);
    if self.text.len() > MAX_TEXT {
      let cut = self.text.len() - MAX_TEXT;
      let at = (cut..self.text.len())
        .find(|&i| self.text.is_char_boundary(i))
        .unwrap_or(0);
      self.text.drain(..at);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::morse::code_of;

  /// 按标准时值把文本编码成 (有音?, 毫秒) 序列，可对时值整体加抖动。
  fn keying(text: &str, dit: f64, jitter: impl Fn(usize) -> f64) -> Vec<(bool, f64)> {
    let mut out = Vec::new();
    let mut n = 0;
    let mut j = || {
      n += 1;
      jitter(n)
    };
    for (wi, word) in text.split(' ').enumerate() {
      if wi > 0 {
        out.push((false, dit * 7.0 * j()));
      }
      for (ci, ch) in word.chars().enumerate() {
        if ci > 0 {
          out.push((false, dit * 3.0 * j()));
        }
        for (si, s) in code_of(ch).unwrap().chars().enumerate() {
          if si > 0 {
            out.push((false, dit * j()));
          }
          out.push((true, if s == '.' { dit } else { dit * 3.0 } * j()));
        }
      }
    }
    out.push((false, dit * 10.0));
    out
  }

  fn decode(seq: &[(bool, f64)]) -> Decoder {
    let mut d = Decoder::new();
    for &(on, ms) in seq {
      d.key(on, ms);
    }
    d
  }

  #[test]
  fn decodes_at_initial_speed() {
    let d = decode(&keying("CQ CQ DE BG4XXX", 60.0, |_| 1.0));
    assert_eq!(d.text().trim_end(), "CQ CQ DE BG4XXX");
  }

  #[test]
  fn adapts_to_slow_and_fast_senders() {
    // 12 WPM 与 30 WPM，前几个字符允许因适应而出错
    for dit in [100.0, 40.0] {
      let d = decode(&keying("TEST TEST PARIS PARIS 5NN TU", dit, |_| 1.0));
      assert!(
        d.text().trim_end().ends_with("PARIS PARIS 5NN TU"),
        "dit {dit}: {}",
        d.text()
      );
      assert!(
        (d.wpm() - 1200.0 / dit).abs() < 2.0,
        "dit {dit}: wpm {}",
        d.wpm()
      );
    }
  }

  #[test]
  fn tolerates_hand_keying_jitter() {
    let jitter = |n: usize| [1.0, 1.2, 0.85, 1.1, 0.9][n % 5];
    let d = decode(&keying("73 GL OM", 60.0, jitter));
    assert_eq!(d.text().trim_end(), "73 GL OM");
  }

  #[test]
  fn idle_splits_before_next_tone_and_ignores_glitches() {
    let mut d = Decoder::new();
    d.key(true, 60.0);
    d.key(true, 5.0);
    assert_eq!(d.pending(), ".");
    d.idle(100.0);
    assert_eq!(d.pending(), ".");
    d.idle(200.0);
    assert_eq!(d.text(), "E");
    d.idle(500.0);
    assert_eq!(d.text(), "E ");
    d.idle(900.0);
    assert_eq!(d.text(), "E ");
    for _ in 0..9 {
      d.key(true, 60.0);
    }
    assert_eq!(d.text(), "E *");
  }
}

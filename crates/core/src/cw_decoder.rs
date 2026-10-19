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
/// 峰值至少高出底噪这么多倍才认为有信号（与麦克风侧的门限同一口径）。
const MIN_SNR: f32 = 4.0;

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

/// 自适应门限的包络检测：跟踪底噪与峰值，取二者之间带回差的门限。
///
/// 回差（按下 0.5、松开 0.35）让门限附近的抖动只翻转一次，噪声不会被切成碎点。
#[derive(Debug, Clone, Default)]
pub struct Envelope {
  floor: f32,
  peak: f32,
  on: bool,
  /// 是否已收到第一块。第一块可能是信号本身，不能拿它当底噪。
  primed: bool,
}

impl Envelope {
  /// 送入一个电平样本，返回更新后的按键状态。
  pub fn update(&mut self, level: f32) -> bool {
    // 包络电平语义上非负（幅度 / 能量）；负值（某些前级检波的瞬态）会让 `MIN_SNR` 的乘法
    // 比较方向失真（`-1 > -4` 恒真，把「峰值高于底噪 4 倍」这一判定变成恒成立），先归零。
    let level = level.max(0.0);
    if self.primed {
      if level < self.floor {
        self.floor = level;
      } else {
        self.floor += (level - self.floor) * 0.002;
      }
    } else {
      self.primed = true;
      // 首块同时当底噪与峰值：`span == 0` 保证这一块一定判「没按下」，最多丢掉首个音
      // 起始沿的几毫秒。反过来把底噪钉在 0 会让 `MIN_SNR` 恒成立、门限变成 `0.5 × 峰值`，
      // 于是「开始监听」这件本身就等于按下——底噪平稳时会一直按住到第一个真实间隙，
      // 那一段跨秒的电平被当成一个划，`dit_ms` 还被推到 240 ms 上限。
      self.floor = level;
      self.peak = level;
    }
    if level > self.peak {
      self.peak = level;
    } else {
      self.peak -= (self.peak - self.floor) * 0.0015;
    }
    let span = self.peak - self.floor;
    let threshold = self.floor + span * if self.on { 0.35 } else { 0.5 };
    self.on = self.peak > self.floor * MIN_SNR && level > threshold;
    self.on
  }

  /// 当前按键状态。
  #[must_use]
  pub const fn on(&self) -> bool {
    self.on
  }

  /// 当前电平在 [底噪, 峰值] 中的相对位置，用于电平表。
  #[must_use]
  pub fn relative(&self, level: f32) -> f32 {
    let span = self.peak - self.floor;
    if span <= f32::EPSILON {
      0.0
    } else {
      ((level - self.floor) / span).clamp(0.0, 1.0)
    }
  }
}

/// 实时收报：把逐块电平变成按键时序，再交给 [`Decoder`] 还原文字。
#[derive(Debug, Clone, Default)]
pub struct Reception {
  env: Envelope,
  decoder: Decoder,
  /// 已送入的电平总时长（毫秒）。
  elapsed_ms: f64,
  /// 上次按键状态切换的时刻（毫秒）。
  change_ms: f64,
}

impl Reception {
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// 送入一块电平，`block_ms` 是这块电平代表的音频时长。
  ///
  /// 返回本块处理后的按键状态。块时长由音频线程按采样计数切分，因此主线程
  /// 晚收几十块也只是晚处理，点划与间隔的时长仍然准确。
  pub fn push(&mut self, level: f32, block_ms: f64) -> bool {
    self.elapsed_ms += block_ms;
    let was_on = self.env.on();
    let on = self.env.update(level);
    if on != was_on {
      self.decoder.key(was_on, self.elapsed_ms - self.change_ms);
      self.change_ms = self.elapsed_ms;
    }
    if !on {
      // 静默期间持续调用，字符 / 单词切分才会在下一个音到来前完成
      self.decoder.idle(self.elapsed_ms - self.change_ms);
    }
    on
  }

  /// 当前按键状态。
  #[must_use]
  pub const fn keyed(&self) -> bool {
    self.env.on()
  }

  /// 当前电平在 [底噪, 峰值] 中的相对位置，用于电平表。
  #[must_use]
  pub fn relative(&self, level: f32) -> f32 {
    self.env.relative(level)
  }

  /// 已经还原的文本。
  #[must_use]
  pub fn text(&self) -> &str {
    self.decoder.text()
  }

  /// 当前正在接收、尚未成字的点划。
  #[must_use]
  pub fn pending(&self) -> &str {
    self.decoder.pending()
  }

  /// 估计的发报速度（WPM）。
  #[must_use]
  pub fn wpm(&self) -> f64 {
    self.decoder.wpm()
  }

  pub fn clear(&mut self) {
    self.decoder.clear();
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

  /// 把 (有音?, 毫秒) 序列按 5 ms 小片送进实时收报器：有音片给满幅，静默片给 0。
  ///
  /// **不能整段一次 `push`**：静默期间的字符切分靠 `push` 里的 `idle()` 累积，
  /// 一整段静默只会报告一次 0 ms，字永远成不了。
  ///
  /// 开头先送两片静默：真实用法里「点开始监听」总是早于「开始发报」，包络检测靠这几片
  /// 建起底噪。首块没有历史，`span == 0` 时它不可能被判成按键——这是信息上限，不是缺陷；
  /// 反过来把底噪钉在 0 就会让「开始监听」本身等价于按下（见 `Envelope` 的注释）。
  fn feed(r: &mut Reception, seq: &[(bool, f64)]) {
    const BLOCK_MS: f64 = 5.0;
    r.push(0.0, BLOCK_MS);
    r.push(0.0, BLOCK_MS);
    for &(on, ms) in seq {
      let level = if on { 1.0 } else { 0.0 };
      let mut left = ms;
      while left > 0.0 {
        let step = left.min(BLOCK_MS);
        r.push(level, step);
        left -= step;
      }
    }
  }

  #[test]
  fn background_noise_alone_never_latches_the_key() {
    // 底噪钉在 0 时第一块就判「按下」并一直按住，等到真信号来时首个划会长达数秒；
    // 这里要求整段纯底噪始终是松开，且解不出任何字符。
    let mut r = Reception::new();
    for _ in 0..200 {
      r.push(0.02, 5.0);
    }
    assert!(!r.keyed(), "底噪被当成按键按住");
    assert!(r.text().is_empty(), "底噪解出了字符：{}", r.text());
  }

  #[test]
  fn a_tone_after_background_noise_is_still_decoded() {
    // 先有底噪、再发报是正常用法：底噪不能把首个字符吞掉。
    let mut r = Reception::new();
    for _ in 0..40 {
      r.push(0.02, 5.0);
    }
    feed(&mut r, &keying("E", 60.0, |_| 1.0));
    assert_eq!(r.text().trim_end(), "E");
  }

  #[test]
  fn reception_keeps_timing_when_blocks_arrive_late() {
    // 主线程卡顿只会让块晚到，块的时长仍由采样计数给出，解码结果不变
    let mut r = Reception::new();
    feed(&mut r, &keying("PARIS 73", 60.0, |_| 1.0));
    assert_eq!(r.text().trim_end(), "PARIS 73");
    assert!(!r.keyed());
  }

  #[test]
  fn a_jittery_noise_floor_still_decodes_cleanly() {
    // 上面的用例静音恒为 0；真实麦克风有非零且抖动的底噪，音也不是恒定满幅。
    // 门限靠「底噪 / 峰值」自适应，这里钉住它在带噪声时仍能把整串报文解对。
    let mut seed = 7u32;
    let mut noise = move || {
      seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
      f32::from((seed >> 16) as u16) / 65_536.0 - 0.5 // [-0.5, 0.5)
    };
    let mut r = Reception::new();
    // 先听一会底噪建立门限：真实用法里「点开始监听」总是早于「开始发报」。
    for _ in 0..80 {
      r.push(0.03 + 0.02 * noise(), 5.0);
    }
    for (on, ms) in keying("CQ DE BG4XXX", 60.0, |_| 1.0) {
      let mut left = ms;
      while left > 0.0 {
        let step = left.min(5.0);
        let level = if on {
          1.0 + 0.2 * noise()
        } else {
          0.03 + 0.02 * noise()
        };
        r.push(level, step);
        left -= step;
      }
    }
    assert_eq!(r.text().trim_end(), "CQ DE BG4XXX");
    assert!(!r.keyed(), "报文结束后按键应已松开");
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
  fn reception_decodes_level_blocks_with_word_gaps() {
    let mut r = Reception::new();
    feed(&mut r, &keying("CQ CQ DE BG4XXX", 60.0, |_| 1.0));
    // 单词间隔（7 个点长）后的第一个音最容易被切错：它的前导划一旦被吞，
    // 「C」就成了「R」、「B」就成了「S」。
    assert_eq!(r.text().trim_end(), "CQ CQ DE BG4XXX");
    assert!((r.wpm() - 20.0).abs() < 1.0, "wpm {}", r.wpm());
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

  #[test]
  fn negative_levels_do_not_drag_the_floor_negative() {
    let mut env = Envelope::default();
    for _ in 0..20 {
      env.update(0.02);
    }
    // 负电平（前级检波瞬态）本身不该判按键。
    for _ in 0..20 {
      assert!(!env.update(-1.0));
    }
    // 若 floor 被负电平拖到负值，`threshold` 跟着为负，连 0 电平（纯静音）都会误判成按键。
    for _ in 0..5 {
      assert!(!env.update(0.0), "floor 被负电平拖负，静音误判成按键");
    }
  }
}

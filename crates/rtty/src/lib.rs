//! RTTY（无线电传）编码与解码：45.45 波特、170 Hz FSK、ITA2（Baudot）5 位码。
//!
//! 纯 Rust 实现、无外部依赖，可同时用于 WASM 前端与命令行/测试。
//!
//! # 协议速览
//!
//! - 每字符：1 起始位（空号 Space）+ 5 数据位（低位先发）+ 1.5 停止位（Mark）；
//! - 传号频率 = 载波（默认 2125 Hz），空号 Space = 传号 + 170 Hz；
//! - 5 位码经 LTRS / FIGS 两个换挡状态解释（字母表 / 数字符号表）。
//!
//! # 变体说明
//!
//! 字符集采用 ITA2（CCITT-2）商业标准版（FIGS 的 30 号是 `=`）；美军 TTY 变体的
//! 个别符号（如 `+`）不在其列 —— 解码结果以本表为准。

use ham_web_dsp::tone_power;

/// 波特率（符号/秒）。
pub const BAUD: f64 = 45.45;
/// 每符号时长（秒）。
pub const SYMBOL_SECONDS: f64 = 1.0 / BAUD;
/// 传号 / 空号频移（Hz）。
pub const SHIFT_HZ: f64 = 170.0;
/// 停止位时长（符号数）。
pub const STOP_BITS: f64 = 1.5;

/// ITA2 字母表（下标 = 5 位码值，bit0 = 先发的位）。
pub const LTRS: [char; 32] = [
  '\0', 'E', '\n', 'A', ' ', 'S', 'I', 'U', '\r', 'D', 'R', 'J', 'N', 'F', 'C', 'K', 'T', 'Z', 'L',
  'W', 'H', 'Y', 'P', 'Q', 'O', 'B', 'G', '\u{1F}', 'M', 'X', 'V', '\u{1E}',
];

/// ITA2 数字符号表（下标 = 5 位码值；0x1F = FIGS、0x1E = LTRS 换挡码）。
pub const FIGS: [char; 32] = [
  '\0', '3', '\n', '-', ' ', '\'', '8', '7', '\r', '$', '4', '\'', ',', '!', ':', '(', '5', '"',
  ')', '2', '#', '6', '0', '1', '9', '?', '&', '\u{1F}', '.', '/', '=', '\u{1E}',
];

pub const CODE_LTRS: u8 = 31;
pub const CODE_FIGS: u8 = 27;

/// 一段文字 → 传输位流（含起始位与停止位；未知字符跳过）。
///
/// 数字与符号自动切 FIGS 档，字母切回 LTRS 档。
#[must_use]
pub fn encode_text(text: &str) -> Vec<bool> {
  let mut out = Vec::new();
  let mut figs = false;
  for ch in text.to_uppercase().chars() {
    let (code, want_figs) = if let Some(i) = LTRS.iter().position(|&c| c == ch) {
      (i as u8, false)
    } else if let Some(i) = FIGS.iter().position(|&c| c == ch) {
      (i as u8, true)
    } else {
      continue;
    };
    if want_figs != figs {
      out.push(false);
      push_data(&mut out, if want_figs { CODE_FIGS } else { CODE_LTRS });
      push_stop(&mut out);
      figs = want_figs;
    }
    out.push(false);
    push_data(&mut out, code);
    push_stop(&mut out);
  }
  out
}

fn push_data(out: &mut Vec<bool>, code: u8) {
  for bit in 0..5 {
    out.push(code & (1 << bit) != 0);
  }
}

/// 位流里写 1 个停止位；余下 0.5 个停止位由**调制层**补齐
/// （位是离散的，1.5 停止位 = 位流里的 1 位 + 采样层的半符号 Mark）。
fn push_stop(out: &mut Vec<bool>) {
  out.push(true);
}

/// 已同步的位流 → 文本（含换挡状态机）。
///
/// 输入每字符 7 位（1 起始 + 5 数据 + 1 停止，1.5 停止的余下 0.5 位在同步时已剥掉）；
/// 起始位或停止位错得离谱的字符跳过（计一次坏字符，不中断解码）。
#[must_use]
pub fn decode_bits(bits: &[bool]) -> String {
  let mut out = String::new();
  let mut figs = false;
  let mut i = 0usize;
  while i + 7 <= bits.len() {
    let start = bits[i];
    let mut code = 0u8;
    for (bit, &b) in bits[i + 1..i + 6].iter().enumerate() {
      if b {
        code |= 1 << bit;
      }
    }
    let stop = bits[i + 6];
    if start || !stop {
      i += 7;
      continue;
    }
    if code == CODE_LTRS {
      figs = false;
    } else if code == CODE_FIGS {
      figs = true;
    } else {
      let table = if figs { FIGS } else { LTRS };
      let ch = table[code as usize];
      if ch != '\0' {
        out.push(ch);
      }
    }
    i += 7;
  }
  out
}

/// 解调所需的最低采样率：45.45 波特下每符号至少 1 个样本。
const MIN_RATE: u32 = 46;

/// 音频样本 → 逐符号判定的位流（1 = Mark，0 = Space）。
///
/// 符号窗口 = `rate / 45.45`（非整采样率按四舍五入的边界累进，不漂移）。
#[must_use]
pub fn demodulate(samples: &[f32], rate: u32, mark_hz: f64) -> Vec<bool> {
  // rate=0 → spb=0 → `start += 0` 死循环；`rate < 46` 时 spb < 1 只产生空窗口，输出退化成全 Mark
  // 的纯 CPU 浪费；非有限 / 非正频率让 Goertzel 全 NaN 或判别无意义（worker 层的校验不能
  // 代替公共 API 自己的守卫）。
  if rate < MIN_RATE || samples.is_empty() || !mark_hz.is_finite() || mark_hz <= 0.0 {
    return Vec::new();
  }
  let spb = f64::from(rate) / BAUD;
  let space_hz = mark_hz + SHIFT_HZ;
  let mut out = Vec::new();
  let mut start = 0.0f64;
  while (start + spb) as usize <= samples.len() {
    let from = start.round() as usize;
    let to = (start + spb).round() as usize;
    let window = &samples[from..to.min(samples.len())];
    let mark = tone_power(window, rate, mark_hz);
    let space = tone_power(window, rate, space_hz);
    out.push(mark >= space);
    start += spb;
  }
  out
}

/// 一个字符的「起始位 = Space、停止位 = Mark」匹配度，范围约 `[-2, 2]`（2 = 完全吻合）。
///
/// 各取 `(Space − Mark) / (Space + Mark)` 的归一化差，所以与信号强弱无关；静音 / 噪声里两项
/// 都接近 0，不会产生有意义的偏好。
fn frame_margin(
  samples: &[f32],
  rate: u32,
  mark_hz: f64,
  space_hz: f64,
  spb: f64,
  base: f64,
) -> f64 {
  let norm = |from: f64| -> f64 {
    let a = from.round() as usize;
    let b = (from + spb).round() as usize;
    let w = &samples[a..b];
    let (m, s) = (tone_power(w, rate, mark_hz), tone_power(w, rate, space_hz));
    (s - m) / (s + m + f64::EPSILON)
  };
  // 起始位应是 Space（+），停止位应是 Mark（−）。
  norm(base) - norm(base + 6.0 * spb)
}

/// 在预测起点 `base` 两侧各一小步内，找使字符帧匹配度最高的起点，返回修正后的起点。
///
/// 步长取 `spb/16`，修正只在匹配度提升超过 [`TRACK_MIN_GAIN`] 时才生效：静音与噪声里各候选
/// 的匹配度几乎相同，不会因此让起点随机游走。越界的候选直接跳过（只比 0 偏移）。
fn track_char(samples: &[f32], rate: u32, mark_hz: f64, space_hz: f64, spb: f64, base: f64) -> f64 {
  let step = (spb / 16.0).max(1.0);
  let fits = |b: f64| b >= 0.0 && (b + 7.0 * spb).round() as usize <= samples.len();
  if !fits(base) {
    return base;
  }
  let margin = |b: f64| frame_margin(samples, rate, mark_hz, space_hz, spb, b);
  let here = margin(base);
  let mut best = (base, here);
  for cand in [base - step, base + step] {
    if fits(cand) {
      let m = margin(cand);
      if m > best.1 {
        best = (cand, m);
      }
    }
  }
  if best.1 - here > TRACK_MIN_GAIN {
    best.0
  } else {
    base
  }
}

/// 时钟跟踪的最小匹配度增益：低于它视为噪声，不移动起点。
const TRACK_MIN_GAIN: f64 = 0.02;

/// 完整解码：起始位同步搜索 + 逐位解调 + 换挡状态机。
///
/// 同步方式：把字符网格（每字符 7.5 个符号）在**两个字符周期**内按 1/4 符号步进滑窗，
/// 每个位窗口**直接按样本位置解调**（不经过整数符号网格 —— 1.5 停止位会让位边界落在
/// 半个符号上，先量化成符号再解码会把跨边界的位读错）。取「起始位与停止位都有效」最多
/// 的偏移；坏字符惩罚 2 分，所以乱对齐的偏移赢不了正确对齐。
///
/// 返回 `None` 表示没有解出像样的文本：判定要求锁定点之后有足够的合法字符且占多数
/// （见 [`SyncStats::decodes_text`]），白噪声与静音都落在这个门限之外。
#[must_use]
pub fn decode_audio(samples: &[f32], rate: u32, mark_hz: f64) -> Option<String> {
  if rate < MIN_RATE || samples.is_empty() || !mark_hz.is_finite() || mark_hz <= 0.0 {
    return None;
  }
  let spb = f64::from(rate) / BAUD;
  let space_hz = mark_hz + SHIFT_HZ;
  let per_char = 7.5 * spb;
  let scan = (2.0 * per_char).ceil() as usize; // 两个字符周期内的样本范围
  let step = (spb / 4.0).max(1.0) as usize; // 1/4 符号步进：够对齐，又不至于太慢
  let mut best: Option<(usize, usize, SyncStats, String)> = None;
  let mut offset = 0usize;
  while offset < scan.min(samples.len()) {
    let mut text = String::new();
    let mut figs = false;
    let mut good = 0usize;
    let mut stats = SyncStats::default();
    let mut k = 0usize;
    // 字符网格的当前预测起点：每个字符结束后前进 `per_char`，并按「起始位 / 停止位」的
    // 匹配度做微调 —— 固定网格只在发射机与声卡时钟严格一致时成立，实测 100 ppm 就开始
    // 丢字符、200 ppm 起整段失步（见 `tracks_clock_drift_*` 测试）。
    let mut base = offset as f64;
    loop {
      base = track_char(samples, rate, mark_hz, space_hz, spb, base);
      let end = (base + 7.0 * spb).round() as usize;
      if end > samples.len() {
        break;
      }
      // 第 j 位：窗口 [base + j·spb, base + (j+1)·spb)，直接对样本解调。
      let bit = |j: usize| -> bool {
        let a = (base + j as f64 * spb).round() as usize;
        let b = (base + (j + 1) as f64 * spb).round() as usize;
        let w = &samples[a..b];
        tone_power(w, rate, mark_hz) >= tone_power(w, rate, space_hz)
      };
      let start = bit(0);
      let mut code = 0u8;
      for j in 1..=5 {
        if bit(j) {
          code |= 1 << (j - 1);
        }
      }
      let stop = bit(6);
      if !start && stop {
        good += 1;
        if stats.valid == 0 {
          stats.before_first = k;
        }
        stats.valid += 1;
        if code == CODE_LTRS {
          figs = false;
        } else if code == CODE_FIGS {
          figs = true;
        } else {
          let ch = (if figs { FIGS } else { LTRS })[code as usize];
          if ch != '\0' {
            text.push(ch);
          }
        }
      } else {
        // 坏字符惩罚 2 分：乱对齐的偏移赢不了正确对齐。
        good = good.saturating_sub(2);
        stats.invalid += 1;
      }
      k += 1;
      base += per_char;
    }
    stats.total = k;
    if best.as_ref().is_none_or(|(_, b, _, _)| good > *b) {
      best = Some((offset, good, stats, text));
    }
    offset += step;
  }
  best.and_then(|(_, _, stats, text)| (stats.decodes_text() && !text.is_empty()).then_some(text))
}

/// 一个候选偏移的同步质量。
#[derive(Clone, Copy, Default)]
struct SyncStats {
  /// 通过「起始位 = Space、停止位 = Mark」检验的字符数。
  valid: usize,
  /// 未通过检验的窗口数。
  invalid: usize,
  /// 首个合法字符之前的窗口数（前导静音或未对齐段）。
  before_first: usize,
  /// 该偏移下尝试过的窗口总数。
  total: usize,
}

/// 解码器接受一个偏移所需的最少合法字符数。
///
/// 白噪声里「起始位 Space + 停止位 Mark」是随机命中（约 1/4），要靠连续结构才能
/// 与真信号区分；6 个合法字符已足够排除噪声（噪声连成 6 个的概率极低），也不会
/// 把常见的中短报文判死。
const MIN_VALID_CHARS: usize = 6;

impl SyncStats {
  /// 是否够格算「解出了文本」。
  ///
  /// 比例只在**锁定点之后**统计：前导静音与未对齐窗口不参与，否则真实录音开头的
  /// 长静音会把好信号判死。白噪声既攒不出 [`MIN_VALID_CHARS`] 个合法字符，也占不到
  /// 锁定后窗口的三分之二 —— 旧判据 `good > 0` 正是把这类乱码当成了解码结果。
  fn decodes_text(self) -> bool {
    let span = self.total.saturating_sub(self.before_first);
    self.valid >= MIN_VALID_CHARS && self.valid * 3 >= span * 2
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 带时钟误差的独立合成器（与库里的编码 / 解码不共享代码）：每个符号 `rate/45.45 × (1+ppm)`
  /// 个样本，相位连续。`ppm > 0` = 接收端采样时钟比发射端慢，符号在录音里更长。
  fn synth(text: &str, rate: u32, mark: f64, ppm: f64, noise: f32) -> Vec<f32> {
    let bits = encode_text(text);
    let spb = f64::from(rate) / BAUD * (1.0 + ppm * 1e-6);
    let (mut out, mut phase, mut acc) = (Vec::new(), 0.0f64, 0.0f64);
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    let mut push = |out: &mut Vec<f32>, f: f64, len: f64, acc: &mut f64| {
      *acc += len;
      let n = acc.round() as usize;
      *acc -= n as f64;
      for _ in 0..n {
        phase += 2.0 * std::f64::consts::PI * f / f64::from(rate);
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let r = (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5;
        out.push(phase.sin() as f32 + noise * r as f32);
      }
    };
    let mut i = 0;
    while i + 7 <= bits.len() {
      for &b in &bits[i..i + 7] {
        push(
          &mut out,
          if b { mark } else { mark + SHIFT_HZ },
          spb,
          &mut acc,
        );
      }
      push(&mut out, mark, spb / 2.0, &mut acc); // 1.5 停止位的余下 0.5
      i += 7;
    }
    out
  }

  /// 期望文本中被正确还原的比例（按最长公共子序列，容忍个别丢 / 错字符）。
  fn match_ratio(expect: &str, got: &str) -> f64 {
    let (a, b): (Vec<char>, Vec<char>) = (expect.chars().collect(), got.chars().collect());
    let mut row = vec![0usize; b.len() + 1];
    for &x in &a {
      let mut diag = 0;
      for (j, &y) in b.iter().enumerate() {
        let up = row[j + 1];
        row[j + 1] = if x == y {
          diag + 1
        } else {
          row[j + 1].max(row[j])
        };
        diag = up;
      }
    }
    row[b.len()] as f64 / a.len().max(1) as f64
  }

  #[test]
  fn tracks_clock_drift_over_a_long_message() {
    // 约 3.6 分钟正文（1320 字符）。修复前：100 ppm 总体 0.98、200 ppm 0.61、≥ 500 ppm 返回 None。
    let text = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789 ".repeat(24);
    for ppm in [0.0, 100.0, 200.0, 500.0, -500.0, 1000.0] {
      let audio = synth(&text, 12000, 2125.0, ppm, 0.0);
      let got = decode_audio(&audio, 12000, 2125.0)
        .unwrap_or_else(|| panic!("{ppm} ppm：整段被判成无信号"));
      let ratio = match_ratio(&text, &got);
      assert!(ratio >= 0.97, "{ppm} ppm：只还原了 {:.1}%", ratio * 100.0);
    }
  }

  #[test]
  fn drift_tracking_survives_noise_and_a_low_sample_rate() {
    let text = "CQ CQ DE BG1AAA 599 K ".repeat(20);
    for (rate, ppm, noise) in [
      (8000, 300.0, 0.4f32),
      (12000, -300.0, 0.4),
      (48000, 500.0, 0.4),
    ] {
      let audio = synth(&text, rate, 2125.0, ppm, noise);
      let got = decode_audio(&audio, rate, 2125.0)
        .unwrap_or_else(|| panic!("{rate} Hz {ppm} ppm 带噪：被判成无信号"));
      assert!(
        match_ratio(&text, &got) >= 0.95,
        "{rate} Hz {ppm} ppm: {got}"
      );
    }
  }

  #[test]
  fn tracking_does_not_turn_silence_or_noise_into_text() {
    assert!(decode_audio(&vec![0.0f32; 12000 * 30], 12000, 2125.0).is_none());
    let mut seed = 0x1234_5678_9ABC_DEF1u64;
    let noise: Vec<f32> = (0..12000 * 30)
      .map(|_| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        ((seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5) as f32
      })
      .collect();
    assert!(decode_audio(&noise, 12000, 2125.0).is_none());
  }

  #[test]
  fn demodulate_rejects_unusable_rate_and_mark() {
    // R2-19：lib 层自己守卫，不依赖 worker 的参数校验。
    let samples = vec![0.5f32; 4096];
    for rate in [0, 1, 45] {
      assert!(demodulate(&samples, rate, 2125.0).is_empty(), "rate={rate}");
      assert!(decode_audio(&samples, rate, 2125.0).is_none());
    }
    for mark in [-2125.0, 0.0, f64::NAN, f64::INFINITY] {
      assert!(demodulate(&samples, 8000, mark).is_empty(), "mark={mark}");
      assert!(decode_audio(&samples, 8000, mark).is_none());
    }
  }

  #[test]
  fn ita2_tables_are_self_consistent() {
    // 两张表同下标互斥的换挡码位置一致；字母表 26 个字母齐全。
    assert_eq!(
      CODE_LTRS as usize,
      LTRS.iter().position(|&c| c == '\u{1E}').unwrap()
    );
    assert_eq!(
      CODE_FIGS as usize,
      LTRS.iter().position(|&c| c == '\u{1F}').unwrap()
    );
    for ch in 'A'..='Z' {
      assert!(LTRS.contains(&ch), "缺少 {ch}");
    }
    for ch in '0'..='9' {
      assert!(FIGS.contains(&ch), "缺少 {ch}");
    }
  }

  #[test]
  fn text_round_trips_through_bits() {
    let text = "CQ CQ DE BG1AAA 599 K";
    let bits = encode_text(text);
    assert_eq!(decode_bits(&bits), text, "位流往返");
  }

  #[test]
  fn standard_start_and_stop_bits_are_required() {
    let a = [false, true, true, false, false, false, true];
    assert_eq!(decode_bits(&a), "A");
    assert_eq!(encode_text("A"), a);
    let mut bad_start = a;
    bad_start[0] = true;
    assert!(decode_bits(&bad_start).is_empty());
    let mut bad_stop = a;
    bad_stop[6] = false;
    assert!(decode_bits(&bad_stop).is_empty());
  }

  #[test]
  fn demod_discriminates_mark_and_space_with_noise() {
    // 合成一段 12000 Hz 的 FSK：Mark 2125 / Space 2295，含白噪声。
    let rate = 12_000u32;
    let spb = (f64::from(rate) / BAUD).round() as usize;
    let mark = 2125.0f64;
    let space = mark + SHIFT_HZ;
    let mut samples = Vec::new();
    let mut seed = 0x1234_5678u32;
    for i in 0..(spb * 20) {
      let is_mark = (i / spb).is_multiple_of(2);
      let f = if is_mark { mark } else { space };
      let t = i as f64 / f64::from(rate);
      let signal = (2.0 * std::f64::consts::PI * f * t).sin() as f32;
      seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
      let noise = ((seed >> 8) as f32 / 16_777_216.0) * 0.06 - 0.03;
      samples.push(signal + noise);
    }
    let bits = demodulate(&samples, rate, mark);
    assert_eq!(bits.len(), 20);
    // 生成器里第 s 个符号是 Mark（s 偶）还是 Space（s 奇）。
    for (s, &b) in bits.iter().enumerate() {
      assert_eq!(b, s % 2 == 0, "第 {s} 个符号判错");
    }
  }

  #[test]
  fn full_decode_handles_arbitrary_sample_offset() {
    // 生成 "RTTY TEST 599" 的音频：前面垫 3.33 个符号的静音，仍应解出原文。
    // 前导长度刻意**不是**搜索步长（spb/4）的整数倍 —— 否则偏移搜索即使完全不
    // 生效也能解对，用例名与覆盖范围就对不上了。
    let text = "RTTY TEST 599";
    let bits = encode_text(text);
    let rate = 12_000u32;
    let spb = (f64::from(rate) / BAUD).round() as usize;
    let mark = 2125.0f64;
    let space = mark + SHIFT_HZ;
    let mut samples = vec![0.0f32; spb * 3 + spb / 3];
    let push = |samples: &mut Vec<f32>, f: f64, count: usize| {
      for _ in 0..count {
        let t = samples.len() as f64 / f64::from(rate);
        samples.push((2.0 * std::f64::consts::PI * f * t).sin() as f32);
      }
    };
    let half_symbol = (spb as f64 / 2.0).round() as usize;
    let mut bit_idx = 0usize;
    while bit_idx + 7 <= bits.len() {
      // 每字符：1 起始 + 5 数据 + 1 停止（位流）+ 0.5 停止（纯 Mark）。
      for &b in &bits[bit_idx..bit_idx + 7] {
        push(&mut samples, if b { mark } else { space }, spb);
      }
      push(&mut samples, mark, half_symbol);
      bit_idx += 7;
    }
    let decoded = decode_audio(&samples, rate, mark).expect("应当解出文本");
    assert!(decoded.contains("RTTY"), "{decoded}");
    assert!(decoded.contains("TEST"), "{decoded}");
    assert!(decoded.contains("599"), "{decoded}");
  }

  #[test]
  fn silence_yields_nothing() {
    assert!(decode_audio(&[0.0f32; 4096], 12_000, 2125.0).is_none());
  }

  #[test]
  fn noise_yields_nothing() {
    // 白噪声：起始位 / 停止位的随机命中（约 1/4）会攒出零星「合法字符」，
    // 旧判据 `good > 0` 会把这类乱码当成解码结果。
    let rate = 12_000u32;
    for seed0 in [
      0x1234_5678u32,
      0x9E37_79B9,
      0x5BD1_E995,
      0x85EB_CA6B,
      0xC2B2_AE35,
      0x27D4_EB2F,
      0x1656_67B1,
      0xD3A2_6464,
    ] {
      let mut seed = seed0;
      let samples: Vec<f32> = (0..rate * 10)
        .map(|_| {
          seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
          ((seed >> 8) as f32 / 8_388_608.0) - 1.0
        })
        .collect();
      assert!(
        decode_audio(&samples, rate, 2125.0).is_none(),
        "噪声不应解出文本（seed {seed0:#x}）"
      );
    }
  }

  #[test]
  fn silence_preamble_does_not_hide_a_real_signal() {
    // 判据只在锁定点之后统计：真实录音开头有几秒静音不能被判成「没信号」。
    let text = "CQ CQ DE BG1AAA 599 K";
    let bits = encode_text(text);
    let rate = 12_000u32;
    let spb = (f64::from(rate) / BAUD).round() as usize;
    let mark = 2125.0f64;
    let space = mark + SHIFT_HZ;
    let mut audio: Vec<f32> = Vec::new();
    let push = |audio: &mut Vec<f32>, f: f64, count: usize| {
      for _ in 0..count {
        let t = audio.len() as f64 / f64::from(rate);
        audio.push((2.0 * std::f64::consts::PI * f * t).sin() as f32);
      }
    };
    let half_symbol = (spb as f64 / 2.0).round() as usize;
    let mut bit_idx = 0usize;
    while bit_idx + 7 <= bits.len() {
      for &b in &bits[bit_idx..bit_idx + 7] {
        push(&mut audio, if b { mark } else { space }, spb);
      }
      push(&mut audio, mark, half_symbol);
      bit_idx += 7;
    }
    let mut samples = vec![0.0f32; rate as usize * 3];
    samples.extend_from_slice(&audio);
    let decoded = decode_audio(&samples, rate, mark).expect("静音前导后仍应解出文本");
    assert!(decoded.contains("BG1AAA"), "{decoded}");
  }
}

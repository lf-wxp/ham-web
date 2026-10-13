//! RTTY 的独立参考实现（仅对拍用，与 `src/` 不共享任何代码）。
//!
//! 依据 ITA2（CCITT-2）公开标准独立书写：45.45 波特、170 Hz 频移、1 起始位 +
//! 5 数据位（低位先发）+ 1.5 停止位。编解码状态机与解调算法均独立成文 ——
//! 与本仓实现的差异点（码表组织、同步方式、音频合成方式）正是对拍的价值所在。

/// 采样率（合成样本固定 12 kHz，每符号 264 样本）。
pub const SAMPLE_RATE: u32 = 12_000;
/// 传号频率。
pub const MARK: f64 = 2125.0;
/// 频移。
const SHIFT: f64 = 170.0;
/// 波特率。
const BAUD: f64 = 45.45;
/// 每字符位数（1 起始 + 5 数据 + 1 停止，1.5 停止的余下 0.5 在调制层补齐）。
const BITS_PER_CHAR: usize = 7;

/// ITA2 字母表：`[bit0..bit4]` 权值 → 字符（bit0 = 先发的位）。
const LETTERS: [char; 32] = [
  '\0', 'E', '\n', 'A', ' ', 'S', 'I', 'U', '\r', 'D', 'R', 'J', 'N', 'F', 'C', 'K', 'T', 'Z', 'L',
  'W', 'H', 'Y', 'P', 'Q', 'O', 'B', 'G', '\0', 'M', 'X', 'V', '\0',
];

/// ITA2 数字表。
const FIGURES: [char; 32] = [
  '\0', '3', '\n', '-', ' ', '\'', '8', '7', '\r', '$', '4', '\'', ',', '!', ':', '(', '5', '"',
  ')', '2', '#', '6', '0', '1', '9', '?', '&', '\0', '.', '/', '=', '\0',
];

/// FIGS 换挡码（0b11011 = 27）。
const SHIFT_FIGS: u8 = 27;
/// LTRS 换挡码（0b11111 = 31）。
const SHIFT_LTRS: u8 = 31;

/// 文本 → 位流（含换挡码；未知字符跳过）。
fn encode(text: &str) -> Vec<bool> {
  let mut out = Vec::new();
  let mut figs = false;
  for ch in text.to_ascii_uppercase().chars() {
    let (code, next_figs) = if let Some(i) = LETTERS.iter().position(|&c| c == ch) {
      (i as u8, false)
    } else if let Some(i) = FIGURES.iter().position(|&c| c == ch) {
      (i as u8, true)
    } else {
      continue;
    };
    if next_figs != figs {
      let shift_code = if next_figs { SHIFT_FIGS } else { SHIFT_LTRS };
      out.push(false); // Space 起始位
      for bit in 0..5 {
        out.push(shift_code & (1 << bit) != 0);
      }
      out.push(true); // 停止位
      figs = next_figs;
    }
    out.push(false); // Space 起始位
    for bit in 0..5 {
      out.push(code & (1 << bit) != 0);
    }
    out.push(true); // 停止位（1.5 的余下 0.5 在调制层）
  }
  out
}

/// 文本 → 音频样本（相位连续；每 7 位补 0.5 符号的传号尾）。
pub fn synthesize(text: &str) -> Vec<f32> {
  let bits = encode(text);
  let spb = (f64::from(SAMPLE_RATE) / BAUD).round() as usize;
  let half = (spb as f64 / 2.0).round() as usize;
  let mut out = Vec::new();
  let mut phase = 0.0f64;
  let push = |out: &mut Vec<f32>, phase: &mut f64, f: f64, count: usize| {
    for _ in 0..count {
      out.push(phase.sin() as f32);
      *phase += 2.0 * std::f64::consts::PI * f / f64::from(SAMPLE_RATE);
    }
  };
  for chunk in bits.chunks(BITS_PER_CHAR) {
    for &b in chunk {
      push(
        &mut out,
        &mut phase,
        if b { MARK } else { MARK + SHIFT },
        spb,
      );
    }
    push(&mut out, &mut phase, MARK, half);
  }
  out
}

/// 音频 → 文本（对齐已知：合成信号从 0 开始；逐符号 Goertzel 判 Mark / Space，
/// 字符网格 7.5 符号，与本仓实现的「字符网格滑窗同步」是两套独立算法）。
pub fn decode(audio: &[f32]) -> String {
  let spb = (f64::from(SAMPLE_RATE) / BAUD).round() as usize;
  let half = (spb as f64 / 2.0).round() as usize;
  let per_char = 7 * spb + half;
  let mut out = String::new();
  let mut figs = false;
  let mut base = 0usize;
  while base + per_char <= audio.len() {
    let bit = |j: usize| -> bool {
      let a = base + j * spb;
      let b = a + spb;
      power(&audio[a..b.min(audio.len())], MARK)
        >= power(&audio[a..b.min(audio.len())], MARK + SHIFT)
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
      if code == SHIFT_LTRS {
        figs = false;
      } else if code == SHIFT_FIGS {
        figs = true;
      } else {
        let ch = if figs { FIGURES } else { LETTERS }[code as usize];
        if ch != '\0' {
          out.push(ch);
        }
      }
    }
    base += per_char;
  }
  out
}

/// Goertzel 单频功率。
fn power(samples: &[f32], freq: f64) -> f64 {
  let w = 2.0 * std::f64::consts::PI * freq / f64::from(SAMPLE_RATE);
  let coeff = 2.0 * w.cos();
  let mut s0 = 0.0f64;
  let mut s1 = 0.0f64;
  for &x in samples {
    let s = f64::from(x) + coeff * s1 - s0;
    s0 = s1;
    s1 = s;
  }
  s1 * s1 + s0 * s0 - coeff * s1 * s0
}

//! DominoEX 数字模式编解码：IFK+（偏移增量频移键控）+ Nibble Varicode。
//!
//! 算法与参数表转写自 **fldigi**（GPL-3.0）`src/dominoex/`（dominoex.cxx +
//! dominovar.cxx，© Tomi Manninen OH2BNS 等的 DominoEX Varicode；模式设计
//! © Murray Greenman ZL1BPU）—— fldigi 是 DominoEX 的事实参考实现。
//!
//! 结构（以默认的 DominoEX 8 为例）：
//! - 18 个音调、音调间隔 = 波特率 × 2（= 15.625 Hz）、7.8125 波特；
//! - **IFK+**：数据由**相邻音调的频率增量**携带 —— 下一个音调 =
//!   `(当前音调 + 2 + nibble) mod 18`。+2 的固定偏移保证**永不重复同一音调**
//!   （多径自干扰规避 + 同步沿不缺失）；nibble=15 时增量 17 ≡ −1 (mod 18)，
//!   环绕会出现 ±1 的相邻音调，属规格内行为。接收端测频率差、除以间隔、圆整、
//!   减 2，差分测量把调谐误差抵消（规格标称 ±200 Hz 容差）；
//! - 每个字符 1–3 个 nibble（4 位）：首个 nibble MSB = 0、续码 MSB = 1，
//!   由**数据本身**自同步，无需额外同步开销；
//! - FEC 模式（MultiPSK 兼容：与 MFSK16 相同的 K=7 卷积码 + 交织 + IZ8BLY Varicode）
//!   见 [`fec`]；调制完全相同，只是 nibble 的含义变了，所以收发两端必须选同一种。

use ham_web_dsp::tone_power;

pub mod fec;
pub mod mfsk_varicode;
pub mod varicode;

use varicode::{VARICODE, VARIDECODE};

/// 音调数。
pub const NUMTONES: usize = 18;
/// 基频（信号中心）。
pub const BASEFREQ: f64 = 1000.0;

/// 支持的标准子模式：`(名, 波特率, 音调间隔 = 波特率 × doublespaced)`。
pub const MODES: &[(&str, f64, f64)] = &[
  ("4", 3.90625, 7.8125),
  ("5", 5.3833, 10.766),
  ("8", 7.8125, 15.625),
  ("11", 10.766, 10.766),
  ("16", 15.625, 15.625),
  ("22", 21.533, 21.533),
];

/// 解析 `"8"` / `"16"` 形式的子模式名 → `(波特率, 音调间隔)`。
#[must_use]
pub fn parse_mode(name: &str) -> Option<(f64, f64)> {
  MODES
    .iter()
    .find(|(n, _, _)| *n == name.trim())
    .map(|(_, baud, spacing)| (*baud, *spacing))
}

/// 音调 k 的频率（中心 1000 Hz 布局，与 fldigi 的 `sendtone` 一致）。
#[must_use]
pub fn tone_freq(k: usize, spacing: f64) -> f64 {
  BASEFREQ - (NUMTONES as f64) * spacing / 2.0 + (k as f64 + 0.5) * spacing
}

/// 字符 → 发送 nibble 序列（主字母表；尾部 0 不收）。
#[must_use]
pub fn encode_char_nibbles(c: u8) -> Vec<u8> {
  let code = VARICODE[c as usize];
  let mut out = vec![code[0]];
  for &n in &code[1..] {
    if n & 0x8 != 0 {
      out.push(n);
    } else {
      break;
    }
  }
  out
}

/// 文本 → nibble 序列（每字符间由 Varicode 自身的 MSB 结构分隔；`idle` 用 NUL）。
#[must_use]
pub fn encode_text_nibbles(text: &str) -> Vec<u8> {
  let mut out = Vec::new();
  for b in text.bytes() {
    out.extend(encode_char_nibbles(b));
  }
  out
}

/// nibble → 下一个音调号（IFK+：`(prev + 2 + nibble) mod 18`）。
#[must_use]
pub fn next_tone(prev: usize, nibble: u8) -> usize {
  (prev + 2 + usize::from(nibble)) % NUMTONES
}

/// 单个符号的样本数上限（2^24）：`baud` 极小时 `rate / baud` 会饱和成 `usize::MAX`，
/// 之后 `i + symlen` 回绕、切片越界；设上界把它挡在入口。
const MAX_SYMLEN: f64 = 16_777_216.0;

/// 空闲符时长上限（秒）：`idle_secs = 1e300` 曾让空闲符数量饱和成 `usize::MAX` → 无界 push。
const MAX_IDLE_SECS: f64 = 600.0;

/// `encode_audio` 输出样本总数上限（5×10⁷ ≈ 200 MB f32）：`sample_rate = u32::MAX` 时
/// 单个符号就有数亿样本，不设总量上限会在 wasm 里直接 OOM。
const MAX_ENCODE_SAMPLES: usize = 50_000_000;

/// 每个符号的样本数；任何非有限 / 非正 / 越界参数一律 `None`。
fn symbol_len(rate: u32, baud: f64) -> Option<usize> {
  if rate == 0 || !baud.is_finite() || baud <= 0.0 {
    return None;
  }
  let len = (f64::from(rate) / baud).round();
  (1.0..=MAX_SYMLEN).contains(&len).then_some(len as usize)
}

/// 文本 → 音频样本（无前导；相位连续）。`idle_secs` 控制前后补 NUL 空闲符的时长。
///
/// 参数越界（非有限、非正、`idle_secs` 为负或超过 [`MAX_IDLE_SECS`]、输出样本数超过
/// [`MAX_ENCODE_SAMPLES`]）一律返回 `None`，不做静默饱和。
#[must_use]
pub fn encode_audio(
  text: &str,
  baud: f64,
  spacing: f64,
  sample_rate: u32,
  idle_secs: f64,
) -> Option<Vec<f32>> {
  if !spacing.is_finite() || spacing <= 0.0 || !(0.0..=MAX_IDLE_SECS).contains(&idle_secs) {
    return None;
  }
  let symlen = symbol_len(sample_rate, baud)?;
  let idle_chars = (idle_secs * baud / 3.0).round() as usize; // NUL = 3 nibbles
  // 每字符至多 3 个 nibble；按上界估算总样本数，先拒绝再分配。
  let max_nibbles = idle_chars
    .saturating_add(idle_chars.max(4))
    .saturating_add(text.len())
    .saturating_mul(3);
  if max_nibbles.saturating_mul(symlen) > MAX_ENCODE_SAMPLES {
    return None;
  }
  let mut nibbles = Vec::new();
  for _ in 0..idle_chars {
    nibbles.extend(encode_char_nibbles(0));
  }
  nibbles.extend(encode_text_nibbles(text));
  for _ in 0..idle_chars.max(4) {
    nibbles.extend(encode_char_nibbles(0));
  }
  Some(synth(&nibbles, symlen, spacing, sample_rate))
}

/// FEC 模式的文本 → 音频。前后各补二次信道空格：开头让接收端的 Viterbi 收敛，结尾把
/// 交织器（12 个 nibble）与回溯（45 位 ≈ 23 个 nibble）里压着的最后几个字符推出来。
/// 至少各 4 / 10 个，`idle_secs` 更长时按时长补。
///
/// 参数约束与 [`encode_audio`] 相同。
#[must_use]
pub fn encode_audio_fec(
  text: &str,
  baud: f64,
  spacing: f64,
  sample_rate: u32,
  idle_secs: f64,
) -> Option<Vec<f32>> {
  if !spacing.is_finite() || spacing <= 0.0 || !(0.0..=MAX_IDLE_SECS).contains(&idle_secs) {
    return None;
  }
  let symlen = symbol_len(sample_rate, baud)?;
  // 二次信道空格 = 11 位码字 = 22 个码位 = 5.5 个 nibble。
  let idle_chars = (idle_secs * baud / 5.5).round() as usize;
  let (head, tail) = (idle_chars.max(4), idle_chars.max(10));
  // 最长码字 12 位 → 每字符至多 6 个 nibble。
  let max_nibbles = head
    .saturating_add(tail)
    .saturating_add(text.len())
    .saturating_mul(6);
  if max_nibbles.saturating_mul(symlen) > MAX_ENCODE_SAMPLES {
    return None;
  }
  let mut enc = fec::FecEncoder::new();
  enc.clear_bits();
  let mut nibbles = Vec::new();
  for _ in 0..head {
    enc.encode_char(b' ', true, &mut nibbles);
  }
  for b in text.bytes() {
    enc.encode_char(b, false, &mut nibbles);
  }
  for _ in 0..tail {
    enc.encode_char(b' ', true, &mut nibbles);
  }
  Some(synth(&nibbles, symlen, spacing, sample_rate))
}

/// nibble 序列 → IFK+ 音频（相位连续：按全局样本序号取正弦）。
fn synth(nibbles: &[u8], symlen: usize, spacing: f64, sample_rate: u32) -> Vec<f32> {
  let mut out = Vec::with_capacity(nibbles.len() * symlen);
  let mut prev = 0usize;
  for &n in nibbles {
    prev = next_tone(prev, n);
    let f = tone_freq(prev, spacing);
    for _ in 0..symlen {
      out.push(
        (2.0 * std::f64::consts::PI * f * out.len() as f64 / f64::from(sample_rate)).sin() as f32,
      );
    }
  }
  out
}

/// 单个窗口内，音调频带上最强的频率（在 `spacing/4` 网格上搜索）。
fn peak_freq(samples: &[f32], rate: u32, spacing: f64) -> f64 {
  peak_freq_power(samples, rate, spacing).0
}

/// 同 [`peak_freq`]，同时返回该频率的 Goertzel 功率 —— [`sync_offset`] 需要
/// 用功率做对齐打分，一次搜索两种信息都拿走，避免对同一窗口重复 Goertzel
/// （原实现每窗口 ×73 个网格点 ×2 遍，约 6 亿次乘加）。
fn peak_freq_power(samples: &[f32], rate: u32, spacing: f64) -> (f64, f64) {
  let band = NUMTONES as f64 * spacing;
  let lo = BASEFREQ - band / 2.0;
  let hi = BASEFREQ + band / 2.0;
  // 定长网格（`NUMTONES × 4 + 1` = 73 点）而不是 `while f <= hi { f += step }`：
  // `spacing` 极小时 `step` 小于 `ulp(f)/2`，累加对 f64 无效 → 死循环。
  let points = NUMTONES * 4;
  let step = (hi - lo) / points as f64;
  let mut best = (0.0f64, f64::NEG_INFINITY);
  for k in 0..=points {
    let f = lo + k as f64 * step;
    let p = tone_power(samples, rate, f);
    if p > best.1 {
      best = (f, p);
    }
  }
  best
}

/// nibble 累加器：Varicode 自同步解码。
#[derive(Debug, Default)]
pub struct NibbleDecoder {
  /// 已收到的 nibble（最新在前）。
  buf: [u8; 3],
  count: usize,
  /// 主信道文本。
  pub primary: String,
  /// 二次信道（ID 信标）文本。
  pub secondary: String,
}

impl NibbleDecoder {
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// 喂入一个 nibble（0–17，其中 16/17 不是编码数据），返回刚完成的字符（若有）。
  ///
  /// 16/17 只在「相邻两个窗口测到同一音调」（Δf = 0，静音或噪声）时出现，不属于任何
  /// 字符：按字符边界结算已攒满的字符后丢弃。它们的高位若被当成数据进查表，打包值会
  /// 超出 4096 项的表长（旧实现即由此越界）。
  pub fn feed(&mut self, nibble: u8) -> Option<u16> {
    let out = if nibble > 0x0F || nibble & 0x8 == 0 {
      // 新字符开始（或无效 nibble）：先把积攒的旧字符结算掉。
      let done = self.flush();
      self.count = 0;
      done
    } else {
      None
    };
    if nibble <= 0x0F && self.count < 3 {
      self.buf[self.count] = nibble;
      self.count += 1;
    }
    out
  }

  fn flush(&self) -> Option<u16> {
    if self.count == 0 {
      return None;
    }
    // 最新 nibble 在最低 4 位（fldigi 的 symbolbuf 打包顺序）。
    let packed: usize = (0..self.count)
      .map(|i| usize::from(self.buf[self.count - 1 - i]) << (4 * i))
      .sum();
    // 用 `get` 而不是下标：万一以后上游改了取值域，查表越界只会退化成「没有这个
    // 字符」，不能把整条解码链打成 wasm trap（`panic = "abort"` 下没有恢复余地）。
    VARIDECODE
      .get(packed)
      .copied()
      .filter(|&v| v >= 0)
      .map(|v| v as u16)
  }
}

/// 解调：符号流 → nibble 流（IFK+ 差分：`round(Δf/spacing) − 2 mod 18`）。
///
/// 第一个符号没有「前一个」，直接丢弃（Varicode 自同步会自己找齐）。
#[must_use]
pub fn demodulate_nibbles(
  samples: &[f32],
  rate: u32,
  baud: f64,
  spacing: f64,
  offset: usize,
) -> Vec<u8> {
  // rate=0 / baud≤0 → symlen=0 → `i += 0` 死循环；公共 API 防住。
  if !spacing.is_finite() || spacing <= 0.0 {
    return Vec::new();
  }
  let Some(symlen) = symbol_len(rate, baud) else {
    return Vec::new();
  };
  let mut freqs = Vec::new();
  // `offset` 无上界（`usize::MAX`）时 `i + symlen` 会回绕：改用剩余长度比较，
  // 越过末尾的偏移直接得到空切片。
  let mut rest = samples.get(offset..).unwrap_or(&[]);
  while rest.len() >= symlen {
    let (win, tail) = rest.split_at(symlen);
    freqs.push(peak_freq(win, rate, spacing));
    rest = tail;
  }
  let mut out = Vec::new();
  for w in freqs.windows(2) {
    let d = (w[1] - w[0]) / spacing;
    let nibble = (d.round() as i64 - 2).rem_euclid(NUMTONES as i64) as u8;
    out.push(nibble);
  }
  // 第一个差值没有有效的「前一个符号」参照（fldigi 的 prev 起点也是随机的），
  // 它必然是个垃圾 nibble —— 直接丢弃，Varicode 自同步负责后续对齐。
  // 用 drain 而不是 remove(0)（后者对头部删除是 O(n)）。
  if !out.is_empty() {
    out.drain(..1);
  }
  out
}

/// 在 `offset` 处连续取至多 `max_symbols` 个符号窗口，累加每窗的音调峰值功率。
fn alignment_score(
  samples: &[f32],
  rate: u32,
  spacing: f64,
  symlen: usize,
  offset: usize,
  max_symbols: usize,
) -> f64 {
  let mut total = 0.0f64;
  let mut rest = samples.get(offset..).unwrap_or(&[]);
  for _ in 0..max_symbols {
    let Some((win, tail)) = rest.split_at_checked(symlen) else {
      break;
    };
    total += peak_freq_power(win, rate, spacing).1;
    rest = tail;
  }
  total
}

/// 符号对齐搜索：在 `0..symlen` 内找使「每符号音调峰值能量和」最大的偏移
/// （错位窗口能量分散，正确对齐能量最集中 —— 与 NAVTEX 的相位同步同理）。
///
/// 两级搜索控制代价：先以 `symlen/16` 为步长粗扫（每点只看 16 个符号），再在最优点两侧
/// 各一个粗步长内以 `symlen/64` 细扫（32 个符号）。全分辨率逐样本扫描要对 `symlen` 个
/// 偏移各做 32 × 73 次 Goertzel，EX8 @ 8 kHz 约 6 亿次乘加，浏览器里要数秒。
///
/// 旧实现把「录音秒数 × 0.5」当样本数做扫描上界，实际只试 0 和 4 两个偏移，
/// 起点不在符号边界上的录音全靠 Goertzel 对错位的容忍。
#[must_use]
pub fn sync_offset(samples: &[f32], rate: u32, baud: f64, spacing: f64) -> usize {
  if !spacing.is_finite() || spacing <= 0.0 {
    return 0;
  }
  let Some(symlen) = symbol_len(rate, baud) else {
    return 0;
  };
  let coarse = (symlen / 16).max(1);
  let fine = (symlen / 64).max(1);
  let score = |offset: usize, n: usize| alignment_score(samples, rate, spacing, symlen, offset, n);

  let mut best = (0usize, f64::NEG_INFINITY);
  for offset in (0..symlen).step_by(coarse) {
    let total = score(offset, 16);
    if total > best.1 {
      best = (offset, total);
    }
  }
  // 在粗扫最优点两侧各一个粗步长内细扫（环绕符号边界）。
  let centre = best.0;
  let mut refined = (centre, score(centre, 32));
  for delta in (1..=coarse / fine).map(|k| k * fine) {
    for offset in [
      (centre + delta) % symlen,
      (centre + symlen - delta % symlen) % symlen,
    ] {
      let total = score(offset, 32);
      if total > refined.1 {
        refined = (offset, total);
      }
    }
  }
  refined.0
}

/// 完整解码：音频 → `(主信道文本, 二次信道文本)`。
///
/// 返回 `None` 表示没解出信号：噪声与静音都会被质量门限挡掉（见 [`locked_nibbles`]），
/// 不再把随机命中拼出来的乱码当成文本。
#[must_use]
pub fn decode_audio(
  samples: &[f32],
  rate: u32,
  baud: f64,
  spacing: f64,
) -> Option<(String, String)> {
  let nibbles = locked_stream(samples, rate, baud, spacing)?;
  let locked = nibbles.as_slice();
  let mut dec = NibbleDecoder::new();
  let take = |c: u16, dec: &mut NibbleDecoder| {
    if c == 0 {
      return; // NUL = 空闲符，不是正文
    }
    if c <= 0xFF {
      dec.primary.push(char::from(c as u8));
    } else {
      dec.secondary.push(char::from((c - 256) as u8));
    }
  };
  for &n in locked {
    if let Some(c) = dec.feed(n) {
      take(c, &mut dec);
    }
  }
  // 流尾结算最后一个字符（feed 只在下一个字符的首 nibble 到来时才结算上一个）。
  if let Some(c) = dec.flush() {
    take(c, &mut dec);
  }
  Some((dec.primary, dec.secondary))
}

/// 对齐 → 解调 → 质量门限，返回锁定后的 nibble 流（两种模式共用）。
fn locked_stream(samples: &[f32], rate: u32, baud: f64, spacing: f64) -> Option<Vec<u8>> {
  let offset = sync_offset(samples, rate, baud, spacing);
  let nibbles = demodulate_nibbles(samples, rate, baud, spacing, offset);
  locked_nibbles(&nibbles).map(<[u8]>::to_vec)
}

/// FEC 模式的完整解码：音频 → `(主信道文本, 二次信道文本)`。
///
/// 与 [`decode_audio`] 共用对齐、解调与质量门限，之后走 [`fec::FecDecoder`]。
#[must_use]
pub fn decode_audio_fec(
  samples: &[f32],
  rate: u32,
  baud: f64,
  spacing: f64,
) -> Option<(String, String)> {
  let nibbles = locked_stream(samples, rate, baud, spacing)?;
  let mut dec = fec::FecDecoder::new();
  let mut chars = Vec::new();
  for n in nibbles {
    dec.feed(n, &mut chars);
  }
  let (mut primary, mut secondary) = (String::new(), String::new());
  for c in chars {
    if c >= 0x100 {
      secondary.push(char::from((c & 0xFF) as u8));
    } else {
      primary.push(char::from(c as u8));
    }
  }
  Some((primary, secondary))
}

/// 锁定后允许的「无音调变化」（nibble 16/17）百分比：白噪声约 11%，真实信号接近 0。
const MAX_INVALID_PERCENT: usize = 5;

/// 锁定后至少要有的 nibble 数（约 10 个符号），否则比例没有统计意义。
const MIN_LOCKED_NIBBLES: usize = 30;

/// 丢弃锁定点之前的部分并做质量门限，返回锁定后的 nibble 流。
///
/// IFK+ 的增量恒为 `2 + nibble` ∈ 2..=17，所以「相邻两个窗口测到同一音调」只可能来自
/// 静音（Δf = 0，全部 16/17）或噪声里的随机命中（约 11%）。前导静音不属于信号，因此
/// 从第一个合法 nibble 起算：静音段没有合法 nibble → `None`；白噪声锁定后仍会超出
/// [`MAX_INVALID_PERCENT`] → `None`；真实信号这一段接近 0。
fn locked_nibbles(nibbles: &[u8]) -> Option<&[u8]> {
  let start = nibbles.iter().position(|&n| n <= 0x0F)?;
  // 末尾同理：录音收尾留的静音（全是 16/17）也不属于信号。此前整段按比例统计，尾静音超过
  // 5% 就把一份完好的报文整体判成 no-signal（实测 EX8 / 16 / 22 在尾静音 10% 起全部失败）。
  // 噪声里随机命中的合法 nibble 会散布到文件末尾，所以这一步不会放宽对噪声的判断。
  let end = nibbles.iter().rposition(|&n| n <= 0x0F)? + 1;
  let locked = &nibbles[start..end];
  let invalid = locked.iter().filter(|&&n| n > 0x0F).count();
  (locked.len() >= MIN_LOCKED_NIBBLES && invalid * 100 <= locked.len() * MAX_INVALID_PERCENT)
    .then_some(locked)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ifk_offset_guarantees_no_same_tone_repeats() {
    // 增量 = 2 + nibble ∈ [2, 17]：绝不为 0 —— 同音调重复被杜绝（多径自干扰
    // 规避的依据）。如实说明：增量为 17 时模 18 环绕会产生 ±1 的音调间隔，
    // 规格宣称「永不出现相同或相邻音调」与代码的环绕行为并不完全一致 ——
    // 这里只断言代码层面确实成立的性质。
    for prev in 0..NUMTONES {
      for nibble in 0..=15u8 {
        let next = next_tone(prev, nibble);
        assert_ne!(next, prev, "同音调重复：prev={prev} nibble={nibble}");
        assert!(
          (2..=17).contains(&(2 + usize::from(nibble))),
          "增量应在 2..=17"
        );
      }
    }
  }

  #[test]
  fn varicode_tables_are_self_consistent() {
    // 主字母表 256/256 精确往返；二次信道除已知碰撞外全部往返。
    // 打包顺序 = 发送序反转（最新 nibble 在最低 4 位）。
    let pack = |code: &[u8; 3]| -> usize {
      let nibs: Vec<u8> = code.iter().copied().filter(|&n| n != 0).collect();
      let nibs = if nibs.is_empty() { vec![0] } else { nibs };
      nibs
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &n)| usize::from(n) << (4 * i))
        .sum()
    };
    for (c, code) in VARICODE.iter().enumerate() {
      if c == 379 {
        continue; // 已知碰撞：379 与 381 同码 {5,10,12}，表内解为 381
      }
      let packed = pack(code);
      assert_eq!(VARIDECODE[packed], c as i16, "char {c} packed {packed:#x}");
    }
  }

  #[test]
  fn nibble_decoder_self_synchronizes() {
    // 喂入「THE」的 nibble 流（含首字符之前的乱序）：MSB 结构自己找齐。
    let stream = encode_text_nibbles("THE");
    let mut dec = NibbleDecoder::new();
    let mut text = String::new();
    for n in stream {
      if let Some(c) = dec.feed(n) {
        text.push(char::from(c as u8));
      }
    }
    // 流尾 flush：最后一个字符要等下一个首 nibble 才结算。
    if let Some(c) = dec.flush() {
      text.push(char::from(c as u8));
    }
    assert_eq!(text, "THE");
  }

  fn round_trip(mode: &str, text: &str) {
    let (baud, spacing) = parse_mode(mode).expect("mode");
    let audio = encode_audio(text, baud, spacing, 8000, 0.5).unwrap();
    let (primary, _) = decode_audio(&audio, 8000, baud, spacing).unwrap();
    // Varicode 自同步在空闲段锁定期间可能先输出一个瞬态字符（fldigi 同样如此），
    // 断言正文完整出现、且多余的前缀 ≤ 2 个字符。
    assert!(primary.contains(text), "DominoEX {mode} 往返：{primary}");
    assert!(
      primary.len() <= text.len() + 2,
      "DominoEX {mode} 前缀垃圾过多：{primary}"
    );
  }

  #[test]
  fn round_trips_default_mode() {
    round_trip("8", "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 12345");
  }

  #[test]
  fn round_trips_all_modes() {
    for mode in ["4", "16", "11"] {
      round_trip(mode, "DOMINOEX TEST 599");
    }
  }

  #[test]
  fn unencodable_nibbles_cannot_break_the_decoder() {
    // 复现旧缺陷：nibble 16（Δf = 0）的 0x8 位为 0，被当成起始 nibble 写进 buf[0] 后
    // packed ≥ 4096 → 查表越界 panic（wasm 里就是 trap）。静音段每个窗口都产出 16，
    // 所以这是可达路径，而不是只靠构造输入。
    let mut dec = NibbleDecoder::new();
    for n in [16u8, 9, 9, 16, 9, 9, 17, 200, 255] {
      let _ = dec.feed(n);
    }
    let _ = dec.flush();
  }

  #[test]
  fn noise_yields_nothing() {
    // 白噪声里差分与音调归属都是随机的，Varicode 自同步会拼出乱码：质量门限必须
    // 把它挡在 `decode_audio` 之外，而不是让页面显示一段看似正常的文本。
    let (baud, spacing) = parse_mode("8").unwrap();
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
      let samples: Vec<f32> = (0..8000 * 10)
        .map(|_| {
          seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
          ((seed >> 8) as f32 / 8_388_608.0) - 1.0
        })
        .collect();
      assert!(
        decode_audio(&samples, 8000, baud, spacing).is_none(),
        "噪声不应解出文本（seed {seed0:#x}）"
      );
    }
  }

  #[test]
  fn silence_preamble_does_not_break_decoding() {
    // 前导静音 → 每个窗口 Δf = 0 → nibble 16 连发，紧接着才是真实信号：这一段既不能
    // panic，也不能把后面的正文整段带坏。首个字符仍会错位（静音→信号的差分过渡本来
    // 就没有有效参照，与「自同步锁定前可能多出瞬态字符」的既有行为一致），所以断言
    // 正文尾部完好而不是逐字相等。
    let (baud, spacing) = parse_mode("8").unwrap();
    let mut audio = vec![0.0f32; 8000];
    audio
      .extend_from_slice(&encode_audio("THE QUICK BROWN FOX", baud, spacing, 8000, 0.0).unwrap());
    let (primary, _) = decode_audio(&audio, 8000, baud, spacing).expect("解码");
    assert!(primary.contains("BROWN FOX"), "{primary}");
  }

  #[test]
  fn secondary_channel_chars_go_to_the_secondary_text() {
    // 二次信道（ID 信标）此前零覆盖：表中码字 > 0xFF 的字符必须进 secondary，
    // 而不是混进主信道文本。
    let (c, code) = (256u16..512)
      .find_map(|c| {
        let code = VARICODE[usize::from(c)];
        (code != [0, 0, 0]).then_some((c, code))
      })
      .expect("二次信道表里应有码字");
    let mut dec = NibbleDecoder::new();
    let mut secondary = String::new();
    for n in code
      .iter()
      .copied()
      .filter(|&n| n != 0)
      .chain(std::iter::once(0))
    {
      if let Some(t) = dec.feed(n)
        && t > 0xFF
      {
        secondary.push(char::from((t - 256) as u8));
      }
    }
    assert_eq!(
      secondary.chars().next(),
      Some(char::from((c - 256) as u8)),
      "二次信道字符（码 {c:#x}）"
    );
  }

  #[test]
  fn encode_rejects_invalid_parameters() {
    // baud=0 曾让 symlen 溢出成 usize::MAX → 无界 push 直到 OOM；非有限参数同理。
    assert!(encode_audio("A", 0.0, 12.5, 8000, 0.5).is_none());
    assert!(encode_audio("A", f64::NAN, 12.5, 8000, 0.5).is_none());
    assert!(encode_audio("A", 8.0, f64::NAN, 8000, 0.5).is_none());
    assert!(encode_audio("A", 8.0, 12.5, 0, 0.5).is_none());
    assert!(encode_audio("A", 8.0, 12.5, 8000, f64::INFINITY).is_none());
    // 正常参数仍然可用。
    assert!(encode_audio("A", 8.0, 12.5, 8000, 0.5).is_some());
  }

  #[test]
  fn encode_rejects_overflowing_and_oversized_requests() {
    // R2-01：`idle_secs = 1e300` 曾饱和成 usize::MAX 个空闲符；负值曾静默当 0。
    assert!(encode_audio("A", 8.0, 12.5, 8000, 1e300).is_none());
    assert!(encode_audio("A", 8.0, 12.5, 8000, -5.0).is_none());
    assert!(encode_audio("A", 8.0, 12.5, 8000, MAX_IDLE_SECS + 1.0).is_none());
    // R2-09：巨大采样率 → 单符号数亿样本，必须在分配前拒绝。
    assert!(encode_audio("A", 8.0, 12.5, u32::MAX, 0.5).is_none());
    // baud 极小 → symlen 越过上界。
    assert!(encode_audio("A", 1e-300, 12.5, 8000, 0.5).is_none());
  }

  #[test]
  fn peak_search_terminates_for_tiny_spacing() {
    // R2-02：spacing = 1e-300 时 `f += step` 对 f64 无效，旧实现死循环。
    let samples = vec![0.0f32; 64];
    let _ = peak_freq_power(&samples, 8000, 1e-300);
    assert!(demodulate_nibbles(&samples, 8000, 7.8125, 1e-300, 0).len() <= 64);
  }

  #[test]
  fn demodulate_survives_extreme_offset_and_baud() {
    // R2-03：offset = usize::MAX 使 `i + symlen` 回绕；baud 极小使 symlen 饱和。
    let samples = vec![0.0f32; 4096];
    assert!(demodulate_nibbles(&samples, 8000, 7.8125, 15.625, usize::MAX).is_empty());
    assert!(demodulate_nibbles(&samples, 8000, 1e-300, 15.625, 0).is_empty());
    assert!(demodulate_nibbles(&samples, 8000, 7.8125, 15.625, samples.len() + 1).is_empty());
    assert_eq!(sync_offset(&samples, 8000, 1e-300, 15.625), 0);
  }

  #[test]
  fn trailing_silence_does_not_reject_a_good_message() {
    let text = "CQ CQ DE BG1AAA BG1AAA PSE K ".repeat(4);
    for mode in ["8", "16", "22"] {
      let (baud, spacing) = parse_mode(mode).unwrap();
      let audio = encode_audio(&text, baud, spacing, 8000, 0.5).unwrap();
      for tail_pct in [10usize, 30, 60, 200] {
        let mut s = audio.clone();
        s.extend(std::iter::repeat_n(0.0f32, audio.len() * tail_pct / 100));
        let (primary, _) = decode_audio(&s, 8000, baud, spacing)
          .unwrap_or_else(|| panic!("EX{mode} 尾静音 {tail_pct}% 不应判为无信号"));
        assert!(
          primary.contains("BG1AAA"),
          "EX{mode} {tail_pct}%: {primary}"
        );
      }
    }
  }

  #[test]
  fn silence_and_noise_are_still_rejected_after_trimming() {
    let (baud, spacing) = parse_mode("8").unwrap();
    assert!(decode_audio(&vec![0.0f32; 8000 * 20], 8000, baud, spacing).is_none());
    let mut seed = 0x1234_5678u32;
    let noise: Vec<f32> = (0..8000 * 20)
      .map(|_| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((seed >> 8) as f32 / 8_388_608.0) - 1.0
      })
      .collect();
    assert!(decode_audio(&noise, 8000, baud, spacing).is_none());
  }

  #[test]
  fn sync_finds_alignment_anywhere_inside_a_symbol() {
    // 旧实现把「秒数」当样本数用，扫描范围只有 0 和 4 两个偏移；错位在符号中段时
    // （半个符号 = 512 样本 @ 8 kHz、EX8）窗口跨两个音调，对齐只靠 Goertzel 的容错。
    let (baud, spacing) = parse_mode("8").unwrap();
    let audio = encode_audio("ALIGNMENT SWEEP TEST", baud, spacing, 8000, 0.5).unwrap();
    let symlen = symbol_len(8000, baud).unwrap();
    for pad in [1, 137, symlen / 4, symlen / 2, symlen * 3 / 4, symlen - 1] {
      let mut padded = vec![0.0f32; pad];
      padded.extend_from_slice(&audio);
      let offset = sync_offset(&padded, 8000, baud, spacing);
      // 前面垫了 `pad` 个样本，符号边界落在 `pad`（mod symlen）；允许 1/16 符号的搜索分辨率。
      let want = pad % symlen;
      let tol = symlen / 16 + 1;
      let diff = offset.abs_diff(want).min(symlen - offset.abs_diff(want));
      assert!(diff <= tol, "pad={pad} offset={offset} want={want}");
      let (primary, _) = decode_audio(&padded, 8000, baud, spacing).expect("应当解出");
      assert!(primary.contains("ALIGNMENT"), "pad={pad}: {primary}");
    }
  }

  #[test]
  fn decode_handles_arbitrary_leading_offset() {
    let (baud, spacing) = parse_mode("8").unwrap();
    let text = "OFFSET TEST";
    let audio = encode_audio(text, baud, spacing, 8000, 0.5).unwrap();
    let mut padded = vec![0.0f32; 137];
    padded.extend_from_slice(&audio);
    let (primary, _) = decode_audio(&padded, 8000, baud, spacing).unwrap();
    assert!(primary.contains("OFFSET"), "{primary}");
  }
}

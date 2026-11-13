//! THOR 发端：文本 → 音调序列 → 音频。逐步对应 fldigi `thor::tx_process`。

use ham_web_mfsk::fec::{Encoder, Interleaver};
use ham_web_mfsk::lookup::varienc;
use ham_web_mfsk::picture::push_tone;

use crate::varicode::SECONDARY;
use crate::{Mode, NUMTONES};

/// 单个音频的样本总数上限（5×10⁷ ≈ 200 MB f32）：不设上限时，巨大采样率或超长文本会在 wasm 里 OOM。
const MAX_SAMPLES: usize = 50_000_000;

/// 码位 → 音调号的发送状态机（fldigi `sendsymbol` / `sendchar` / `Clearbits` / `flushtx`）。
struct Sender {
  mode: Mode,
  enc: Encoder,
  inlv: Interleaver,
  bitshreg: u32,
  bitstate: usize,
  prev_tone: usize,
  tones: Vec<u8>,
}

impl Sender {
  fn new(mode: Mode) -> Self {
    Self {
      mode,
      enc: Encoder::new(mode.code()),
      inlv: Interleaver::new(4, mode.depth, true),
      bitshreg: 0,
      bitstate: 0,
      prev_tone: 0,
      tones: Vec::new(),
    }
  }

  /// IFK+：下一个音调 = `(上一个 + 2 + nibble) mod 18`。
  fn send_symbol(&mut self, nibble: u32) {
    let tone = (self.prev_tone + 2 + nibble as usize) % NUMTONES;
    self.prev_tone = tone;
    self.tones.push(tone as u8);
  }

  /// 把 2 位码字逐位推进 nibble 寄存器；攒满 4 位就过交织器，`emit` 为真时发出。
  fn push_coded(&mut self, data: u8, emit: bool) {
    for i in 0..2 {
      self.bitshreg = (self.bitshreg << 1) | u32::from((data >> i) & 1);
      self.bitstate += 1;
      if self.bitstate == 4 {
        let nibble = self.inlv.bits(self.bitshreg);
        if emit {
          self.send_symbol(nibble);
        }
        self.bitstate = 0;
        self.bitshreg = 0;
      }
    }
  }

  /// fldigi `thorvarienc`：二次信道只认 `' '..='z'`，其余当 NUL。
  fn code(c: u8, secondary: bool) -> &'static str {
    if !secondary {
      return varienc(c);
    }
    if (b' '..=b'z').contains(&c) {
      SECONDARY[usize::from(c - b' ')]
    } else {
      varienc(0)
    }
  }

  fn send_char(&mut self, c: u8, secondary: bool) {
    for b in Self::code(c, secondary).bytes() {
      let data = self.enc.encode(b == b'1');
      self.push_coded(data, true);
    }
  }

  fn send_idle(&mut self) {
    self.send_char(0, false);
  }

  /// fldigi `Clearbits`：把 1400 对零码位推进交织器但**不发符号**。
  fn clear_bits(&mut self) {
    let data = self.enc.encode(false);
    for _ in 0..1400 {
      self.push_coded(data, false);
    }
  }

  /// fldigi `flushtx`：发若干空闲符把卷积码与交织器冲干净。
  fn flush(&mut self) {
    for _ in 0..self.mode.flush {
      self.send_idle();
    }
    self.bitstate = 0;
  }
}

/// 文本 → 完整一帧的音调号序列（0–17，IFK+ 之后）。
///
/// 帧结构与 fldigi 一致：`Clearbits` → 16 个 `sendsymbol(0)`（前导）→ 空闲符 → `\r STX \r` →
/// 正文 → `\r EOT \r` → 冲刷（THOR micro 没有 STX / EOT）。正文按 UTF-8 字节走主信道；
/// **`\x01` 之后的下一个字节走二次信道**（ID 信标），与 fldigi 空闲时发的二次文本是同一条路径。
#[must_use]
pub fn encode_symbols(text: &str, mode: &Mode) -> Vec<u8> {
  let mut tx = Sender::new(*mode);
  tx.clear_bits();
  for _ in 0..16 {
    tx.send_symbol(0);
  }
  tx.send_idle();
  tx.send_char(b'\r', false);
  if !mode.micro {
    tx.send_char(2, false);
    tx.send_char(b'\r', false);
  }
  let bytes = text.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == 1 && i + 1 < bytes.len() {
      tx.send_char(bytes[i + 1], true);
      i += 2;
    } else {
      tx.send_char(bytes[i], false);
      i += 1;
    }
  }
  tx.send_char(b'\r', false);
  if !mode.micro {
    tx.send_char(4, false);
    tx.send_char(b'\r', false);
  }
  tx.flush();
  tx.tones
}

/// 文本 → 音频样本（单声道，相位连续，振幅 1）。
///
/// `center_hz` 是信号中心（18 个音调关于它对称）。任何采样率都可以：符号边界按 `rate / baud` 的
/// 小数位置落在样本上，不会因取整而累积偏移。参数越界（非有限、信号带越过 0 或 Nyquist、
/// 样本数超过上限）返回 `None`。
#[must_use]
pub fn encode_audio(text: &str, mode: &Mode, rate: u32, center_hz: f64) -> Option<Vec<f32>> {
  if rate == 0 || !center_hz.is_finite() {
    return None;
  }
  let low = center_hz + mode.tone_offset(0);
  let high = center_hz + mode.tone_offset(NUMTONES - 1);
  if low < 0.0 || high >= f64::from(rate) / 2.0 {
    return None;
  }
  let tones = encode_symbols(text, mode);
  let sym = f64::from(rate) / mode.baud();
  if tones.len() as f64 * sym > MAX_SAMPLES as f64 {
    return None;
  }
  let mut out = Vec::with_capacity((tones.len() as f64 * sym) as usize + 1);
  let mut phase = 0.0f64;
  synth_tones(&tones, mode, rate, center_hz, &mut phase, &mut out);
  Some(out)
}

/// 把音调序列合成成样本，追加到 `out`：相位连续（`phase` 由调用方带着走），符号边界按小数位置落在样本上。
/// `n` 个符号从 `out` 当前末尾起算。
fn synth_tones(
  tones: &[u8],
  mode: &Mode,
  rate: u32,
  center_hz: f64,
  phase: &mut f64,
  out: &mut Vec<f32>,
) {
  let sym = f64::from(rate) / mode.baud();
  let origin = out.len();
  for (n, &t) in tones.iter().enumerate() {
    let freq = center_hz + mode.tone_offset(usize::from(t));
    let end = origin + ((n + 1) as f64 * sym).round() as usize;
    push_tone(out, phase, freq, rate, end.saturating_sub(out.len()));
  }
}

/// 一次图片发射的音频（单声道，相位连续，振幅 1）：普通 THOR 文本（含 `pic%X` 报头）→ 冲刷 →
/// 20 个符号长的静音 → 20 个符号长的同步音 → FM 图像 → `\r EOT \r` 与冲刷。结构与 fldigi 的
/// `tx_process` / `send_image` 一致，见 [`crate::picture`]。
///
/// - `text` 在报头前面发送（可为空）；`rgb` 是 `宽 × 高 × 3` 字节；
/// - 只支持有图片模式的子模式（[`Mode::has_picture`]）、[`crate::picture::code_for`] 认得的尺寸，
///   且 `rate` 必须等于该模式的标称采样率；其余返回 `None`。
#[must_use]
pub fn encode_picture_audio(
  text: &str,
  header: &ham_web_mfsk::picture::Header,
  rgb: &[u8],
  mode: &Mode,
  rate: u32,
  center_hz: f64,
) -> Option<Vec<f32>> {
  let (w, h) = (header.width, header.height);
  let code = crate::picture::code_for(header)?;
  let sym = f64::from(rate) / mode.baud();
  let bandwidth = mode.bandwidth();
  let low = center_hz + mode.tone_offset(0);
  let high = center_hz + mode.tone_offset(NUMTONES - 1);
  // 同步音在中心 − 0.6 × 带宽，比最低音调还低 0.1 × 带宽：只查音调带不够，它会先掉到 0 Hz 以下。
  let sync_hz = center_hz + crate::picture::SYNC_OFFSET * bandwidth;
  if !mode.has_picture()
    || rate != mode.rate
    || rgb.len() != w * h * 3
    || !center_hz.is_finite()
    || low < 0.0
    || sync_hz < 0.0
    || high >= f64::from(rate) / 2.0
    || header.spp != crate::picture::SPP
  {
    return None;
  }
  let mut tx = Sender::new(*mode);
  tx.clear_bits();
  for _ in 0..16 {
    tx.send_symbol(0);
  }
  tx.send_idle();
  tx.send_char(b'\r', false);
  if !mode.micro {
    tx.send_char(2, false);
    tx.send_char(b'\r', false);
  }
  for b in text.bytes().chain(*b"pic%").chain([code]) {
    tx.send_char(b, false);
  }
  tx.flush();
  let before = tx.tones.len();

  let planes = if header.color { 3 } else { 1 };
  let sync_samples = (crate::picture::SYNC_SYMBOLS as f64 * sym).round() as usize;
  let image_samples = w * h * planes * header.spp;
  // 合成之前先按总长估一下：超限就不分配、不要等合成完才拒绝。文本与冲刷按 `before` 的两倍保守估
  // （第二次冲刷不会比「文本 + 第一次冲刷」长）。
  let estimate = 2.0 * before as f64 * sym + 2.0 * sync_samples as f64 + image_samples as f64;
  if estimate > MAX_SAMPLES as f64 {
    return None;
  }
  let mut out = Vec::with_capacity(estimate as usize);
  let mut phase = 0.0f64;
  synth_tones(
    &tx.tones[..before],
    mode,
    rate,
    center_hz,
    &mut phase,
    &mut out,
  );
  // 静音、同步音。
  out.extend(std::iter::repeat_n(0.0f32, sync_samples));
  let mut tone = |freq: f64, count: usize, out: &mut Vec<f32>| {
    push_tone(out, &mut phase, freq, rate, count);
  };
  tone(sync_hz, sync_samples, &mut out);
  // 图像：彩色每行 R、G、B 三遍，灰度每行一遍。
  for y in 0..h {
    for plane in 0..planes {
      for x in 0..w {
        let byte = rgb[3 * (y * w + x) + plane];
        tone(
          center_hz + bandwidth * (f64::from(byte) - 128.0) / 256.0,
          header.spp,
          &mut out,
        );
      }
    }
  }
  // 图像之后：`\r EOT \r` 与冲刷。
  tx.send_char(b'\r', false);
  if !mode.micro {
    tx.send_char(4, false);
    tx.send_char(b'\r', false);
  }
  tx.flush();
  synth_tones(
    &tx.tones[before..],
    mode,
    rate,
    center_hz,
    &mut phase,
    &mut out,
  );
  (out.len() <= MAX_SAMPLES).then_some(out)
}

//! THOR 发端：文本 → 音调序列 → 音频。逐步对应 fldigi `thor::tx_process`。

use ham_web_mfsk::fec::{Encoder, Interleaver};
use ham_web_mfsk::lookup::varienc;

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
  for (n, &t) in tones.iter().enumerate() {
    let freq = center_hz + mode.tone_offset(usize::from(t));
    let step = std::f64::consts::TAU * freq / f64::from(rate);
    let end = ((n + 1) as f64 * sym).round() as usize;
    while out.len() < end {
      out.push(phase.cos() as f32);
      phase += step;
      if phase > std::f64::consts::TAU {
        phase -= std::f64::consts::TAU;
      }
    }
  }
  Some(out)
}

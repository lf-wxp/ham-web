//! MFSK 发端：文本 → 音调序列 → 音频。逐步对应 fldigi `mfsk::tx_process`。

use crate::fec::{Code, Encoder, Interleaver};
use crate::lookup::varienc;
use crate::{Mode, gray_encode};

/// 单个音频的样本总数上限（5×10⁷ ≈ 200 MB f32）：不设上限时，巨大采样率或超长文本会在 wasm 里 OOM。
const MAX_SAMPLES: usize = 50_000_000;

/// 码位 → 音调号的发送状态机（fldigi `sendbit` / `sendchar` / `clearbits` / `flushtx`）。
struct Sender {
  mode: Mode,
  enc: Encoder,
  inlv: Interleaver,
  bitshreg: u32,
  bitstate: usize,
  /// 已发出的音调号（格雷码映射之后）。
  tones: Vec<u8>,
}

impl Sender {
  fn new(mode: Mode) -> Self {
    Self {
      mode,
      enc: Encoder::new(Code::NASA_K7),
      inlv: Interleaver::new(mode.symbits, mode.depth, true),
      bitshreg: 0,
      bitstate: 0,
      tones: Vec::new(),
    }
  }

  /// 把 2 位码字逐位推进符号寄存器；攒满一个符号就过交织器，`emit` 为真时发出。
  fn push_coded(&mut self, data: u8, emit: bool) {
    for i in 0..2 {
      self.bitshreg = (self.bitshreg << 1) | u32::from((data >> i) & 1);
      self.bitstate += 1;
      if self.bitstate == self.mode.symbits {
        let sym = self.inlv.bits(self.bitshreg);
        if emit {
          self
            .tones
            .push(gray_encode(sym & (self.mode.numtones() as u32 - 1)) as u8);
        }
        self.bitstate = 0;
        self.bitshreg = 0;
      }
    }
  }

  fn send_bit(&mut self, bit: bool) {
    let data = self.enc.encode(bit);
    self.push_coded(data, true);
  }

  fn send_char(&mut self, c: u8) {
    for b in varienc(c).bytes() {
      self.send_bit(b == b'1');
    }
  }

  /// fldigi `clearbits`：把 `preamble` 对零码位推进交织器但**不发符号**。
  fn clear_bits(&mut self) {
    let data = self.enc.encode(false);
    for _ in 0..self.mode.preamble {
      self.push_coded(data, false);
    }
  }

  /// fldigi `flushtx`：先发一个 1 把对端的 Varicode 译码器冲掉，再发 `n` 个零把卷积码与交织器冲干净。
  fn flush(&mut self, n: usize) {
    self.send_bit(true);
    for _ in 0..n {
      self.send_bit(false);
    }
    self.bitstate = 0;
  }
}

/// 文本 → 完整一帧的音调号序列（0 起，已过格雷码映射）。
///
/// 帧结构与 fldigi 一致：`clearbits` → `preamble / 3` 个零位 → `\r STX \r` → 正文 →
/// `\r EOT \r` → `flushtx(preamble)`。正文按 UTF-8 字节发送。
#[must_use]
pub fn encode_symbols(text: &str, mode: &Mode) -> Vec<u8> {
  let mut tx = Sender::new(*mode);
  tx.clear_bits();
  for _ in 0..mode.preamble / 3 {
    tx.send_bit(false);
  }
  for c in [b'\r', 2, b'\r'] {
    tx.send_char(c);
  }
  for b in text.bytes() {
    tx.send_char(b);
  }
  for c in [b'\r', 4, b'\r'] {
    tx.send_char(c);
  }
  tx.flush(mode.preamble);
  tx.tones
}

/// 文本 → 音频样本（单声道，相位连续，振幅 1）。
///
/// `center_hz` 是信号中心（最低与最高音调的中点）。任何采样率都可以：符号边界按
/// `rate / baud` 的小数位置落在样本上，不会因取整而累积偏移。参数越界（非有限、中心使信号
/// 越过 0 或 Nyquist、样本数超过上限）返回 `None`。
#[must_use]
pub fn encode_audio(text: &str, mode: &Mode, rate: u32, center_hz: f64) -> Option<Vec<f32>> {
  let baud = mode.baud();
  let low = center_hz - mode.bandwidth() / 2.0;
  if rate == 0 || !center_hz.is_finite() || low < 0.0 {
    return None;
  }
  if low + mode.bandwidth() >= f64::from(rate) / 2.0 {
    return None;
  }
  let tones = encode_symbols(text, mode);
  let sym = f64::from(rate) / baud;
  if tones.len() as f64 * sym > MAX_SAMPLES as f64 {
    return None;
  }
  let mut out = Vec::with_capacity((tones.len() as f64 * sym) as usize + 1);
  let mut phase = 0.0f64;
  for (n, &t) in tones.iter().enumerate() {
    let freq = low + f64::from(t) * baud;
    let step = 2.0 * std::f64::consts::PI * freq / f64::from(rate);
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

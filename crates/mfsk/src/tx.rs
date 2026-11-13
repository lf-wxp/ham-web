//! MFSK 发端：文本 → 音调序列 → 音频。逐步对应 fldigi `mfsk::tx_process`。

use crate::fec::{Code, Encoder, Interleaver};
use crate::lookup::varienc;
use crate::picture::{MAX_PIXELS, PROLOGUE_SAMPLES, push_tone};
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
/// 帧结构与 fldigi 一致：`clearbits` → `preamble / 3` 个零位（长交织变体 64L / 128L 没有） → `\r STX \r` → 正文 →
/// `\r EOT \r` → `flushtx(preamble)`。正文按 UTF-8 字节发送。
#[must_use]
pub fn encode_symbols(text: &str, mode: &Mode) -> Vec<u8> {
  let mut tx = Sender::new(*mode);
  tx.clear_bits();
  if !mode.long {
    for _ in 0..mode.preamble / 3 {
      tx.send_bit(false);
    }
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
  synth_tones(&tones, mode, rate, low, &mut phase, &mut out);
  Some(out)
}

/// 把音调序列合成成样本，追加到 `out`：相位连续（`phase` 由调用方带着走），符号边界按小数位置落在样本上。
/// `n` 个符号从 `out` 当前末尾起算，`low` 是最低音调的频率。
fn synth_tones(
  tones: &[u8],
  mode: &Mode,
  rate: u32,
  low: f64,
  phase: &mut f64,
  out: &mut Vec<f32>,
) {
  let baud = mode.baud();
  let sym = f64::from(rate) / baud;
  let origin = out.len();
  for (n, &t) in tones.iter().enumerate() {
    let freq = low + f64::from(t) * baud;
    let end = origin + ((n + 1) as f64 * sym).round() as usize;
    push_tone(out, phase, freq, rate, end.saturating_sub(out.len()));
  }
}

/// 一次图片发射的音频（单声道，相位连续，振幅 1）：普通 MFSK 文本（含 `Pic:…;` 报头）→ 冲刷 → 序言 →
/// FM 图像 → 冲刷。结构与 fldigi 的 `tx_process` / `sendpic` 一致，见 [`crate::picture`]。
///
/// - `text` 在报头前面发送（可为空）；
/// - `rgb` 是 `宽 × 高 × 3` 字节；`gray` 为真时只发一遍（取每个像素的 R 通道，调用方应已做过灰度）；
/// - 只支持有图片模式的子模式（[`Mode::has_picture`]），且 `rate` 必须等于该模式的标称采样率（序言与每像素样本数是按它给的）；
///   其余返回 `None`。
#[must_use]
pub fn encode_picture_audio(
  text: &str,
  header: &crate::picture::Header,
  rgb: &[u8],
  mode: &Mode,
  rate: u32,
  center_hz: f64,
) -> Option<Vec<f32>> {
  let (w, h) = (header.width, header.height);
  // `Header` 的字段是公开的，调用方可以塞任何值：尺寸与 `spp` 要按解码端认得的范围收紧，
  // 否则写出去的报头（`p3`、巨型尺寸）解码端要么当成别的值、要么根本不认。
  let pixels = w.checked_mul(h)?;
  if !mode.has_picture()
    || rate != mode.rate
    || pixels == 0
    || pixels > MAX_PIXELS
    || rgb.len() != pixels * 3
    || !center_hz.is_finite()
    || center_hz - mode.bandwidth() / 2.0 < 0.0
    || center_hz + mode.bandwidth() / 2.0 >= f64::from(rate) / 2.0
    || !matches!(header.spp, 2 | 4 | 8)
  {
    return None;
  }
  let low = center_hz - mode.bandwidth() / 2.0;
  let mut tx = Sender::new(*mode);
  tx.clear_bits();
  if !mode.long {
    for _ in 0..mode.preamble / 3 {
      tx.send_bit(false);
    }
  }
  for c in [b'\r', 2, b'\r'] {
    tx.send_char(c);
  }
  let mut line = text.to_owned();
  line.push_str(&format!(
    "Pic:{w}x{h}{}{};",
    if header.color { "C" } else { "" },
    if header.spp == 8 {
      String::new()
    } else {
      format!("p{}", header.spp)
    }
  ));
  for b in line.bytes() {
    tx.send_char(b);
  }
  tx.flush(mode.preamble);
  let before = tx.tones.len();

  // 合成之前先按总长估一下：超限就不分配，不要等合成完才拒绝。
  // 文本与冲刷的符号数按 `before` 的两倍保守估（第二次冲刷不会比「文本 + 第一次冲刷」长）。
  let sym = f64::from(rate) / mode.baud();
  let planes = if header.color { 3 } else { 1 };
  let estimate =
    (2 * before) as f64 * sym + PROLOGUE_SAMPLES + (pixels * planes * header.spp) as f64;
  if estimate > MAX_SAMPLES as f64 {
    return None;
  }
  let mut out = Vec::with_capacity(estimate as usize);
  let mut phase = 0.0f64;
  synth_tones(&tx.tones[..before], mode, rate, low, &mut phase, &mut out);
  // 序言：最低音调。
  push_tone(&mut out, &mut phase, low, rate, PROLOGUE_SAMPLES as usize);
  // 图像：发送顺序是彩色每行 R、G、B 三遍，灰度每行一遍。
  for y in 0..h {
    for plane in 0..planes {
      for x in 0..w {
        let byte = rgb[3 * (y * w + x) + plane];
        let f = center_hz + mode.bandwidth() * (f64::from(byte) - 128.0) / 256.0;
        push_tone(&mut out, &mut phase, f, rate, header.spp);
      }
    }
  }
  tx.flush(mode.preamble);
  synth_tones(&tx.tones[before..], mode, rate, low, &mut phase, &mut out);
  (out.len() <= MAX_SAMPLES).then_some(out)
}

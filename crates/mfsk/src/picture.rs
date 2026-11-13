//! MFSK 图片模式：报头识别与模拟调频图像解调。
//!
//! 转写自 **fldigi**（GPL-3.0）`src/mfsk/mfsk.cxx` 的 `check_picture_header` / `recvpic` / `recvchar` /
//! `sendpic`（图片模式 © W1HKJ）。
//!
//! 一次图片发射的结构：
//! 1. 一段普通的 MFSK 文本，里面有报头 `Pic:<宽>x<高>[C][p<2|4|8>];`（`C` = 彩色，`p` 后面是每像素样本数，缺省 8）；
//! 2. `flushtx(preamble)` 把前向纠错与交织器冲干净（一串最低音调）；
//! 3. **序言**：`352` 个样本的最低音调（`tracepair.delay`，让接收端的解码流水线赶上来）；
//! 4. **图像**：每个像素一个字节，发成「频率 = 中心 + 带宽 × (字节 − 128) / 256」的**纯调频**音调，
//!    每个像素占 `spp` 个样本；彩色图一行发三遍（R 行、G 行、B 行），灰度图只发一遍；
//! 5. 之后又是普通的 MFSK（`flushtx` 的冲刷，或接着发的文本）。
//!
//! 接收端（这里）：先照常解出文本找到报头，由报头的位置推出图像的起点，再对图像段做 FM 解调。

use std::f64::consts::TAU;

/// fldigi 的 `PICHEADER`：报头窗口长 64 字节（最后一个是 NUL），能放下 63 个字符。
const HEADER_WINDOW: usize = 63;

/// 序言的样本数（fldigi `tracepair.delay`）：与采样率无关，MFSK11 / 22 的 11025 Hz 上也是 352 个样本。
pub const PROLOGUE_SAMPLES: f64 = 352.0;

/// 图像段至少要录到多少比例才出图。报头来自解出的文本，一段噪声或一句手敲的 `Pic:640x480C;` 都能造出一个
/// 巨大的报头，而它后面根本没有图像：不设下限就会按报头尺寸分配并输出一幅几乎全黑的图。
const MIN_COVERAGE: f64 = 0.1;

/// 解调用的低通 FIR 抽头数（fldigi 的 `bpfilt` 也是 127 阶）。
const TAPS: usize = 127;

/// [`instantaneous_frequency`] 一次处理的样本数：混频缓冲只保留「一块 + 两侧各 [`TAPS`] / 2」，
/// 峰值内存与图像面积无关。1024 × 1024 彩色的图像段有 2500 万个样本，整段物化两个 `f64` 复数缓冲要 800 MB。
const CHUNK: usize = 16_384;

/// 图片尺寸上限（fldigi：宽高都不超过 4095）。
const MAX_SIDE: usize = 4095;

/// 图片总像素上限。fldigi 的 4095 × 4095 在 MFSK16 的每像素 8 个样本下要发近一个小时，没有人会这么发；
/// 而报头是从**解出来的文本**里读的，噪声或别的信号误识别出一个巨型报头，就会按它分配整幅图
/// （4095² 彩色约 50 MB，到页面上还要再乘 RGBA 与 PNG）。1024 × 1024 够放任何实际发射的图。
pub const MAX_PIXELS: usize = 1 << 20;

/// 每个像素样本数的上限。MFSK 报头只写 2 / 4 / 8，THOR 固定 10；留到 16 是为了限住 `samples()` 的乘积：
/// 在 32 位的 wasm 里 `usize` 只有 4 G，`spp` 不设限时 `3 × 像素上限 × spp` 会回绕。
pub const MAX_SPP: usize = 16;

/// 一张图片的报头。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
  pub width: usize,
  pub height: usize,
  pub color: bool,
  /// 每个像素占多少个样本（报头里 `p8` / `p4` / `p2`，缺省 8；THOR 固定 10）。
  pub spp: usize,
}

impl Header {
  /// 图像段共多少个样本：彩色图每个像素发三遍。
  #[must_use]
  pub fn samples(&self) -> usize {
    self.spp * self.width * self.height * if self.color { 3 } else { 1 }
  }

  /// 尺寸与每像素样本数是否在解码端认得的范围内。
  ///
  /// `Header` 的字段是公开的、报头又来自**解出来的文本**（可以被噪声或一句手敲的 `Pic:…;` 伪造），
  /// 所以解码入口要自己再核一遍，不能只靠 [`HeaderScanner`] 的那道关。
  #[must_use]
  pub fn is_supported(&self) -> bool {
    in_range(self.width, self.height) && (1..=MAX_SPP).contains(&self.spp)
  }
}

/// 宽高是否在解码端认得的范围内。
fn in_range(width: usize, height: usize) -> bool {
  width != 0
    && height != 0
    && width <= MAX_SIDE
    && height <= MAX_SIDE
    && width.saturating_mul(height) <= MAX_PIXELS
}

/// 解出来的图片：`rgb` 是 `宽 × 高 × 3` 字节（灰度图三个通道相同）。
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
  pub width: usize,
  pub height: usize,
  pub color: bool,
  pub rgb: Vec<u8>,
}

impl Picture {
  /// 页面（`ImageData`）要的 RGBA：`宽 × 高 × 4` 字节，不透明。MFSK / THOR 两个 Worker 共用。
  #[must_use]
  pub fn to_rgba(&self) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(self.rgb.len() / 3 * 4);
    for px in self.rgb.as_chunks::<3>().0 {
      rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
    }
    rgba
  }
}

/// 报头扫描器：逐个字符喂，识别 `Pic:…;`（fldigi `check_picture_header`）。
///
/// 只有可打印字符（`' '..='z'`）才进窗口；识别成功后窗口清空（`recvchar` 里同样如此）。
#[derive(Debug, Clone)]
pub struct HeaderScanner {
  window: [u8; HEADER_WINDOW],
}

impl Default for HeaderScanner {
  fn default() -> Self {
    Self {
      window: [b' '; HEADER_WINDOW],
    }
  }
}

impl HeaderScanner {
  /// 喂一个解出来的字符；这个字符恰好补完报头时返回它。
  pub fn push(&mut self, c: u16) -> Option<Header> {
    let Ok(c) = u8::try_from(c) else {
      return None;
    };
    if !(b' '..=b'z').contains(&c) {
      return None;
    }
    self.window.copy_within(1.., 0);
    self.window[HEADER_WINDOW - 1] = c;
    let header = parse_header(&self.window)?;
    self.window = [b' '; HEADER_WINDOW];
    Some(header)
  }
}

/// 在窗口里找第一个 `Pic:` 并按 fldigi 的语法解析；不完整或不合法返回 `None`。
fn parse_header(window: &[u8]) -> Option<Header> {
  let at = window.windows(4).position(|w| w == b"Pic:")?;
  let mut p = &window[at + 4..];
  let number = |p: &mut &[u8]| -> usize {
    let mut n = 0usize;
    while let Some((&d, rest)) = p.split_first() {
      if !d.is_ascii_digit() {
        break;
      }
      n = n.saturating_mul(10).saturating_add(usize::from(d - b'0'));
      *p = rest;
    }
    n
  };
  let width = number(&mut p);
  let (&x, rest) = p.split_first()?;
  if x != b'x' {
    return None;
  }
  p = rest;
  let height = number(&mut p);
  let mut color = false;
  if let Some((&b'C', rest)) = p.split_first() {
    color = true;
    p = rest;
  }
  let valid = in_range;
  match p.split_first()? {
    (&b';', _) => valid(width, height).then_some(Header {
      width,
      height,
      color,
      spp: 8,
    }),
    (&b'p', rest) => {
      let (&speed, rest) = rest.split_first()?;
      let (&end, _) = rest.split_first()?;
      if end != b';' || !valid(width, height) {
        return None;
      }
      let spp = match speed {
        b'4' => 4,
        b'2' => 2,
        _ => 8,
      };
      Some(Header {
        width,
        height,
        color,
        spp,
      })
    }
    _ => None,
  }
}

/// 以 `fc` 为中心、`cutoff` 为截止频率的加窗 sinc 低通（Hamming 窗，直流增益 1）。
fn lowpass(cutoff: f64, rate: f64) -> Vec<f64> {
  let mid = (TAPS - 1) as f64 / 2.0;
  let fc = (cutoff / rate).min(0.49);
  let mut h: Vec<f64> = (0..TAPS)
    .map(|n| {
      let x = n as f64 - mid;
      let sinc = if x == 0.0 {
        2.0 * fc
      } else {
        (TAU * fc * x).sin() / (std::f64::consts::PI * x)
      };
      sinc * (0.54 - 0.46 * (TAU * n as f64 / (TAPS - 1) as f64).cos())
    })
    .collect();
  let sum: f64 = h.iter().sum();
  for v in &mut h {
    *v /= sum;
  }
  h
}

/// 各样本区间 `[m, m + 1]` 上的瞬时频率（Hz）；`freqs[i]` 对应 `[from + i, from + i + 1]`：把 `[from, to)` 范围的样本混到基带、低通，再取相邻样本的相位差。
///
/// 低通是线性相位的零延迟形式（居中，不是因果），所以结果的下标与输入样本对齐，不需要补偿群延迟。
/// 混频缓冲按 [`CHUNK`] 分块，峰值内存与 `to - from` 无关（只有返回的 `Vec<f64>` 随长度增长）。
pub fn instantaneous_frequency(
  samples: &[f32],
  rate: f64,
  center_hz: f64,
  cutoff_hz: f64,
  from: usize,
  to: usize,
) -> Vec<f64> {
  let half = TAPS / 2;
  // 分段结果与整段一次算完逐位相同：同一批样本、同一批抽头、同一个累加顺序。
  let last = (to + half + 1).min(samples.len());
  if last <= from + 1 {
    return Vec::new();
  }
  // 最后一个要参与的样本：录音在 `to` 之前结束时停在录音末尾（与原实现一致）。
  let stop = to.min(last - 1);
  if stop <= from {
    return Vec::new();
  }
  let step = TAU * center_hz / rate;
  let h = lowpass(cutoff_hz, rate);
  let mut out = Vec::with_capacity(stop - from);
  let mut prev: Option<(f64, f64)> = None;
  // `n` 是当前要算的滤波点，产出的是区间 `[n − 1, n)` 的频率。
  let mut n = from;
  while n <= stop {
    let chunk_end = (n + CHUNK).min(stop + 1);
    // 抽头要覆盖到 `chunk_end − 1` 的正向一半。
    let lo = n.saturating_sub(half);
    let hi = (chunk_end + half).min(samples.len());
    let raw: Vec<(f64, f64)> = (lo..hi)
      .map(|m| {
        let phase = step * m as f64;
        let x = f64::from(samples[m]);
        (x * phase.cos(), -x * phase.sin())
      })
      .collect();
    for m in n..chunk_end {
      // 滤波器中心对准样本 m：抽头 k 取 raw[base + k]（base = m − half − lo）；窗口外按 0（录音的头尾）。
      // 先算出落在窗口内的抽头区间，内层循环里就没有逐抽头的边界判断。
      let base = m as isize - half as isize - lo as isize;
      let k0 = usize::try_from(-base).unwrap_or(0).min(TAPS);
      let k1 = usize::try_from(raw.len() as isize - base).map_or(0, |end| end.min(TAPS));
      let (mut re, mut im) = (0.0, 0.0);
      for (k, &c) in h.iter().enumerate().take(k1).skip(k0) {
        let (r, i) = raw[(base + k as isize) as usize];
        re += c * r;
        im += c * i;
      }
      if let Some((pr, pi)) = prev {
        let re_diff = re * pr + im * pi;
        let im_diff = im * pr - re * pi;
        out.push(center_hz + im_diff.atan2(re_diff) * rate / TAU);
      }
      prev = Some((re, im));
    }
    n = chunk_end;
  }
  out
}

/// 发端：追加 `count` 个 `freq_hz` 的余弦样本，相位接着 `phase` 走（`sendpic` / 符号合成共用，整段相位连续）。
///
/// 频率必须非负：相位只朝正向回绕（`> TAU` 时减一个周期），负频率会让 `phase` 单调下降、
/// 越来越远离 0，正弦虽然还对，但频率会镜像到正半轴（发出去的音频与预期不符）。
pub fn push_tone(out: &mut Vec<f32>, phase: &mut f64, freq_hz: f64, rate: u32, count: usize) {
  debug_assert!(freq_hz >= 0.0, "音调频率必须非负：{freq_hz}");
  let step = TAU * freq_hz / f64::from(rate);
  for _ in 0..count {
    out.push(phase.cos() as f32);
    *phase += step;
    if *phase > TAU {
      *phase -= TAU;
    }
  }
}

/// 图像段 FM 解调的几何：采样率、信号中心、最低到最高音调的带宽、低通截止。
#[derive(Debug, Clone, Copy)]
pub struct FmDemod {
  pub rate: u32,
  /// 信号中心频率（Hz）。
  pub center_hz: f64,
  /// 最低到最高音调的带宽（`(音调数 − 1) × 音调间隔`）：字节 0 对应 `center − bw / 2`，
  /// 字节 255 对应 `center + bw / 2 − bw / 256`。
  pub bandwidth_hz: f64,
  /// 低通截止（Hz）。
  pub cutoff_hz: f64,
}

impl FmDemod {
  /// MFSK：信号摆动 ±bw/2，再留 2 个音调间隔的余量；镜像在 2·fc 之外，远得多。
  #[must_use]
  pub fn mfsk(rate: u32, center_hz: f64, bandwidth_hz: f64) -> Self {
    Self {
      rate,
      center_hz,
      bandwidth_hz,
      cutoff_hz: bandwidth_hz / 2.0 + bandwidth_hz / 7.5,
    }
  }

  /// THOR：沿用 fldigi 的 `picfilter`，截止 = 带宽。
  #[must_use]
  pub fn thor(rate: u32, center_hz: f64, bandwidth_hz: f64) -> Self {
    Self {
      rate,
      center_hz,
      bandwidth_hz,
      cutoff_hz: bandwidth_hz,
    }
  }
}

/// 图像段的 FM 解调。
///
/// - `start`：第一个像素的第一个样本的位置（小数样本）；
/// - `reverse`：边带反转（字节取 `255 − 字节`）。
///
/// 录音在图像中途结束时，没收到的像素是 0。但剩下的样本不够 [`MIN_COVERAGE`]（也不够一整行）就返回 `None`：
/// 这样的「图片」只可能来自误识别的报头，照常输出只会是一幅按报头尺寸分配的、几乎全黑的图。
#[must_use]
pub fn decode_picture(
  samples: &[f32],
  demod: &FmDemod,
  start: f64,
  header: &Header,
  reverse: bool,
) -> Option<Picture> {
  if !header.is_supported() {
    return None;
  }
  let rate = f64::from(demod.rate);
  let (w, h) = (header.width, header.height);
  let planes = if header.color { 3 } else { 1 };
  let total = w * h * planes;
  let first = start.round().max(0.0) as usize;
  let image_samples = total * header.spp;
  let needed = (w * planes * header.spp).max((image_samples as f64 * MIN_COVERAGE).ceil() as usize);
  if first.saturating_add(needed) > samples.len() {
    return None;
  }
  let last = first + image_samples;
  let freqs = instantaneous_frequency(
    samples,
    rate,
    demod.center_hz,
    demod.cutoff_hz,
    first,
    last.min(samples.len()),
  );
  let low = demod.center_hz - demod.bandwidth_hz / 2.0;
  let mut rgb = vec![0u8; w * h * 3];
  for k in 0..total {
    let a = k * header.spp;
    let Some(slice) = freqs.get(a..a + header.spp) else {
      break;
    };
    let f = slice.iter().sum::<f64>() / header.spp as f64;
    let mut byte = (256.0 * (f - low) / demod.bandwidth_hz).clamp(0.0, 255.0) as u8;
    if reverse {
      byte = 255 - byte;
    }
    // 第 `k` 个像素在发送顺序里的位置：彩色图一行三遍（R 行、G 行、B 行），每遍 `w` 个像素。
    if header.color {
      let (row, rest) = (k / (3 * w), k % (3 * w));
      let (plane, col) = (rest / w, rest % w);
      rgb[3 * (row * w + col) + plane] = byte;
    } else {
      let (row, col) = (k / w, k % w);
      let at = 3 * (row * w + col);
      rgb[at..at + 3].fill(byte);
    }
  }
  Some(Picture {
    width: w,
    height: h,
    color: header.color,
    rgb,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn scan(text: &str) -> Vec<Header> {
    let mut s = HeaderScanner::default();
    text.bytes().filter_map(|b| s.push(u16::from(b))).collect()
  }

  #[test]
  fn headers_follow_fldigi_syntax() {
    let h = |w, h, color, spp| Header {
      width: w,
      height: h,
      color,
      spp,
    };
    assert_eq!(scan("Pic:32x24C;"), [h(32, 24, true, 8)]);
    assert_eq!(scan("Pic:59x74;"), [h(59, 74, false, 8)]);
    assert_eq!(scan("Pic:160x120Cp4;"), [h(160, 120, true, 4)]);
    assert_eq!(scan("Pic:160x120p2;"), [h(160, 120, false, 2)]);
    // 前面有别的文字、报头里没有的速度字母按 8 处理。
    assert_eq!(scan("Sending Pic:32x24C;"), [h(32, 24, true, 8)]);
    assert_eq!(scan("Pic:8x8Cp5;"), [h(8, 8, true, 8)]);
  }

  #[test]
  fn broken_headers_are_ignored() {
    for text in [
      "Pic:",
      "Pic:32",
      "Pic:32x",
      "Pic:32x24",
      "Pic:32x24C",
      "Pic:0x24;",
      "Pic:32x0;",
      "Pic:5000x24;",
      // 单边都不超 4095，但总像素超过上限：不按它分配整幅图。
      "Pic:4095x4095C;",
      "Pic:1025x1024;",
      "Pic:32y24;",
      "Pic:32x24Cp;",
      "Pic:32x24Cp4",
      "Pic:32x24Cp4x",
    ] {
      assert!(scan(text).is_empty(), "{text}");
    }
  }

  #[test]
  fn the_window_forgets_after_a_header_and_non_printables_are_skipped() {
    // 补完一个报头之后窗口清空：紧接着的半个报头不会与前面的字符拼成第二个。
    assert_eq!(scan("Pic:32x24C;Pic:8x").len(), 1);
    // 控制字符不进窗口，也不打断报头。
    let mut s = HeaderScanner::default();
    let mut found = None;
    for c in "Pic:3\r2x2\u{2}4;".chars() {
      found = found.or(s.push(c as u16));
    }
    assert_eq!(found.map(|h| (h.width, h.height)), Some((32, 24)));
    assert_eq!(HeaderScanner::default().push(0x1234), None);
  }

  /// 构造一段 FM 图像：每个像素 `spp` 个样本、频率 = 中心 + bw × (字节 − 128) / 256（`sendpic`）。
  fn fm(bytes: &[u8], spp: usize, rate: f64, center: f64, bw: f64) -> Vec<f32> {
    let mut out = Vec::new();
    let mut phase = 0.0f64;
    for &b in bytes {
      let f = center + bw * (f64::from(b) - 128.0) / 256.0;
      for _ in 0..spp {
        out.push(phase.cos() as f32);
        phase += TAU * f / rate;
      }
    }
    out
  }

  #[test]
  fn a_synthetic_fm_image_comes_back() {
    let (w, h, spp) = (12usize, 5usize, 8usize);
    // 发送顺序：每行 R、G、B 三遍。
    let mut want = vec![0u8; w * h * 3];
    let mut sent = Vec::new();
    for y in 0..h {
      for plane in 0..3 {
        for x in 0..w {
          // 相邻像素只差几个字节：127 阶低通抹圆的是像素之间的大跳变，那是 fldigi 同样的局限，不是这里要测的。
          let v = (60 + x * 6 + y * 9 + plane * 4) as u8;
          want[3 * (y * w + x) + plane] = v;
          sent.push(v);
        }
      }
    }
    // 前后各接一段最低音调（真实发射里是序言与冲刷），不是静音；整段一次合成，相位连续（与 `sendpic` 一样）。
    let lead = 38usize;
    let mut all = vec![0u8; lead];
    all.extend(&sent);
    all.extend(vec![0u8; 50]);
    let samples = fm(&all, spp, 8000.0, 1500.0, 234.375);
    let pad = lead * spp;
    let header = Header {
      width: w,
      height: h,
      color: true,
      spp,
    };
    let demod = FmDemod::mfsk(8000, 1500.0, 234.375);
    let pic = decode_picture(&samples, &demod, pad as f64, &header, false).unwrap();
    // 行首 / 行尾各两个像素不看：每一遍换行时值要回落几十个字节，这个台阶被 127 阶低通抹圆，
    // 行首行尾的像素偏得最多；fldigi 的接收机同样如此。中间的像素要在 ±10 以内（平均不到 3）。
    let mut worst = 0u32;
    for y in 0..h {
      for x in 2..w - 2 {
        for c in 0..3 {
          let at = 3 * (y * w + x) + c;
          worst = worst.max(u32::from(pic.rgb[at]).abs_diff(u32::from(want[at])));
        }
      }
    }
    assert!(worst <= 10, "行内像素的最大误差 {worst}");
  }

  #[test]
  fn reverse_flips_the_bytes_and_gray_fills_three_channels() {
    let (w, h, spp) = (8usize, 4usize, 8usize);
    let sent: Vec<u8> = (0..w * h).map(|i| (60 + i * 3) as u8).collect();
    // 图像之后接一段音调：最后一个像素的区间要有后一个样本才算得出相位差。
    let mut all = sent.clone();
    all.extend([60u8; 8]);
    let samples = fm(&all, spp, 8000.0, 1500.0, 234.375);
    let header = Header {
      width: w,
      height: h,
      color: false,
      spp,
    };
    let demod = FmDemod::mfsk(8000, 1500.0, 234.375);
    let normal = decode_picture(&samples, &demod, 0.0, &header, false).unwrap();
    let flipped = decode_picture(&samples, &demod, 0.0, &header, true).unwrap();
    for (a, b) in normal.rgb.iter().zip(&flipped.rgb) {
      assert_eq!(u16::from(*a) + u16::from(*b), 255);
    }
    // 第 5 个像素（离开头的台阶远了）：发的是 60 + 5 × 3 = 75，反转后读出它的补 180。
    assert!(i32::from(normal.rgb[5 * 3]).abs_diff(75) <= 4);
    assert!(i32::from(flipped.rgb[5 * 3]).abs_diff(180) <= 4);
    for px in normal.rgb.as_chunks::<3>().0 {
      assert!(px[0] == px[1] && px[1] == px[2], "灰度图三个通道必须相同");
    }
  }

  #[test]
  fn a_recording_that_ends_inside_the_image_leaves_the_rest_black() {
    let header = Header {
      width: 10,
      height: 10,
      color: true,
      spp: 8,
    };
    let sent = vec![200u8; 10 * 3 * 4];
    let samples = fm(&sent, 8, 8000.0, 1500.0, 234.375);
    let demod = FmDemod::mfsk(8000, 1500.0, 234.375);
    let pic = decode_picture(&samples, &demod, 0.0, &header, false).unwrap();
    assert_eq!(pic.rgb.len(), 300);
    assert!(
      pic.rgb[..3 * 10 * 3].iter().any(|&v| v > 150),
      "收到的几行应有内容"
    );
    assert!(
      pic.rgb[3 * 10 * 5..].iter().all(|&v| v == 0),
      "没收到的行是 0"
    );
  }

  #[test]
  fn a_header_with_no_recorded_image_yields_no_picture() {
    let header = Header {
      width: 4,
      height: 4,
      color: false,
      spp: 8,
    };
    let samples = fm(&[128u8; 3], 8, 8000.0, 1500.0, 234.375);
    let demod = FmDemod::mfsk(8000, 1500.0, 234.375);
    // 起点在录音之外。
    assert!(decode_picture(&samples, &demod, 1.0e6, &header, false).is_none());
    // 起点之后剩下的样本不够一行（一行 4 × 8 = 32 个样本，这里只有 24 个）。
    assert!(decode_picture(&samples, &demod, 0.0, &header, false).is_none());
    // 刚好有一行就出图（其余行是 0）。
    let one_row = fm(&[128u8; 4], 8, 8000.0, 1500.0, 234.375);
    assert!(decode_picture(&one_row, &demod, 0.0, &header, false).is_some());
  }

  #[test]
  fn to_rgba_adds_an_opaque_alpha_channel() {
    let pic = Picture {
      width: 2,
      height: 1,
      color: true,
      rgb: vec![1, 2, 3, 4, 5, 6],
    };
    assert_eq!(pic.to_rgba(), [1, 2, 3, 255, 4, 5, 6, 255]);
  }

  #[test]
  fn a_long_tone_is_continuous_across_the_chunk_boundary() {
    // 混频缓冲是分块算的：跨块时上一块的最后一个滤波点要接着用，否则块边界上会冒出两个假的频率。
    let rate = 8000.0;
    let samples: Vec<f32> = (0..3 * CHUNK + 977)
      .map(|n| (TAU * 1000.0 * n as f64 / rate).cos() as f32)
      .collect();
    let freqs = instantaneous_frequency(&samples, rate, 1000.0, 200.0, 0, samples.len());
    assert_eq!(freqs.len(), samples.len() - 1);
    // 头尾各几十个样本被滤波器的零填充影响，中间必须处处是 1000 Hz（含块边界附近）。
    for (n, &f) in freqs.iter().enumerate().take(freqs.len() - 64).skip(64) {
      assert!((f - 1000.0).abs() < 1.0, "第 {n} 个样本：{f}");
    }
  }

  #[test]
  fn absurd_headers_are_refused_before_allocating() {
    let samples = fm(&[128u8; 64], 8, 8000.0, 1500.0, 234.375);
    let demod = FmDemod::mfsk(8000, 1500.0, 234.375);
    for header in [
      // 宽高为 0、超过单边上限、总像素超过上限。
      Header {
        width: 0,
        height: 4,
        color: false,
        spp: 8,
      },
      Header {
        width: 4095,
        height: 4095,
        color: true,
        spp: 8,
      },
      // 每像素样本数为 0：`samples()` 会是 0，后面按它算切分没有意义。
      Header {
        width: 4,
        height: 4,
        color: false,
        spp: 0,
      },
      // 每像素样本数超过上限：32 位 wasm 里 `samples()` 的乘积会回绕。
      Header {
        width: 4,
        height: 4,
        color: false,
        spp: MAX_SPP + 1,
      },
      Header {
        width: 1024,
        height: 1024,
        color: true,
        spp: usize::MAX,
      },
    ] {
      assert!(!header.is_supported());
      assert!(decode_picture(&samples, &demod, 0.0, &header, false).is_none());
    }
    // THOR 固定 10，MFSK 报头只有 2 / 4 / 8：都在上限内。
    for spp in [2, 4, 8, 10] {
      assert!(
        Header {
          width: 4,
          height: 4,
          color: false,
          spp,
        }
        .is_supported()
      );
    }
    assert!(
      Header {
        width: 4,
        height: 4,
        color: false,
        spp: 8,
      }
      .is_supported()
    );
  }
}

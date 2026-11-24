//! SSTV 发端：RGB 图像 → 基带音频（8 kHz 单声道），按规格文本写。
//!
//! **独立于解码器**：这里不读 [`Mode::scans`] / `sync_ms` / `line_ms`，每个模式的行结构（同步、分隔、
//! 分量顺序与时长）在本文件里再写一遍，只取 [`Mode`] 的名字、VIS 码与图像尺寸。编码器与解码器各自对着
//! 规格实现，才能互相检验 —— 此前两者共用同一份错误假设（VIS 起始位、位极性、Robot / Scottie DX 的
//! 时序），自洽测试一直是绿的，对真实录音却一份也解不出（见 `tests/roundtrip.rs` 的开头）。
//! 另有一份测试内的独立编码器（`tests/roundtrip.rs`），与本文件在 5 个 RGB 模式上互相对拍。
//!
//! # 颜色
//!
//! - Martin / Scottie：R、G、B 各自一段，字节 0–255 线性映射到 1500–2300 Hz；
//! - Robot：BT.601 **视频范围**的 YCbCr（Y 以 16 为黑电平、色差零点 128，与解码器的
//!   `ycc_to_rgb` 互逆），同样线性映射。Robot 36 每行只发一路色度（偶行 R-Y、奇行 B-Y，水平分辨率减半），
//!   两行共用一块 2×2 平均；Robot 72 每行发 R-Y 与 B-Y（水平分辨率减半，行内两个像素平均）。
//!
//! # 边界（如实）
//!
//! Robot 72 的两个分隔脉冲（R-Y 前 1500 Hz、B-Y 前 2300 Hz）按资料写成，本站的解码器不看这个频率，
//! 所以**没有第三方接收端对过**；其余结构由本站解码器与真实录音（Martin M1 / Robot 36）对过。
//! 输出采样率固定为 [`WORK_RATE`]（8 kHz）：像素最窄约 0.275 ms（Robot 36），按累计毫秒取整，
//! 不会累积误差。

use std::f32::consts::TAU;

use crate::{BLACK_HZ, Mode, WHITE_HZ, WORK_RATE};

/// 发端的失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
  /// 这个模式没有编码器（解码器认得的七个模式之外）。
  UnsupportedMode,
  /// 像素数据长度与 `宽 × 高 × 3` 对不上。
  ImageSize,
}

impl EncodeError {
  /// 稳定的机器可读错误码。
  #[must_use]
  pub fn code(&self) -> &'static str {
    match self {
      Self::UnsupportedMode => "unsupported-mode",
      Self::ImageSize => "image-size",
    }
  }
}

/// 这个模式有没有编码器。
#[must_use]
pub fn supports(mode: &Mode) -> bool {
  matches!(
    mode.name,
    "Martin M1"
      | "Martin M2"
      | "Scottie S1"
      | "Scottie S2"
      | "Scottie DX"
      | "Robot 36"
      | "Robot 72"
  )
}

/// 末行之后补的黑电平时长（毫秒）。
///
/// 真实发射的末尾总有一小段余音（PTT 释放前的静默或载波）；这里补 300 ms 黑电平，让严格的接收端
/// 不会因为录音**恰好**在末行最后一个像素结束处截止而丢掉最后一行 —— 本站的解码器就是这样：
/// 末行的整个扫描段必须落在音频内才读，行锚点估计差几个样本就会把它丢掉（Scottie 最明显，
/// 其 `edge_ms` 标着「未验证」）。不影响整图时长的对账（`tests/roundtrip.rs` 允许 ±1.5%）。
const TAIL_MS: f64 = 300.0;

/// 字节 → 频率（0 → 1500 Hz，255 → 2300 Hz）。
fn level(byte: u8) -> f32 {
  BLACK_HZ + (WHITE_HZ - BLACK_HZ) * f32::from(byte) / 255.0
}

/// BT.601 视频范围的 RGB → (Y, Cb, Cr)，定点整数写法（标准的 8 位变换）。
fn ycbcr(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
  let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
  let clamp = |x: i32| x.clamp(0, 255) as u8;
  (
    clamp(16 + ((66 * r + 129 * g + 25 * b + 128) >> 8)),
    clamp(128 + ((-38 * r - 74 * g + 112 * b + 128) >> 8)),
    clamp(128 + ((112 * r - 94 * g - 18 * b + 128) >> 8)),
  )
}

/// 累加时间的音调写入器：用累计毫秒取整，避免每段单独取整带来的系统性时长误差
/// （Martin 一行有十几个段，逐段取整会让整幅图漂掉几行）。
struct Tones {
  freq: Vec<f32>,
  elapsed_ms: f64,
}

impl Tones {
  fn new() -> Self {
    Self {
      freq: Vec::new(),
      elapsed_ms: 0.0,
    }
  }

  fn push(&mut self, hz: f32, ms: f64) {
    self.elapsed_ms += ms;
    let target = (self.elapsed_ms / 1000.0 * f64::from(WORK_RATE)).round() as usize;
    let n = target.saturating_sub(self.freq.len());
    self.freq.extend(std::iter::repeat_n(hz, n));
  }

  /// 一个扫描段：`pixels` 个像素，总时长 `len_ms`，第 `i` 个像素的频率是 `level_of(i)`。
  fn scan(&mut self, len_ms: f64, pixels: usize, level_of: impl Fn(usize) -> f32) {
    let per = len_ms / pixels as f64;
    for i in 0..pixels {
      self.push(level_of(i), per);
    }
  }
}

/// 图像里 `(列, 行)` 的某个通道（0 = R，1 = G，2 = B）。
fn channel(rgb: &[u8], width: usize, col: usize, row: usize, c: usize) -> u8 {
  rgb[3 * (row * width + col) + c]
}

/// 头部：引导音 300 / 间隔 10 / 引导音 300 / 起始位 30 / 8 位 / 停止位 30（毫秒）。
fn header(t: &mut Tones, vis_code: u8) {
  t.push(1900.0, 300.0);
  t.push(1200.0, 10.0);
  t.push(1900.0, 300.0);
  t.push(1200.0, 30.0);
  let parity = u8::from(vis_code.count_ones() % 2 == 1);
  let full = (vis_code & 0x7F) | (parity << 7);
  for b in 0..8 {
    // 1100 Hz = 逻辑 1，1300 Hz = 逻辑 0（LSB 先发），每位 30 ms。
    t.push(if (full >> b) & 1 == 1 { 1100.0 } else { 1300.0 }, 30.0);
  }
  t.push(1200.0, 30.0);
}

/// 一行里 Martin 的三个分量扫描（G → B → R），每段之后一个 0.572 ms 分隔。
fn martin(t: &mut Tones, mode: &Mode, rgb: &[u8]) {
  let scan_ms = if mode.name == "Martin M1" {
    146.432
  } else {
    73.216
  };
  let w = mode.width;
  for row in 0..mode.height {
    t.push(1200.0, 4.862);
    t.push(BLACK_HZ, 0.572);
    for c in [1, 2, 0] {
      t.scan(scan_ms, w, |col| level(channel(rgb, w, col, row, c)));
      t.push(BLACK_HZ, 0.572);
    }
  }
}

/// Scottie：分隔 → G → 分隔 → B → **同步** → 分隔 → R；整幅图开头多一个 9 ms 同步。
fn scottie(t: &mut Tones, mode: &Mode, rgb: &[u8]) {
  let scan_ms = match mode.name {
    "Scottie S1" => 138.24,
    "Scottie S2" => 88.064,
    _ => 345.6,
  };
  let w = mode.width;
  t.push(1200.0, 9.0);
  for row in 0..mode.height {
    t.push(BLACK_HZ, 1.5);
    t.scan(scan_ms, w, |col| level(channel(rgb, w, col, row, 1)));
    t.push(BLACK_HZ, 1.5);
    t.scan(scan_ms, w, |col| level(channel(rgb, w, col, row, 2)));
    t.push(1200.0, 9.0);
    t.push(BLACK_HZ, 1.5);
    t.scan(scan_ms, w, |col| level(channel(rgb, w, col, row, 0)));
  }
}

/// 一个像素的 Y / Cb / Cr。
fn pixel_ycc(rgb: &[u8], width: usize, col: usize, row: usize) -> (u8, u8, u8) {
  ycbcr(
    channel(rgb, width, col, row, 0),
    channel(rgb, width, col, row, 1),
    channel(rgb, width, col, row, 2),
  )
}

/// `(col0.., row0..)` 起的 `cols × rows` 小块里，取某个色差通道的平均（四舍五入）。
fn block_chroma(
  rgb: &[u8],
  width: usize,
  (col0, row0): (usize, usize),
  (cols, rows): (usize, usize),
  pick: impl Fn((u8, u8, u8)) -> u8,
) -> u8 {
  let mut sum = 0u32;
  for r in row0..row0 + rows {
    for c in col0..col0 + cols {
      sum += u32::from(pick(pixel_ycc(rgb, width, c, r)));
    }
  }
  let n = (cols * rows) as u32;
  ((sum + n / 2) / n) as u8
}

/// Robot 36：每行 Y 88 ms + 分隔（偶行 1500 Hz = R-Y，奇行 2300 Hz = B-Y）+ 一路 44 ms 的色度。
fn robot36(t: &mut Tones, mode: &Mode, rgb: &[u8]) {
  let w = mode.width;
  for row in 0..mode.height {
    t.push(1200.0, 9.0);
    t.push(BLACK_HZ, 3.0);
    t.scan(88.0, w, |col| level(pixel_ycc(rgb, w, col, row).0));
    let even = row % 2 == 0;
    t.push(if even { 1500.0 } else { 2300.0 }, 4.5);
    t.push(1900.0, 1.5);
    // 相邻两行（偶 + 奇）共用一块 2×2 平均；高度为奇数时最后一行只用自己。
    let pair_row = row - row % 2;
    let rows = if pair_row + 1 < mode.height { 2 } else { 1 };
    t.scan(44.0, w / 2, |c| {
      let at = (2 * c, pair_row);
      let size = (2, rows);
      level(if even {
        block_chroma(rgb, w, at, size, |(_, _, cr)| cr)
      } else {
        block_chroma(rgb, w, at, size, |(_, cb, _)| cb)
      })
    });
  }
}

/// Robot 72：每行 Y 138 ms，再各一段 69 ms 的 R-Y 与 B-Y（行内两个像素平均）。
fn robot72(t: &mut Tones, mode: &Mode, rgb: &[u8]) {
  let w = mode.width;
  for row in 0..mode.height {
    t.push(1200.0, 9.0);
    t.push(BLACK_HZ, 3.0);
    t.scan(138.0, w, |col| level(pixel_ycc(rgb, w, col, row).0));
    t.push(1500.0, 4.5);
    t.push(1900.0, 1.5);
    t.scan(69.0, w / 2, |c| {
      level(block_chroma(rgb, w, (2 * c, row), (2, 1), |(_, _, cr)| cr))
    });
    t.push(2300.0, 4.5);
    t.push(1900.0, 1.5);
    t.scan(69.0, w / 2, |c| {
      level(block_chroma(rgb, w, (2 * c, row), (2, 1), |(_, cb, _)| cb))
    });
  }
}

/// 频率序列 → FM 调制样本（相位累加，振幅 1）。
fn fm_modulate(freq: &[f32]) -> Vec<f32> {
  let mut out = Vec::with_capacity(freq.len());
  let mut phase = 0.0f32;
  for &f in freq {
    out.push(phase.sin());
    phase = (phase + TAU * f / WORK_RATE as f32) % TAU;
  }
  out
}

/// 把一幅 `mode.width × mode.height` 的 RGB 图像编成 SSTV 基带（[`WORK_RATE`]，单声道，振幅 1）。
///
/// `rgb` 是 `宽 × 高 × 3` 字节、行优先。时长就是该模式的整图时长（Martin M1 约 114 秒，
/// Scottie DX 约 269 秒，Robot 36 约 36 秒），外加头部与结尾的 300 ms 黑电平。
///
/// # Errors
///
/// 模式没有编码器，或像素数据长度对不上。
pub fn encode(mode: &Mode, rgb: &[u8]) -> Result<Vec<f32>, EncodeError> {
  if !supports(mode) {
    return Err(EncodeError::UnsupportedMode);
  }
  if mode
    .width
    .checked_mul(mode.height)
    .and_then(|n| n.checked_mul(3))
    != Some(rgb.len())
  {
    return Err(EncodeError::ImageSize);
  }
  let mut t = Tones::new();
  header(&mut t, mode.vis_code);
  match mode.name {
    "Martin M1" | "Martin M2" => martin(&mut t, mode, rgb),
    "Scottie S1" | "Scottie S2" | "Scottie DX" => scottie(&mut t, mode, rgb),
    "Robot 36" => robot36(&mut t, mode, rgb),
    _ => robot72(&mut t, mode, rgb),
  }
  t.push(BLACK_HZ, TAIL_MS);
  Ok(fm_modulate(&t.freq))
}

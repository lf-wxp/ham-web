//! 生成一段合成的 SSTV 音频样本（WAV），用于手动测试解码器。
//!
//! 用法：
//! ```text
//! cargo run -p ham-web-sstv --release --example generate_sample -- [输出.wav] [VIS码] [噪声幅度]
//! ```
//! - VIS 码：默认 44（Martin M1）；支持 44/40（Martin）、60/56/76（Scottie）、8/12（Robot）。
//! - 噪声幅度：默认 0.02，加在 FM 信号上模拟真实接收；0 表示无噪声。
//!
//! 图像内容（便于验证解码）：左 → 右的灰度渐变；Robot 的色差置中性（输出灰度）。
//!
//! **这是一份独立于解码器的编码器，按规格文本写成**（头部：引导音 / 间隔 / 引导音 / 起始位
//! 30 ms / 8 位 / 停止位；行结构：同步 / 分隔 / 各分量，Scottie 的同步在蓝之后、Robot 36 的色度
//! 每行只发一路）。它**不读** `Mode::scans`：行结构在这里再写一遍，解码器与编码器各自
//! 对着规格实现，才能互相检验。此前两者共用同一份错误假设，所以自洽测试一直是绿的，对真实
//! 录音却一份也解不出。

use std::f32::consts::TAU;

use ham_web_sstv::{MODES, Mode, WORK_RATE};

/// 像素频率范围（Hz）。
const BLACK: f32 = 1500.0;
const WHITE: f32 = 2300.0;

fn main() {
  let mut args = std::env::args().skip(1);
  let out_path = args.next().unwrap_or_else(|| "sstv-sample.wav".to_owned());
  let vis: u8 = args.next().and_then(|s| s.parse().ok()).unwrap_or(44);
  let noise: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.02);

  let mode = MODES.iter().find(|m| m.vis_code == vis).unwrap_or_else(|| {
    let list = MODES
      .iter()
      .map(|m| format!("{}（{}）", m.name, m.vis_code))
      .collect::<Vec<_>>()
      .join("、");
    panic!("未知 VIS 码 {vis}，支持：{list}");
  });

  let freq = build_freq(mode);
  let mut signal = fm_modulate(&freq);
  if noise > 0.0 {
    add_noise(&mut signal, noise);
  }
  let wav = make_wav(&signal);
  std::fs::write(&out_path, &wav).expect("写入 WAV 失败");
  println!(
    "已生成 {out_path}：模式 {}，{}×{}，{} 样本 ≈ {:.1}s，噪声幅度 {noise:.2}",
    mode.name,
    mode.width,
    mode.height,
    signal.len(),
    signal.len() as f32 / WORK_RATE as f32
  );
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
}

fn ramp(col: usize, width: usize) -> f32 {
  BLACK + (col as f32 / (width - 1) as f32) * (WHITE - BLACK)
}

/// 一个扫描段：`pixels` 个像素，左 → 右渐变，总时长 `len_ms`。
fn scan(t: &mut Tones, len_ms: f64, pixels: usize, level: impl Fn(usize) -> f32) {
  let per = len_ms / pixels as f64;
  for i in 0..pixels {
    t.push(level(i), per);
  }
}

/// 构造整段频率序列：VOX 之外的完整头部 + 逐行扫描。
fn build_freq(mode: &Mode) -> Vec<f32> {
  let mut t = Tones::new();
  // 头部：引导音 300 / 间隔 10 / 引导音 300 / 起始位 30 / 8 位 / 停止位 30。
  t.push(1900.0, 300.0);
  t.push(1200.0, 10.0);
  t.push(1900.0, 300.0);
  t.push(1200.0, 30.0);
  let parity = (mode.vis_code.count_ones() % 2) as u8;
  let full = mode.vis_code | (parity << 7);
  for b in 0..8 {
    // 1100 Hz = 逻辑 1，1300 Hz = 逻辑 0（LSB 先发）。
    t.push(if (full >> b) & 1 == 1 { 1100.0 } else { 1300.0 }, 30.0);
  }
  t.push(1200.0, 30.0);

  let w = mode.width;
  match mode.name {
    "Martin M1" | "Martin M2" => {
      let scan_ms = if mode.name == "Martin M1" {
        146.432
      } else {
        73.216
      };
      for _ in 0..mode.height {
        t.push(1200.0, 4.862);
        t.push(BLACK, 0.572);
        for _ in 0..3 {
          scan(&mut t, scan_ms, w, |c| ramp(c, w));
          t.push(BLACK, 0.572);
        }
      }
    }
    "Scottie S1" | "Scottie S2" | "Scottie DX" => {
      let scan_ms = match mode.name {
        "Scottie S1" => 138.24,
        "Scottie S2" => 88.064,
        _ => 345.6,
      };
      // 整幅图开头多一个同步（所以第一行比其余行长 10.5 ms）。
      t.push(1200.0, 9.0);
      for row in 0..mode.height {
        t.push(BLACK, 1.5);
        scan(&mut t, scan_ms, w, |c| ramp(c, w)); // 绿
        t.push(BLACK, 1.5);
        scan(&mut t, scan_ms, w, |c| ramp(c, w)); // 蓝
        t.push(1200.0, 9.0); // 同步：蓝之后、红之前
        t.push(BLACK, 1.5);
        scan(&mut t, scan_ms, w, |c| ramp(c, w)); // 红
        let _ = row;
      }
    }
    "Robot 36" => {
      for row in 0..mode.height {
        t.push(1200.0, 9.0);
        t.push(BLACK, 3.0);
        scan(&mut t, 88.0, w, |c| ramp(c, w)); // Y
        // 分隔脉冲：偶行 1500 Hz（R-Y）、奇行 2300 Hz（B-Y）。
        t.push(if row % 2 == 0 { 1500.0 } else { 2300.0 }, 4.5);
        t.push(1900.0, 1.5);
        scan(&mut t, 44.0, w / 2, |_| 1900.0); // 色度置中性（1900 Hz = 128）
      }
    }
    "Robot 72" => {
      for _ in 0..mode.height {
        t.push(1200.0, 9.0);
        t.push(BLACK, 3.0);
        scan(&mut t, 138.0, w, |c| ramp(c, w)); // Y
        t.push(BLACK, 4.5);
        t.push(1900.0, 1.5);
        scan(&mut t, 69.0, w / 2, |_| 1900.0); // V（R-Y）
        t.push(BLACK, 4.5);
        t.push(1900.0, 1.5);
        scan(&mut t, 69.0, w / 2, |_| 1900.0); // U（B-Y）
      }
    }
    other => panic!("没有为 {other} 写编码器"),
  }
  t.freq
}

/// 频率序列 → FM 调制样本（相位累加）。
fn fm_modulate(freq: &[f32]) -> Vec<f32> {
  let mut out = Vec::with_capacity(freq.len());
  let mut phase = 0.0f32;
  for &f in freq {
    out.push(phase.sin());
    phase = (phase + TAU * f / WORK_RATE as f32) % TAU;
  }
  out
}

/// 叠加均匀白噪声（幅度 `amp`）。
fn add_noise(signal: &mut [f32], amp: f32) {
  let mut seed = 0x1234_5678u32;
  for s in signal {
    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let u = (seed >> 8) as f32 / 16_777_216.0;
    *s += amp * (u * 2.0 - 1.0);
  }
}

/// 16 位 PCM 单声道 WAV（采样率 [`WORK_RATE`]）。
fn make_wav(signal: &[f32]) -> Vec<u8> {
  let data_len = signal.len() * 2;
  let mut out = Vec::with_capacity(44 + data_len);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
  out.extend_from_slice(b"WAVEfmt ");
  out.extend_from_slice(&16u32.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes());
  out.extend_from_slice(&WORK_RATE.to_le_bytes());
  out.extend_from_slice(&(WORK_RATE * 2).to_le_bytes());
  out.extend_from_slice(&2u16.to_le_bytes());
  out.extend_from_slice(&16u16.to_le_bytes());
  out.extend_from_slice(b"data");
  out.extend_from_slice(&(data_len as u32).to_le_bytes());
  for &s in signal {
    out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
  }
  out
}

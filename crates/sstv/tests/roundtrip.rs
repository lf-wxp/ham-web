//! 端到端自检：合成 Martin M1 信号 → 解码 → 验证图像尺寸与像素渐变。

use std::f32::consts::TAU;

const RATE: u32 = 8000;

/// 合成一个左黑右白渐变的 Martin M1 信号（归一化样本）。
fn synth_martin_m1(width: usize, height: usize) -> Vec<f32> {
  let sync_ms = 4.862f32;
  let porch_ms = 0.572f32;
  let pixel_us = 457.6f32;
  let vis_code = 44u8;

  let mut freq = Vec::new();
  push_tone(&mut freq, 1900.0, 0.2);
  push_vis(&mut freq, vis_code);
  for _row in 0..height {
    push_tone(&mut freq, 1200.0, sync_ms / 1000.0);
    push_tone(&mut freq, 1500.0, porch_ms / 1000.0);
    for col in 0..width {
      let f = 1500.0 + (col as f32 / width as f32) * 800.0;
      for _ in 0..3 {
        push_tone(&mut freq, f, pixel_us / 1_000_000.0);
      }
    }
  }

  let mut out = Vec::with_capacity(freq.len());
  let mut phase = 0.0f32;
  for &f in &freq {
    out.push(phase.sin());
    phase = (phase + TAU * f / RATE as f32) % TAU;
  }
  out
}

fn push_tone(freq: &mut Vec<f32>, hz: f32, seconds: f32) {
  let n = (seconds * RATE as f32).round() as usize;
  freq.extend(std::iter::repeat_n(hz, n));
}

fn push_vis(freq: &mut Vec<f32>, code: u8) {
  push_tone(freq, 1200.0, 0.300);
  let parity = code.count_ones() % 2;
  let full = code | ((parity as u8) << 7);
  for b in 0..8 {
    let f = if (full >> b) & 1 == 1 { 1300.0 } else { 1100.0 };
    push_tone(freq, f, 0.030);
  }
  push_tone(freq, 1200.0, 0.030);
}

#[test]
fn decodes_martin_m1_gradient() {
  let samples = synth_martin_m1(320, 256);
  let img = ham_web_sstv::decode_samples(&samples, RATE).expect("应解码成功");

  assert_eq!(img.mode, "Martin M1");
  assert_eq!(img.width, 320);
  // 允许因同步锁定/边界效应损失少量行。
  assert!(img.height >= 200, "行数应足够，实际 {}", img.height);

  // 取中间一行，检查左右两端灰度：左黑右白。
  let mid_row = (img.height / 2) as usize;
  let px = |col: usize| {
    let o = (mid_row * 320 + col) * 4;
    img.rgba[o] // 红色通道（渐变图 R=G=B）
  };
  assert!(px(10) < 40, "左端应偏黑，得到 {}", px(10));
  assert!(px(310) > 210, "右端应偏白，得到 {}", px(310));
}

#[test]
fn rejects_non_sstv_audio() {
  // 纯 1900 Hz 单音（无 VIS）。
  let n = RATE as usize;
  let samples: Vec<f32> = (0..n)
    .map(|i| (TAU * 1900.0 * i as f32 / RATE as f32).sin())
    .collect();
  assert!(ham_web_sstv::decode_samples(&samples, RATE).is_err());
}

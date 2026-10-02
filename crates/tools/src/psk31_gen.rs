//! 生成合成 PSK31 测试样本 WAV，用于手动测试 `/psk-decode` 页面。

use std::path::Path;

use anyhow::{Context, Result};

/// 生成合成 PSK31 音频并写为 WAV。
pub fn generate(output: &Path, text: &str, rate: u32, center: f32, noise: f32) -> Result<()> {
  let samples = ham_web_core::psk31::synthesize(text, rate, center, noise);
  let wav = make_wav(&samples, rate);
  if let Some(parent) = output.parent()
    && !parent.as_os_str().is_empty()
  {
    std::fs::create_dir_all(parent).ok();
  }
  std::fs::write(output, &wav).with_context(|| format!("写入 {} 失败", output.display()))?;
  println!(
    "已生成 {}：{} 采样 = {:.1}s，采样率 {} Hz，载波 {} Hz，噪声 σ={}",
    output.display(),
    samples.len(),
    samples.len() as f64 / f64::from(rate),
    rate,
    center,
    noise
  );
  Ok(())
}

/// 写为 16 位 PCM 单声道 WAV。
fn make_wav(signal: &[f32], rate: u32) -> Vec<u8> {
  let data_len = signal.len() * 2;
  let mut out = Vec::with_capacity(44 + data_len);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
  out.extend_from_slice(b"WAVE");
  out.extend_from_slice(b"fmt ");
  out.extend_from_slice(&16u32.to_le_bytes());
  out.extend_from_slice(&1u16.to_le_bytes()); // PCM
  out.extend_from_slice(&1u16.to_le_bytes()); // 单声道
  out.extend_from_slice(&rate.to_le_bytes());
  out.extend_from_slice(&(rate * 2).to_le_bytes()); // 字节率 = 采样率 × 2（16 位）
  out.extend_from_slice(&2u16.to_le_bytes()); // 块对齐
  out.extend_from_slice(&16u16.to_le_bytes()); // 位深
  out.extend_from_slice(b"data");
  out.extend_from_slice(&(data_len as u32).to_le_bytes());
  for &s in signal {
    out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
  }
  out
}

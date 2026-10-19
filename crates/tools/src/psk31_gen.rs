//! 生成合成 PSK31 测试样本 WAV，用于手动测试 `/psk-decode` 页面。

use std::path::Path;

use anyhow::{Context, Result};

/// 生成合成 PSK31 音频并写为 WAV。
pub fn generate(output: &Path, text: &str, rate: u32, center: f32, noise: f32) -> Result<()> {
  let samples = ham_web_core::psk31::synthesize(text, rate, center, noise);
  let wav = ham_web_wav::encode(&samples, rate);
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

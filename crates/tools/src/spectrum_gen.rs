//! 生成合成频谱测试样本 WAV，以及读取 WAV 做频谱分析（用于 SDR 瀑布图的测试数据）。

use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};

/// 解析「频率:幅度,频率:幅度」形式的音调列表。
pub fn parse_tones(s: &str) -> Result<Vec<(f32, f32)>> {
  let mut out = Vec::new();
  for part in s.split(',') {
    let part = part.trim();
    if part.is_empty() {
      continue;
    }
    let (f, a) = part
      .split_once(':')
      .ok_or_else(|| anyhow!("音调格式应为「频率:幅度」，得到「{part}」"))?;
    out.push((f.parse()?, a.parse()?));
  }
  if out.is_empty() {
    bail!("至少需要一个音调");
  }
  Ok(out)
}

/// 生成合成频谱音频并写为 WAV。
pub fn generate(
  output: &Path,
  rate: u32,
  seconds: f32,
  tones: &[(f32, f32)],
  noise: f32,
) -> Result<()> {
  let samples = ham_web_core::spectrum::synthesize(rate, seconds, tones, noise);
  let wav = make_wav(&samples, rate);
  if let Some(parent) = output.parent()
    && !parent.as_os_str().is_empty()
  {
    std::fs::create_dir_all(parent).ok();
  }
  std::fs::write(output, &wav).with_context(|| format!("写入 {} 失败", output.display()))?;
  let desc = tones
    .iter()
    .map(|(f, a)| format!("{f:.0} Hz×{a:.2}"))
    .collect::<Vec<_>>()
    .join(" + ");
  println!(
    "已生成 {}：{} 采样 = {:.1}s，采样率 {} Hz，音调 {}，噪声幅度 {}",
    output.display(),
    samples.len(),
    samples.len() as f64 / f64::from(rate),
    rate,
    desc,
    noise
  );
  Ok(())
}

/// 读取 WAV 并做频谱分析，打印前 `k` 个峰值频率。
pub fn analyze(path: &Path, k: usize) -> Result<()> {
  let (samples, rate) = read_wav(path)?;
  let db = ham_web_core::spectrum::spectrum_db(&samples);
  let peaks = ham_web_core::spectrum::top_peaks(&db, rate as f32, k);
  println!(
    "样本：{} 个采样 = {:.1}s，采样率 {} Hz",
    samples.len(),
    samples.len() as f64 / f64::from(rate),
    rate
  );
  println!("频谱前 {} 个峰值：", peaks.len());
  for (i, (f, v)) in peaks.iter().enumerate() {
    println!("  {}. {:>8.1} Hz  {:>6.1} dB", i + 1, f, v);
  }
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

/// 读取 16 位 PCM WAV，返回（样本 f32, 采样率）。多声道时只取第一声道。
fn read_wav(path: &Path) -> Result<(Vec<f32>, u32)> {
  let bytes = std::fs::read(path).with_context(|| format!("读取 {} 失败", path.display()))?;
  if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
    bail!("不是有效的 WAV 文件");
  }

  let mut rate = 0u32;
  let mut channels = 1u16;
  let mut bits = 16u16;
  let mut data: Option<&[u8]> = None;

  let mut pos = 12usize;
  while pos + 8 <= bytes.len() {
    let id = &bytes[pos..pos + 4];
    let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
    let body_start = pos + 8;
    let body_end = (body_start + size).min(bytes.len());
    match id {
      b"fmt " => {
        if body_end - body_start >= 16 {
          rate = u32::from_le_bytes(bytes[body_start + 4..body_start + 8].try_into().unwrap());
          channels = u16::from_le_bytes(bytes[body_start + 2..body_start + 4].try_into().unwrap());
          bits = u16::from_le_bytes(bytes[body_start + 14..body_start + 16].try_into().unwrap());
        }
      }
      b"data" => data = Some(&bytes[body_start..body_end]),
      _ => {}
    }
    pos = body_end + (size & 1); // chunk 按 2 字节对齐
  }

  let data = data.ok_or_else(|| anyhow!("未找到 data chunk"))?;
  let samples: Vec<f32> = match bits {
    16 => data
      .as_chunks::<2>()
      .0
      .iter()
      .map(|c| i16::from_le_bytes(*c) as f32 / 32768.0)
      .collect(),
    8 => data.iter().map(|&b| (b as f32 - 128.0) / 128.0).collect(),
    b => bail!("不支持的位深 {b}（仅支持 8 / 16 位）"),
  };

  // 多声道：每隔 channels 个样本取一个（第一声道）。
  let samples = if channels > 1 {
    samples.into_iter().step_by(channels as usize).collect()
  } else {
    samples
  };

  Ok((samples, rate))
}

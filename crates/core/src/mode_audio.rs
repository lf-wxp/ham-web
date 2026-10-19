//! 数字模式基带音频合成：文本 → CW / RTTY 的声卡基带采样与 WAV。
//!
//! 补「只有收没有发」的缺口：把 [`crate::mode_encoder`] 已有的编码（摩尔斯、ITA2）转成
//! 可馈入电台声卡的基带音频。全部是确定性的合成，可用已知码型对拍：
//!
//! - **CW**：点 1 单位、划 3 单位，符号间 1 单位、字符间 3 单位、单词间 7 单位
//!   （1 单位 = 1.2 / WPM 秒，Paris 口径），音调为正弦。
//! - **RTTY**：45.45 波特 AFSK，`mark`（逻辑 1）与 `space`（逻辑 0）两个音频，
//!   每字符起始位 0 + 5 位 ITA2（低位在先）+ 1.5 位停止（1）。

use crate::mode_encoder::text_to_ita2;

/// CW 默认音调（Hz）。
pub const CW_TONE_HZ: f64 = 700.0;
/// CW 默认速度（字/分，Paris）。
pub const CW_WPM: f64 = 20.0;
/// RTTY 默认波特率。
pub const RTTY_BAUD: f64 = 45.45;
/// RTTY 默认 mark / space 音频（Hz，高音对）。
pub const RTTY_MARK_HZ: f64 = 2125.0;
pub const RTTY_SPACE_HZ: f64 = 2295.0;
/// 基带采样率（Hz，声卡输入常用）。
pub const SAMPLE_RATE: u32 = 8000;

/// 文本长度上限（字符）：再长的报文不该走「生成一段 WAV 贴到声卡」这条路。
///
/// 页面在调用前也按这个值校验，好给出「报文过长」的提示而不是导出一个空文件。
pub const MAX_TEXT_CHARS: usize = 1024;

/// 单次合成的采样数上限（约 32 MiB 的 f32）。
///
/// `MAX_TEXT_CHARS` 只限字符数：乘上「一个单位最坏 1 秒」后，1024 字符仍可到几千万
/// 采样（约 200 MB f32，浏览器标签页直接爆）。这里按**输出采样数**兜底，超出按参数失真
/// 处理（返回空，页面会给「无法生成音频」）。站点页面用的是 8 kHz，1024 字符约 490 万
/// 采样，仍在限内；48 kHz 的调用方需要自己缩短文本。
pub const MAX_SAMPLES: usize = 8 * 1024 * 1024;

/// 基带采样率上限（Hz）。
///
/// 覆盖到 192 kHz 声卡再重采样 2 倍的余量；再高已不是声卡基带场景，且会把
/// `MAX_SAMPLES` 兜底生效前的单字符分配量放大到几十 GB。超出按参数失真处理。
pub const MAX_SAMPLE_RATE: u32 = 384_000;

/// 追加一段正弦波（幅度 1.0）。
///
/// 相位跨段累加：每段都从 0 起振会在比特 / 符号边界留下跳变，实发有咔嗒声与频谱旁瓣。
fn tone(out: &mut Vec<f32>, freq_hz: f64, sample_rate: u32, samples: usize, phase: &mut f64) {
  let step = std::f64::consts::TAU * freq_hz / sample_rate as f64;
  for _ in 0..samples {
    out.push(phase.sin() as f32);
    *phase = (*phase + step) % std::f64::consts::TAU;
  }
}

/// 追加一段静音。
fn silence(out: &mut Vec<f32>, samples: usize) {
  out.resize(out.len() + samples, 0.0);
}

/// CW 键控的上升 / 下降沿（毫秒）。
///
/// 矩形键控（幅度瞬间 0 → 1）会在每个点划边缘留下键控咔嗒声，频谱旁瓣还会拓宽到邻频。
/// 业余电台常见的 CW 上升时间是 4–8 ms；5 ms 的升余弦在 700 Hz 音调下约 3.5 个周期，
/// 既压得住咔嗒声，又不会把 40 WPM 的点（30 ms）吃得发糊。
const CW_RAMP_MS: f64 = 5.0;

/// 追加一段带升余弦包络的键控音（CW 的「按下」）。
///
/// 包络落在这段音自己的采样内，不改变时长；音段短于两个包络时各取一半，保证首尾归零。
fn keyed_tone(out: &mut Vec<f32>, freq_hz: f64, sample_rate: u32, samples: usize, phase: &mut f64) {
  let start = out.len();
  tone(out, freq_hz, sample_rate, samples, phase);
  let ramp = ((f64::from(sample_rate) * CW_RAMP_MS / 1000.0).round() as usize).min(samples / 2);
  for i in 0..ramp {
    let gain = (0.5 * (1.0 - (std::f64::consts::PI * i as f64 / ramp as f64).cos())) as f32;
    out[start + i] *= gain;
    out[start + samples - 1 - i] *= gain;
  }
}

/// 文本 → CW 基带音频采样；参数不合法返回空。
#[must_use]
pub fn cw_samples(text: &str, wpm: f64, freq_hz: f64, sample_rate: u32) -> Vec<f32> {
  // `is_finite` 不能省：`NaN <= 0.0` 恒为 false，NaN 频率会一路进到 `tone` 里，整段
  // 采样变 NaN，调用方拿到的是「成功但无声」的 WAV。
  if !wpm.is_finite()
    || wpm <= 0.0
    || !freq_hz.is_finite()
    || freq_hz <= 0.0
    || sample_rate == 0
    || sample_rate > MAX_SAMPLE_RATE
  {
    return Vec::new();
  }
  // 点数不足一个采样（WPM 极高）或一个单位超过 1 秒（WPM < 1.2）都属参数失真：
  // 前者会静默产出空音频，后者会把「48 个字符」放大成几百 MB。
  let unit = (sample_rate as f64 * 1.2 / wpm).round() as usize;
  if unit == 0 || unit > sample_rate as usize || text.chars().count() > MAX_TEXT_CHARS {
    return Vec::new();
  }
  let mut out = Vec::new();
  let mut phase = 0.0;
  let chars: Vec<char> = text.chars().collect();
  for (i, ch) in chars.iter().enumerate() {
    // 每轮先兜底一次：单词空格分支走 `continue`、跳过了字符末尾的检查，超限只能在
    // 下一轮循环头或循环结束后拦（见 `MAX_SAMPLES`）。
    if out.len() > MAX_SAMPLES {
      return Vec::new();
    }
    if *ch == ' ' {
      // 单词间 7 单位：上一符号已留 1 单位，补 6。
      silence(&mut out, unit * 6);
      continue;
    }
    let Some(code) = crate::morse::code_of(*ch) else {
      continue;
    };
    for sym in code.chars() {
      let on = if sym == '.' { unit } else { unit * 3 };
      keyed_tone(&mut out, freq_hz, sample_rate, on, &mut phase);
      silence(&mut out, unit); // 符号间 1 单位
    }
    // 字符间 3 单位：上一符号已留 1，补 2；但下一个是单词分隔时由上面分支处理。
    if i + 1 < chars.len() && chars[i + 1] != ' ' {
      silence(&mut out, unit * 2);
    }
  }
  // 最后一轮若是空格，`continue` 后不会再进循环头检查，这里兜一次。
  if out.len() > MAX_SAMPLES {
    return Vec::new();
  }
  out
}

/// 文本 → RTTY（AFSK）基带音频采样；参数不合法返回空。
#[must_use]
pub fn rtty_samples(
  text: &str,
  baud: f64,
  mark_hz: f64,
  space_hz: f64,
  sample_rate: u32,
) -> Vec<f32> {
  // 同上：NaN 会穿过 `<= 0.0` 的比较，必须显式挡掉。
  if !baud.is_finite()
    || baud <= 0.0
    || !mark_hz.is_finite()
    || mark_hz <= 0.0
    || !space_hz.is_finite()
    || space_hz <= 0.0
    || sample_rate == 0
    || sample_rate > MAX_SAMPLE_RATE
  {
    return Vec::new();
  }
  let bit = (sample_rate as f64 / baud).round() as usize;
  // 同上：一位不足一个采样，或一位超过 1 秒（波特率 < 采样率）都不该继续乗算。
  if bit == 0 || bit > sample_rate as usize || text.chars().count() > MAX_TEXT_CHARS {
    return Vec::new();
  }
  let mut out = Vec::new();
  let mut phase = 0.0;
  for code in text_to_ita2(text) {
    // 起始位 = space（逻辑 0）。
    tone(&mut out, space_hz, sample_rate, bit, &mut phase);
    // 5 位数据，低位在先。
    for b in 0..5 {
      let f = if (code >> b) & 1 == 1 {
        mark_hz
      } else {
        space_hz
      };
      tone(&mut out, f, sample_rate, bit, &mut phase);
    }
    // 1.5 位停止 = mark（逻辑 1）。
    tone(&mut out, mark_hz, sample_rate, bit + bit / 2, &mut phase);
    if out.len() > MAX_SAMPLES {
      return Vec::new();
    }
  }
  out
}

/// 16 位单声道 PCM 的 WAV 编码（44 字节头 + 采样数据）。
///
/// 编码实现只有一份，在 [`ham_web_wav::encode`]（样本生成器、worker 与工具链都调它）；
/// 这里保留薄封装，是为了让「合成音频的页面」只依赖 core 而不必再引一个 crate。
#[must_use]
pub fn to_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
  ham_web_wav::encode(samples, sample_rate)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 近似数零交叉：估算一段采样的主频（Hz）。
  fn estimate_hz(samples: &[f32], sample_rate: u32) -> f64 {
    let crossings = samples
      .windows(2)
      .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
      .count();
    crossings as f64 * sample_rate as f64 / (2.0 * (samples.len() - 1) as f64)
  }

  #[test]
  fn cw_dot_dash_durations() {
    let sr = 8000;
    let unit = (sr as f64 * 1.2 / 20.0).round() as usize; // 20 WPM → 480 样本
    // E = 单个点：1 单位开 + 1 单位关。
    assert_eq!(cw_samples("E", 20.0, 700.0, sr).len(), unit * 2);
    // T = 单个划：3 单位开 + 1 单位关。
    assert_eq!(cw_samples("T", 20.0, 700.0, sr).len(), unit * 4);
    // SOS：S=3 点（6 单位）+字符间 2 = 8；O=3 划（12 单位）+字符间 2 = 14；末 S=6。共 28 单位。
    let sos = cw_samples("SOS", 20.0, 700.0, sr).len();
    assert_eq!(sos, unit * 28);
  }

  #[test]
  fn cw_keying_has_soft_edges_but_full_amplitude() {
    // 矩形键控的咔嗒声：开键第一个 / 最后一个采样必须从 0 起落，中段仍是满幅。
    let sr = 8000;
    let unit = (sr as f64 * 1.2 / 20.0).round() as usize;
    let t = cw_samples("T", 20.0, 700.0, sr); // 3 单位开 + 1 单位关
    let on = &t[..unit * 3];
    assert!(on[0].abs() < 1e-6, "起始沿没有从 0 起：{}", on[0]);
    assert!(
      on[on.len() - 1].abs() < 0.05,
      "结束沿没有落回 0：{}",
      on[on.len() - 1]
    );
    // 前 5 ms（40 采样）里幅度单调爬升，不能一上来就满幅。
    let head_peak = on[..8].iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(head_peak < 0.3, "前 1 ms 已经接近满幅：{head_peak}");
    let mid_peak = on[unit..unit * 2]
      .iter()
      .fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(mid_peak > 0.99, "中段不是满幅：{mid_peak}");
    // 包络落在音段内部，总时长不变。
    assert_eq!(t.len(), unit * 4);
  }

  #[test]
  fn cw_edges_survive_very_short_elements() {
    // 40 WPM、8 kHz：一个点只有 240 样本；包络超过半个音段时必须按一半截短而不是越界。
    let out = cw_samples("E", 400.0, 700.0, 8000);
    assert!(!out.is_empty() && out.iter().all(|s| s.is_finite()));
  }

  #[test]
  fn cw_uses_tone_frequency() {
    // 取 E 的开键段（前 unit 样本）估频，应接近 700 Hz。
    let sr = 8000;
    let unit = (sr as f64 * 1.2 / 20.0).round() as usize;
    let e = cw_samples("E", 20.0, 700.0, sr);
    let f = estimate_hz(&e[..unit], sr);
    assert!((f - 700.0).abs() < 30.0, "f={f}");
  }

  #[test]
  fn rtty_bit_duration_and_frequencies() {
    let sr = 8000;
    let baud = 45.45;
    let bit = (sr as f64 / baud).round() as usize; // 176
    // A = 起始 1 + 数据 5 + 停止 1.5 = 7.5 位。
    let a = rtty_samples("A", baud, 2125.0, 2295.0, sr);
    assert_eq!(a.len(), bit * 7 + bit / 2);
    // 起始位是 space（2295 Hz）。
    let start_hz = estimate_hz(&a[..bit], sr);
    assert!((start_hz - 2295.0).abs() < 60.0, "start={start_hz}");
    // 停止位是 mark（2125 Hz）：最后 1.5 位。
    let stop = &a[a.len() - (bit + bit / 2)..];
    let stop_hz = estimate_hz(stop, sr);
    assert!((stop_hz - 2125.0).abs() < 60.0, "stop={stop_hz}");
  }

  #[test]
  fn rtty_inserts_figs_shift() {
    // "A1"：A + FIGS 移态 + 1，共 3 个字符，总 3×7.5 位。
    let sr = 8000;
    let bit = (sr as f64 / 45.45).round() as usize;
    let out = rtty_samples("A1", 45.45, 2125.0, 2295.0, sr);
    assert_eq!(out.len(), 3 * (bit * 7 + bit / 2));
  }

  #[test]
  fn wav_header_and_payload() {
    let sr = 8000;
    let samples = vec![0.0f32, 0.5, -0.5, 1.0];
    let wav = to_wav(&samples, sr);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[36..40], b"data");
    // 数据长度 = 4 样本 × 2 字节。
    assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 8);
    assert_eq!(wav.len(), 44 + 8);
  }

  #[test]
  fn invalid_params_return_empty() {
    assert!(cw_samples("E", 0.0, 700.0, 8000).is_empty());
    assert!(cw_samples("E", 20.0, 0.0, 8000).is_empty());
    assert!(rtty_samples("A", 0.0, 2125.0, 2295.0, 8000).is_empty());
    assert!(rtty_samples("A", 45.45, 2125.0, 2295.0, 0).is_empty());
  }

  #[test]
  fn absurd_parameters_return_empty_instead_of_allocating() {
    // 单位 / 位长超过 1 秒：乘上字符数就是几百 MB，必须在分配前拒绝。
    assert!(cw_samples("E", 0.5, 700.0, 8000).is_empty());
    assert!(rtty_samples("A", 0.5, 2125.0, 2295.0, 8000).is_empty());
    // 报文过长同理（上限见 `MAX_TEXT_CHARS`）。
    let long = "E".repeat(MAX_TEXT_CHARS + 1);
    assert!(cw_samples(&long, 20.0, 700.0, 8000).is_empty());
    assert!(rtty_samples(&long, 45.45, 2125.0, 2295.0, 8000).is_empty());
  }

  #[test]
  fn non_finite_parameters_are_rejected() {
    // `NaN <= 0.0` 恒为 false：不显式挡掉就会整段采样变 NaN、调用方拿到「成功但无声」。
    assert!(cw_samples("CQ", 20.0, f64::NAN, 8000).is_empty());
    assert!(cw_samples("CQ", f64::NAN, 700.0, 8000).is_empty());
    assert!(cw_samples("CQ", 20.0, f64::INFINITY, 8000).is_empty());
    assert!(rtty_samples("CQ", 45.45, f64::NAN, 2295.0, 8000).is_empty());
    assert!(rtty_samples("CQ", 45.45, 2125.0, f64::INFINITY, 8000).is_empty());
  }

  #[test]
  fn output_beyond_the_sample_cap_is_rejected() {
    // 字符数、单位长度都在各自上限内，但乘积超限：必须提前收手，而不是先分配几百 MB。
    let text = "E".repeat(MAX_TEXT_CHARS);
    assert!(
      cw_samples(&text, 1.2, 700.0, 48_000).is_empty(),
      "一秒长的单位和 1024 个字符相乘已远超 MAX_SAMPLES"
    );
    assert!(rtty_samples(&text, 0.1, 2125.0, 2295.0, 48_000).is_empty());
  }

  #[test]
  fn whitespace_only_text_is_capped_too() {
    // 空格分支走 `continue`、跳过了字符末尾的 `MAX_SAMPLES` 检查，纯空格长文本在 48 kHz
    // 下会绕过兜底累计到几十 MB：循环头 + 循环尾的双重检查必须拦住。
    let text = " ".repeat(MAX_TEXT_CHARS);
    assert!(cw_samples(&text, 20.0, 700.0, 48_000).is_empty());
  }

  #[test]
  fn absurd_sample_rate_is_rejected() {
    assert!(cw_samples("E", 20.0, 700.0, MAX_SAMPLE_RATE + 1).is_empty());
    assert!(rtty_samples("A", 45.45, 2125.0, 2295.0, MAX_SAMPLE_RATE + 1).is_empty());
  }
}

//! NAVTEX（518 / 490 kHz 海事安全电传）解码：CCIR 476 七位码 + 100 波特 FSK
//! + ZCZC/NNNN 报文帧 + DX/RX 双副本纠错。
//!
//! 纯 Rust 实现、无外部依赖，可同时用于 WASM 前端与命令行/测试。
//!
//! # 协议速览
//!
//! - 100 波特 FSK、标称 170 Hz 频移（真实录音的电台调谐各异，解调前自动估计音调）；
//! - 每个字符是 CCIR 476 的 7 位码（恰含 4 个 mark 位 → 权重 ≠ 4 即检出错误）；
//! - 广播帧：相位段 → `ZCZC B1B2B3B4` 报头 → 正文 → `NNNN`；
//!   DX / RX 字符交替，副本相隔五个七位字符。去交织纠错后，完整报文的残余
//!   字符错误率 < 4% 才接受。
//!
//! # 验证
//!
//! 单测覆盖码表自洽（全部 35 个码权重 4）、合成样本端到端（含注入位错）；
//! 另有 SigidWiki 两段真实录音的互操作测试（`samples/`，来源与许可见
//! `samples/README.md`）。

pub mod codec;
pub mod demod;
pub mod message;

pub use codec::{ALPHA, BETA, FIGS, InvalidCode, LTRS, REP, decode_code, is_valid};
pub use demod::{decimate_for_demod, demodulate, demodulate_at, estimate_tones, sync_phase};
pub use message::{NavtexMessage, decode_messages, find_zczc};

/// 完整解码：音频样本 → 报文列表。
///
/// 音调自动估计（Mark 低于 Space）；两条极性都试（估计的音调可能把 Mark / Space
/// 判反）。返回的报文按流内先后排序。
#[must_use]
pub fn decode_audio(samples: &[f32], rate: u32) -> Option<Vec<NavtexMessage>> {
  // 音调估计是 O(bins × rate)、相位同步随采样率平方增长：44.1 / 48 / 96 kHz 的上传先
  // 抽取到 ≥ 12 kHz（NAVTEX 音调都在 3.6 kHz 以下），worker 阻塞时间从十几秒降到数百毫秒。
  let (samples, rate) = decimate_for_demod(samples, rate);
  let (samples, rate) = (samples.as_ref(), rate);
  let (lo, hi) = estimate_tones(samples, rate)?;
  let bits = demodulate(samples, rate, lo, hi);
  let messages = decode_messages(&bits);
  (!messages.is_empty()).then_some(messages)
}

/// 合成一段 NAVTEX 广播（测试与 `cargo make navtex-sample` 用）。
///
/// 结构：REP / ALPHA 相位段（`phasing_secs` 秒）→ 含标准报头及 NNNN 的
/// DX / RX 字符交织流（副本相隔五个字符）。采样率 8000、Mark 1904、
/// Space 2104（与 SigidWiki 长样本同一调谐）。
#[must_use]
pub fn synthesize(text: &str, phasing_secs: f64, noise: f32) -> Vec<f32> {
  synthesize_at(text, phasing_secs, noise, 8000)
}

/// 同 [`synthesize`]，采样率可选（测试用：验证高采样率输入的抽取路径）。
fn synthesize_at(text: &str, phasing_secs: f64, noise: f32, rate: u32) -> Vec<f32> {
  const MARK: f64 = 1904.0;
  const SPACE: f64 = 2104.0;
  let mut bits = Vec::new();
  // REP / ALPHA 交替标明两条通道；尾部重复码相对首次发送延迟五个字符。
  // 上限防止 `phasing_secs` 极大（公共 API）时把码字向量撑到 GB 级。
  const MAX_PHASING_PAIRS: usize = 10_000;
  let pairs = ((phasing_secs.max(0.0) * 100.0 / 14.0) as usize).min(MAX_PHASING_PAIRS);
  for _ in 0..pairs.max(3) {
    push_code(&mut bits, REP);
    push_code(&mut bits, ALPHA);
  }
  let data = message_codes('F', 'A', 79, text);
  for i in 0..data.len() + 2 {
    push_code(&mut bits, data.get(i).copied().unwrap_or(codec::BLK));
    push_code(
      &mut bits,
      i.checked_sub(2)
        .and_then(|j| data.get(j))
        .copied()
        .unwrap_or(ALPHA),
    );
  }
  // 位流 → FSK 采样。
  let spb = f64::from(rate) / 100.0;
  let mut out = Vec::new();
  let mut seed = 0x1234_5678u32;
  for &b in &bits {
    let f = if b { MARK } else { SPACE };
    for _ in 0..spb as usize {
      let t = out.len() as f64 / f64::from(rate);
      let mut v = (2.0 * std::f64::consts::PI * f * t).sin() as f32;
      if noise > 0.0 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let u = (seed >> 8) as f32 / 16_777_216.0;
        v += noise * (u * 2.0 - 1.0);
      }
      out.push(v);
    }
  }
  out
}

/// 标准报头包含空格、换挡及换行；结束前切回字母档以发送 NNNN。
///
/// `serial` 是报头 B3B4 两位序号，NAVTEX 规格上就是 00–99：`>99` 会输出三位数
/// 破坏报头宽度。当前调用方（合成工具）写死 79，仅 `debug_assert` 提醒。
fn message_codes(b1: char, b2: char, serial: u8, text: &str) -> Vec<u8> {
  debug_assert!(serial <= 99, "NAVTEX 序号是两位（00–99）");
  let mut codes = vec![LTRS];
  let mut figs = false;
  for ch in format!("ZCZC {b1}{b2}{serial:02}\r\n{text}\r\nNNNN").chars() {
    let Some((code, want_figs)) = codec::encode_char(ch) else {
      continue;
    };
    if want_figs != figs {
      codes.push(if want_figs { FIGS } else { LTRS });
      figs = want_figs;
    }
    codes.push(code);
  }
  codes
}

fn push_code(out: &mut Vec<bool>, code: u8) {
  for j in 0..7 {
    out.push(code & (1 << j) != 0);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn synthesized_interleaved_audio_recovers_the_complete_message() {
    let text = "GALE WARNING IN SOUTHERN ADRIATIC 25 KNOTS";
    for noise in [0.0, 0.02] {
      let samples = synthesize(text, 1.2, noise);
      let samples: Vec<f32> = samples
        .iter()
        .map(|sample| (sample * 32000.0).round().clamp(-32767.0, 32767.0) / 32768.0)
        .collect();
      let messages = decode_audio(&samples, 8000).expect("应恢复完整报文");
      assert_eq!(messages.len(), 1);
      assert_eq!(messages[0].text, text);
      assert_eq!(messages[0].serial, 79);
      assert!(messages[0].accepted);
    }
  }

  #[test]
  fn high_sample_rates_are_decimated_and_still_decode() {
    // R2-16：44.1 / 48 / 96 kHz 上传先抽取再解调，结果与 8 kHz 一致。
    let text = "GALE WARNING IN SOUTHERN ADRIATIC 25 KNOTS";
    for rate in [48_000u32, 44_100, 96_000] {
      let samples = synthesize_at(text, 1.2, 0.01, rate);
      let messages = decode_audio(&samples, rate).expect("高采样率应恢复报文");
      assert_eq!(messages.len(), 1, "rate={rate}");
      assert_eq!(messages[0].text, text, "rate={rate}");
    }
  }
}

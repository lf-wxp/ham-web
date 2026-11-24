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

/// 单条报文正文的字符上限。
///
/// 与 `ham_web_core::mode_audio::MAX_TEXT_CHARS` 取同一个值（100 波特下 1024 字符已约
/// 70 秒音频、约 60 万样本）；页面必须先按这个值校验，否则一段长粘贴就能生成几百 MB
/// 的基带采样。
pub const MAX_MESSAGE_CHARS: usize = 1024;

/// 发端合成的基带采样率（Hz）。
///
/// Worker 把它连同样本一起回给页面，页面写 WAV 头时用这个值，不再自己写一份 8000。
pub const SAMPLE_RATE: u32 = 8000;

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

/// 合成一条**指定报头**的 NAVTEX 广播（发端用：8 kHz、无噪声）。
///
/// `b1` 是台站识别字母、`b2` 是电报类型字母、`serial` 是两位序号 —— 三者都写进
/// `ZCZC B1B2SS` 报头，解码端会原样读回来（见 [`NavtexMessage`]）。
#[must_use]
pub fn synthesize_message(
  b1: char,
  b2: char,
  serial: u8,
  text: &str,
  phasing_secs: f64,
) -> Vec<f32> {
  let bits = broadcast_bits(b1, b2, serial, text, phasing_secs);
  render_fsk(&bits, SAMPLE_RATE, 0.0)
}

/// 相位段时长上限（秒）：再长的前导没有意义，也挡住把码字向量撑大的入参。
pub const MAX_PHASING_SECS: f64 = 60.0;

/// 发端入参不合法。[`EncodeError::code`] 是 worker 回给页面的英文错误码
/// （`encode-params:<字段>`，见 `docs/domain/decode-worker-protocol.md`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
  /// 台站识别字母不是恰好一个 ASCII 字母。
  Station,
  /// 电报类型字母不是恰好一个 ASCII 字母。
  Category,
  /// 序号不是 0–99 的整数。
  Serial,
  /// 正文为空（只有空白）或超过 [`MAX_MESSAGE_CHARS`]。
  Text,
  /// 相位段时长不在 0 – [`MAX_PHASING_SECS`] 秒内。
  Phasing,
}

impl EncodeError {
  /// 英文错误码（`encode-params:<字段>`）。
  #[must_use]
  pub fn code(self) -> &'static str {
    match self {
      Self::Station => "encode-params:station",
      Self::Category => "encode-params:category",
      Self::Serial => "encode-params:serial",
      Self::Text => "encode-params:text",
      Self::Phasing => "encode-params:phasing",
    }
  }
}

/// 恰好一个 ASCII 字母 → 大写字母。
fn single_letter(value: &str) -> Option<char> {
  let mut chars = value.trim().chars();
  let c = chars.next()?;
  (chars.next().is_none() && c.is_ascii_alphabetic()).then(|| c.to_ascii_uppercase())
}

/// 校验后合成一条 NAVTEX 广播：Worker 的 `encode` 入口。
///
/// 入参保持 JS 传来的原始形态（字符串 / 数字）：校验只在这一处，页面与 worker 都不必各写一份。
/// 序号必须是整数（`42.5`、`NaN`、负数都拒绝）；正文先看长度再合成 ——
/// [`synthesize_message`] 本身不设上限，调用方不校验就能生成几百 MB 的采样。
///
/// # Errors
///
/// 任一入参不合法时返回对应字段的 [`EncodeError`]。
pub fn encode_message(
  station: &str,
  category: &str,
  serial: f64,
  text: &str,
  phasing_secs: f64,
) -> Result<Vec<f32>, EncodeError> {
  let b1 = single_letter(station).ok_or(EncodeError::Station)?;
  let b2 = single_letter(category).ok_or(EncodeError::Category)?;
  if !serial.is_finite() || serial.fract() != 0.0 || !(0.0..=99.0).contains(&serial) {
    return Err(EncodeError::Serial);
  }
  if text.trim().is_empty() || text.chars().count() > MAX_MESSAGE_CHARS {
    return Err(EncodeError::Text);
  }
  if !phasing_secs.is_finite() || !(0.0..=MAX_PHASING_SECS).contains(&phasing_secs) {
    return Err(EncodeError::Phasing);
  }
  Ok(synthesize_message(b1, b2, serial as u8, text, phasing_secs))
}

/// 同 [`synthesize`]，采样率可选（测试用：验证高采样率输入的抽取路径）。
fn synthesize_at(text: &str, phasing_secs: f64, noise: f32, rate: u32) -> Vec<f32> {
  let bits = broadcast_bits('F', 'A', 79, text, phasing_secs);
  render_fsk(&bits, rate, noise)
}

/// 相位段 + 报头 + 正文 + NNNN 的完整位流。
fn broadcast_bits(b1: char, b2: char, serial: u8, text: &str, phasing_secs: f64) -> Vec<bool> {
  let mut bits = Vec::new();
  // REP / ALPHA 交替标明两条通道；尾部重复码相对首次发送延迟五个字符。
  // 上限防止 `phasing_secs` 极大（公共 API）时把码字向量撑到 GB 级。
  const MAX_PHASING_PAIRS: usize = 10_000;
  let pairs = ((phasing_secs.max(0.0) * 100.0 / 14.0) as usize).min(MAX_PHASING_PAIRS);
  for _ in 0..pairs.max(3) {
    push_code(&mut bits, REP);
    push_code(&mut bits, ALPHA);
  }
  let data = message_codes(b1, b2, serial, text);
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
  bits
}

/// 位流 → 100 波特 FSK 采样。
fn render_fsk(bits: &[bool], rate: u32, noise: f32) -> Vec<f32> {
  const MARK: f64 = 1904.0;
  const SPACE: f64 = 2104.0;
  if rate == 0 {
    return Vec::new();
  }
  let spb = f64::from(rate) / 100.0;
  let mut out = Vec::new();
  let mut seed = 0x1234_5678u32;
  for &b in bits {
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
  // 100 波特下每字符约 0.07 s、约 560 个基带样本；正文没有上限时，一段几 MB 的粘贴
  // 就能生成 GB 级采样。页面必须按 `MAX_MESSAGE_CHARS` 先校验，这里再断一次，
  // 防止新的调用方绕过页面校验。
  debug_assert!(
    text.chars().count() <= MAX_MESSAGE_CHARS,
    "NAVTEX 正文超过 {MAX_MESSAGE_CHARS} 字符：调用方必须先校验"
  );
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

  #[test]
  fn synthesized_message_keeps_the_requested_header() {
    let text = "GALE WARNING IN SOUTHERN ADRIATIC 25 KNOTS";
    let samples = synthesize_message('F', 'B', 42, text, 2.0);
    let messages = decode_audio(&samples, 8000).expect("应恢复报文");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].transmitter, 'F');
    assert_eq!(messages[0].category, 'B');
    assert_eq!(messages[0].serial, 42);
    assert_eq!(messages[0].text, text);
    assert!(messages[0].accepted);
  }

  /// 校验后的入口与 `synthesize_message` 逐样本相同，报头原样读回；
  /// 小写字母与首尾空白被规整（页面不再替它做一遍）。
  #[test]
  fn encode_message_matches_synthesize_message_and_normalises_letters() {
    let text = "GALE WARNING IN SOUTHERN ADRIATIC 25 KNOTS";
    let samples = encode_message(" f ", "b", 42.0, text, 2.0).unwrap();
    assert_eq!(samples, synthesize_message('F', 'B', 42, text, 2.0));
    let messages = decode_audio(&samples, 8000).expect("应恢复报文");
    assert_eq!(
      (
        messages[0].transmitter,
        messages[0].category,
        messages[0].serial
      ),
      ('F', 'B', 42)
    );
  }

  #[test]
  fn encode_message_rejects_each_bad_field_with_its_own_code() {
    let ok = |station, category, serial, text: &str, phasing| {
      encode_message(station, category, serial, text, phasing)
    };
    assert_eq!(ok("", "A", 1.0, "X", 2.0), Err(EncodeError::Station));
    assert_eq!(ok("FF", "A", 1.0, "X", 2.0), Err(EncodeError::Station));
    assert_eq!(ok("1", "A", 1.0, "X", 2.0), Err(EncodeError::Station));
    assert_eq!(ok("F", "你", 1.0, "X", 2.0), Err(EncodeError::Category));
    for bad in [-1.0, 100.0, 42.5, f64::NAN, f64::INFINITY] {
      assert_eq!(
        ok("F", "A", bad, "X", 2.0),
        Err(EncodeError::Serial),
        "{bad}"
      );
    }
    // 序号边界 0 与 99 合法。
    assert!(ok("F", "A", 0.0, "X", 2.0).is_ok());
    assert!(ok("F", "A", 99.0, "X", 2.0).is_ok());
    assert_eq!(ok("F", "A", 1.0, "", 2.0), Err(EncodeError::Text));
    assert_eq!(ok("F", "A", 1.0, "  \n", 2.0), Err(EncodeError::Text));
    let too_long = "A".repeat(MAX_MESSAGE_CHARS + 1);
    assert_eq!(ok("F", "A", 1.0, &too_long, 2.0), Err(EncodeError::Text));
    assert!(ok("F", "A", 1.0, &"A".repeat(MAX_MESSAGE_CHARS), 0.0).is_ok());
    for bad in [-0.1, MAX_PHASING_SECS + 0.1, f64::NAN, f64::INFINITY] {
      assert_eq!(
        ok("F", "A", 1.0, "X", bad),
        Err(EncodeError::Phasing),
        "{bad}"
      );
    }
  }

  #[test]
  fn encode_error_codes_have_the_protocol_shape() {
    for e in [
      EncodeError::Station,
      EncodeError::Category,
      EncodeError::Serial,
      EncodeError::Text,
      EncodeError::Phasing,
    ] {
      assert!(e.code().starts_with("encode-params:"), "{e:?}");
    }
  }
}

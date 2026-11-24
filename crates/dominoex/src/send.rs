//! DominoEX 发端入口校验：Worker 的 `encode` 调这里。
//!
//! 校验规则（可发字符、长度、样本上限）与 MFSK / THOR 共用，住在 `ham_web_mfsk::send`；
//! 这里只查 DominoEX 自己的子模式并调 `encode_audio` / `encode_audio_fec`。
//! 为什么要走 Worker 见那份文件的头部（GPL-3.0，不能链进 MIT 的主包）。

use ham_web_mfsk::{EncodeError, sendable_text, within_budget};

use crate::{encode_audio, encode_audio_fec, parse_mode};

/// 发端合成的基带采样率（Hz）：fldigi 里 DominoEX 全部子模式的标称采样率。
///
/// 信号中心固定在 1000 Hz（`BASEFREQ`，不像 MFSK / THOR 那样可调），最宽的 DominoEX 22 的音调带
/// 上沿约 1.2 kHz，8 kHz 下远低于 Nyquist。
pub const ENCODE_SAMPLE_RATE: u32 = 8000;

/// 前后各补的空闲符时长（秒）：让接收端先锁定、并把交织器与回溯里压着的最后几个字符推出来。
const IDLE_SECS: f64 = 1.0;

/// 校验后合成一段 DominoEX 基带，返回 `(样本, 采样率)`。`fec` 为真时用 MultiPSK 兼容的 FEC 模式
/// （调制完全相同，只是 nibble 的含义变了，**收发两端必须选同一种**）。
///
/// # Errors
///
/// 子模式不在表内、报文不可发或过长。
pub fn encode_message(
  text: &str,
  mode_name: &str,
  fec: bool,
) -> Result<(Vec<f32>, u32), EncodeError> {
  let (baud, spacing) =
    parse_mode(mode_name).ok_or_else(|| EncodeError::UnsupportedSubmode(mode_name.to_owned()))?;
  let text = sendable_text(text)?;
  let encode = if fec { encode_audio_fec } else { encode_audio };
  let samples = within_budget(encode(&text, baud, spacing, ENCODE_SAMPLE_RATE, IDLE_SECS))?;
  Ok((samples, ENCODE_SAMPLE_RATE))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{MODES, decode_audio, decode_audio_fec};

  /// 每个子模式、两种编码方式都能合成，并在解码器里读回原文（DominoEX 的往返很快，全量跑）。
  #[test]
  fn every_mode_round_trips_with_and_without_fec() {
    for (name, ..) in MODES {
      let (baud, spacing) = parse_mode(name).unwrap();
      for fec in [false, true] {
        let (samples, rate) = encode_message("CQ CQ DE BG1AAA", name, fec)
          .unwrap_or_else(|e| panic!("DominoEX {name} fec={fec}: {e:?}"));
        assert_eq!(rate, ENCODE_SAMPLE_RATE);
        assert!(samples.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
        let decode = if fec { decode_audio_fec } else { decode_audio };
        let (text, _id) = decode(&samples, rate, baud, spacing)
          .unwrap_or_else(|| panic!("DominoEX {name} fec={fec}: 没有解出信号"));
        assert!(
          text.contains("CQ CQ DE BG1AAA"),
          "DominoEX {name} fec={fec}: {text:?}"
        );
      }
    }
  }

  /// FEC 与非 FEC 的音频不同：页面把「是否 FEC」当成一个必须与对端一致的选项，不是装饰。
  #[test]
  fn fec_changes_the_audio() {
    let plain = encode_message("CQ DE BG1AAA", "8", false).unwrap();
    let fec = encode_message("CQ DE BG1AAA", "8", true).unwrap();
    assert_ne!(plain.0, fec.0);
  }

  #[test]
  fn bad_parameters_are_reported_with_their_own_codes() {
    assert_eq!(
      encode_message("CQ", "99", false).unwrap_err(),
      EncodeError::UnsupportedSubmode("99".into())
    );
    assert_eq!(
      encode_message("你好", "8", false).unwrap_err(),
      EncodeError::Text
    );
    assert_eq!(
      encode_message("  ", "8", true).unwrap_err(),
      EncodeError::Text
    );
    assert_eq!(
      encode_message(&"A".repeat(1025), "8", false).unwrap_err(),
      EncodeError::Text
    );
  }
}

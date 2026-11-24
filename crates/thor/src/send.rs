//! THOR 发端入口校验：Worker 的 `encode` 调这里。
//!
//! 校验规则（可发字符、长度、中心频率、样本上限）与 MFSK / DominoEX 共用，住在
//! `ham_web_mfsk::send`；这里只查 THOR 自己的子模式并调 THOR 的 `encode_audio`。
//! 为什么要走 Worker 见那份文件的头部（GPL-3.0，不能链进 MIT 的主包）。

use ham_web_mfsk::picture::Header;
use ham_web_mfsk::{EncodeError, lead_in_text, resolve_center, sendable_text, within_budget};

use crate::picture::{SPP, SYNC_OFFSET, code_for};
use crate::{center_in_range, encode_audio, encode_picture_audio, parse_mode};

/// ID 信标文本的字符数上限：fldigi 空闲时发的二次文本是一句台号，几十个字符绰绰有余。
pub const MAX_ID_CHARS: usize = 64;

/// ID 信标里能发的字符：THOR 二次信道的码表只覆盖 `' '`（0x20）到 `'z'`（0x7A），
/// 控制字符（含换行）与 `{ | } ~` 都不在表里，一律跳过。
///
/// # Errors
///
/// 原文超过 [`MAX_ID_CHARS`] 个字符。
pub fn sendable_id(id: &str) -> Result<String, EncodeError> {
  if id.chars().count() > MAX_ID_CHARS {
    return Err(EncodeError::Id);
  }
  Ok(id.chars().filter(|c| matches!(c, ' '..='z')).collect())
}

/// 主信道文本 + ID 信标：每个 ID 字符前面带一个 `\x01`（二次信道的开关，其后的字节走二次信道），
/// 排在正文之后、`\r EOT \r` 之前。ID 为空时就是原文。
fn with_id(text: &str, id: &str) -> String {
  let mut out = text.to_owned();
  for c in id.chars() {
    out.push('\u{1}');
    out.push(c);
  }
  out
}

/// 校验后合成一段 THOR 基带，返回 `(样本, 采样率)`。采样率取该模式的标称值
/// （多数 8 kHz，THOR 5 / 11 / 22 / 44 为 11.025 kHz，THOR 56 为 16 kHz）；中心频率不给就用缺省。
///
/// `id` 是 ID 信标（二次信道）文本，可以为空；接收端把它与正文分开显示。
/// **主信道报文里的 `\x01` 会被过滤掉**：它是二次信道的开关，用户文本不该意外触发它 ——
/// 想发 ID 只能走 `id` 参数。
///
/// # Errors
///
/// 子模式不在表内、中心放不下、报文不可发或过长、ID 过长。
pub fn encode_message(
  text: &str,
  mode_name: &str,
  center_hz: Option<f64>,
  id: &str,
) -> Result<(Vec<f32>, u32), EncodeError> {
  let mode =
    parse_mode(mode_name).ok_or_else(|| EncodeError::UnsupportedSubmode(mode_name.to_owned()))?;
  let text = sendable_text(text)?;
  let id = sendable_id(id)?;
  let rate = mode.rate;
  let center = resolve_center(center_hz, rate, |c| center_in_range(mode, rate, c))?;
  let samples = within_budget(encode_audio(&with_id(&text, &id), mode, rate, center))?;
  Ok((samples, rate))
}

/// THOR 图片模式认得的尺寸：`(宽, 高)`，彩色与灰度各有一个报头码（`code_for`）。
///
/// 与 fldigi 的 `pic%X` 报头一致；页面持有一份同样的名单（`codec_modes::THOR_PICTURE_SIZES`），
/// 两边对不上时 Worker 会回 `encode-params:picture`，协议 e2e 把名单逐个送进去。
pub const PICTURE_SIZES: &[(usize, usize)] = &[
  (59, 74),
  (120, 150),
  (160, 120),
  (240, 300),
  (320, 240),
  (640, 480),
];

/// THOR：校验后合成一次图片发射（引导文本 + `pic%X` 报头 + 同步音 + 模拟调频图像），返回 `(样本, 采样率)`。
///
/// 只有 THOR 11 / 16 / 22 / 25 / 32 / 44 / 56 有图片模式，尺寸只能是 [`PICTURE_SIZES`] 里的六种
/// （彩色 / 灰度各有报头码），每个像素固定 10 个样本。`rgb` 是 `宽 × 高 × 3` 字节；灰度时取 R 通道。
/// `text` 是报头前面的引导文本，可以为空。
///
/// # Errors
///
/// 子模式不在表内、没有图片模式、尺寸不被支持或像素数据对不上、中心放不下（含同步音会掉到 0 Hz 以下）、
/// 引导文本过长、合成后超限（640 × 480 彩色在 8 kHz 下就超了）。
pub fn encode_picture_message(
  text: &str,
  mode_name: &str,
  center_hz: Option<f64>,
  (width, height): (usize, usize),
  color: bool,
  rgb: &[u8],
) -> Result<(Vec<f32>, u32), EncodeError> {
  let mode =
    parse_mode(mode_name).ok_or_else(|| EncodeError::UnsupportedSubmode(mode_name.to_owned()))?;
  let header = Header {
    width,
    height,
    color,
    spp: SPP,
  };
  if !mode.has_picture()
    || code_for(&header).is_none()
    || width.checked_mul(height).and_then(|n| n.checked_mul(3)) != Some(rgb.len())
  {
    return Err(EncodeError::Picture);
  }
  let text = lead_in_text(text)?;
  let rate = mode.rate;
  // 同步音在中心 − 0.6 × 带宽，比最低音调还低：只查音调带不够，它会先掉到 0 Hz 以下。
  let center = resolve_center(center_hz, rate, |c| {
    center_in_range(mode, rate, c) && c + SYNC_OFFSET * mode.bandwidth() >= 0.0
  })?;
  let samples = within_budget(encode_picture_audio(
    &text, &header, rgb, mode, rate, center,
  ))?;
  Ok((samples, rate))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{MODES, decode_audio};

  /// 不带 ID 的合成（旧测试的入口）。
  fn enc(text: &str, mode: &str, center: Option<f64>) -> Result<(Vec<f32>, u32), EncodeError> {
    encode_message(text, mode, center, "")
  }

  #[test]
  fn every_mode_encodes_with_the_default_center_at_its_nominal_rate() {
    for mode in MODES {
      let (samples, rate) =
        enc("CQ", mode.name, None).unwrap_or_else(|e| panic!("THOR {}: {e:?}", mode.name));
      assert_eq!(rate, mode.rate, "THOR {}", mode.name);
      assert!(!samples.is_empty(), "THOR {}", mode.name);
      assert!(
        samples.iter().all(|v| v.is_finite() && v.abs() <= 1.0),
        "THOR {}",
        mode.name
      );
    }
  }

  /// 覆盖三类：K = 7（16）、K = 15 的高速模式（100）、标称 11.025 kHz（22）与 16 kHz（56）。
  #[test]
  fn synthesised_audio_decodes_back_to_the_text() {
    for name in ["16", "8", "22", "100", "56"] {
      let mode = parse_mode(name).unwrap();
      let (samples, rate) = enc("CQ CQ DE BG1AAA", name, None).unwrap();
      let decoded = decode_audio(&samples, rate, mode, None, false)
        .unwrap_or_else(|| panic!("THOR {name}: 没有解出信号"));
      assert!(
        decoded.text.contains("CQ CQ DE BG1AAA"),
        "THOR {name}: {:?}",
        decoded.text
      );
    }
  }

  /// `\x01` 是二次信道开关：用户文本里带它不能把后面的字节发到 ID 信标里去。
  #[test]
  fn a_stray_secondary_channel_marker_is_not_sent() {
    let plain = enc("CQ DE BG1AAA", "16", None).unwrap();
    let with_marker = enc("CQ \u{1}DE BG1AAA", "16", None).unwrap();
    assert_eq!(plain, with_marker);
  }

  #[test]
  fn bad_parameters_are_reported_with_their_own_codes() {
    assert_eq!(
      enc("CQ", "99", None).unwrap_err(),
      EncodeError::UnsupportedSubmode("99".into())
    );
    assert_eq!(
      enc("CQ", "16", Some(100_000.0)).unwrap_err(),
      EncodeError::CenterOutOfRange(100_000.0)
    );
    assert_eq!(enc("你好", "16", None).unwrap_err(), EncodeError::Text);
    // 宽模式在给定的中心上放不下就报错，不悄悄挪中心：THOR 100 的信号带约 1.8 kHz。
    assert!(enc("CQ", "100", Some(3500.0)).is_err());
    assert!(enc("CQ", "100", Some(1500.0)).is_ok());
  }

  // ---------- ID 信标（二次信道） ----------

  /// ID 走二次信道：解码端把它与正文分开，正文里不会混进 ID 的字符。
  #[test]
  fn the_id_beacon_is_decoded_on_the_secondary_channel_only() {
    for name in ["16", "8", "22"] {
      let mode = parse_mode(name).unwrap();
      let (samples, rate) = encode_message("CQ CQ TEST", name, None, "DE BG1AAA").unwrap();
      let decoded = decode_audio(&samples, rate, mode, None, false)
        .unwrap_or_else(|| panic!("THOR {name}: 没有解出信号"));
      assert!(
        decoded.text.contains("CQ CQ TEST"),
        "THOR {name}: {:?}",
        decoded.text
      );
      assert!(
        decoded.id.contains("DE BG1AAA"),
        "THOR {name}: {:?}",
        decoded.id
      );
      assert!(
        !decoded.text.contains("BG1AAA"),
        "THOR {name}: ID 混进了正文 {:?}",
        decoded.text
      );
    }
  }

  /// 空 ID 与不给 ID 逐样本相同；有 ID 的音频更长。
  #[test]
  fn an_empty_id_changes_nothing_and_a_real_one_adds_audio() {
    let plain = encode_message("CQ DE BG1AAA", "16", None, "").unwrap();
    assert_eq!(
      plain,
      encode_message("CQ DE BG1AAA", "16", None, "你好{}~\n").unwrap(),
      "ID 里全是发不了的字符，等于没给"
    );
    let with_id = encode_message("CQ DE BG1AAA", "16", None, "DE BG1AAA").unwrap();
    assert!(with_id.0.len() > plain.0.len());
  }

  #[test]
  fn the_id_filter_keeps_only_what_the_secondary_table_covers() {
    // 码表覆盖 0x20–0x7A：字母数字、空格与常见标点；`{ | } ~`、控制字符、非 ASCII 跳过。
    assert_eq!(sendable_id("DE BG1AAA/P 599!").unwrap(), "DE BG1AAA/P 599!");
    assert_eq!(sendable_id("a{b|c}d~e\nf\t你好z").unwrap(), "abcdefz");
    assert_eq!(sendable_id("").unwrap(), "");
    assert!(sendable_id(&"A".repeat(MAX_ID_CHARS)).is_ok());
    assert_eq!(
      sendable_id(&"A".repeat(MAX_ID_CHARS + 1)),
      Err(EncodeError::Id)
    );
    assert_eq!(
      encode_message("CQ", "16", None, &"A".repeat(MAX_ID_CHARS + 1)).unwrap_err(),
      EncodeError::Id
    );
  }

  /// 把 ID 当成主信道文本发 `\x01` 的路被堵死：用户文本里的 `\x01` 仍被过滤，ID 只能走 `id` 参数。
  #[test]
  fn a_stray_marker_in_the_text_still_cannot_open_the_secondary_channel() {
    let with_marker = encode_message("CQ \u{1}DE BG1AAA", "16", None, "").unwrap();
    let plain = encode_message("CQ DE BG1AAA", "16", None, "").unwrap();
    assert_eq!(with_marker, plain);
  }

  // ---------- 图片 ----------

  fn quadrants(w: usize, h: usize) -> Vec<u8> {
    let mut rgb = vec![0u8; w * h * 3];
    for y in 0..h {
      for x in 0..w {
        let px = [
          if x < w / 2 { 220 } else { 40 },
          if y < h / 2 { 180 } else { 70 },
          if x < w / 2 { 30 } else { 210 },
        ];
        rgb[3 * (y * w + x)..][..3].copy_from_slice(&px);
      }
    }
    rgb
  }

  /// 发出去的图片交给本 crate 的解码器：尺寸、颜色（FM 解调加低通，容差 ±4）、引导文本都读得回；
  /// 覆盖 K = 7（16）、11.025 kHz（22）与 16 kHz（56）。
  #[test]
  fn a_synthesised_picture_decodes_back_with_its_colours() {
    let (w, h) = (59, 74);
    let rgb = quadrants(w, h);
    for name in ["16", "22", "56"] {
      let mode = parse_mode(name).unwrap();
      let (samples, rate) = encode_picture_message("CQ ", name, None, (w, h), true, &rgb).unwrap();
      assert_eq!(rate, mode.rate, "THOR {name}");
      let decoded = decode_audio(&samples, rate, mode, None, false)
        .unwrap_or_else(|| panic!("THOR {name}: 没有解出信号"));
      assert_eq!(decoded.pictures.len(), 1, "THOR {name}");
      let got = &decoded.pictures[0];
      assert_eq!(
        (got.width, got.height, got.color),
        (w, h, true),
        "THOR {name}"
      );
      assert!(
        decoded.text.contains("CQ"),
        "THOR {name}: {:?}",
        decoded.text
      );
      for (x, y) in [(15, 18), (44, 18), (15, 55), (44, 55)] {
        for c in 0..3 {
          let want = rgb[3 * (y * w + x) + c];
          let have = got.rgb[3 * (y * w + x) + c];
          assert!(
            have.abs_diff(want) <= 4,
            "THOR {name} ({x},{y}) 通道 {c}: {have} vs {want}"
          );
        }
      }
    }
  }

  /// 灰度读回来是灰的；引导文本可以为空。
  #[test]
  fn a_grey_picture_without_lead_in_text() {
    let (w, h) = (59, 74);
    let rgb = quadrants(w, h);
    let grey: Vec<u8> = rgb.chunks(3).flat_map(|p| [p[0], p[0], p[0]]).collect();
    let (samples, rate) = encode_picture_message("", "16", None, (w, h), false, &grey).unwrap();
    let decoded = decode_audio(&samples, rate, parse_mode("16").unwrap(), None, false).unwrap();
    assert_eq!(decoded.pictures.len(), 1);
    assert!(!decoded.pictures[0].color);
  }

  #[test]
  fn only_the_six_known_sizes_and_the_picture_modes_are_accepted() {
    // 名单与 `code_for` 逐个对上：彩色与灰度都认。
    for &(w, h) in PICTURE_SIZES {
      for color in [true, false] {
        let header = Header {
          width: w,
          height: h,
          color,
          spp: SPP,
        };
        assert!(code_for(&header).is_some(), "{w}×{h} color={color}");
      }
    }
    assert_eq!(PICTURE_SIZES.len(), 6);
    // 不在名单里的尺寸（哪怕像素数对得上）拒绝。
    let rgb = vec![0u8; 100 * 100 * 3];
    assert_eq!(
      encode_picture_message("", "16", None, (100, 100), true, &rgb).unwrap_err(),
      EncodeError::Picture
    );
    // 只有 11 / 16 / 22 / 25 / 32 / 44 / 56 有图片模式。
    let ok = vec![0u8; 59 * 74 * 3];
    for name in ["micro", "4", "5", "8", "25x4", "50x1", "50x2", "100"] {
      assert_eq!(
        encode_picture_message("", name, None, (59, 74), true, &ok).unwrap_err(),
        EncodeError::Picture,
        "THOR {name}"
      );
    }
    for name in ["11", "16", "22", "25", "32", "44", "56"] {
      assert!(
        encode_picture_message("", name, None, (59, 74), true, &ok).is_ok(),
        "THOR {name}"
      );
    }
    // 像素数据对不上尺寸；子模式不存在。
    assert_eq!(
      encode_picture_message("", "16", None, (59, 74), true, &ok[1..]).unwrap_err(),
      EncodeError::Picture
    );
    assert_eq!(
      encode_picture_message("", "99", None, (59, 74), true, &ok).unwrap_err(),
      EncodeError::UnsupportedSubmode("99".into())
    );
  }

  /// 同步音在中心 − 0.6 × 带宽：中心太低时它会掉到 0 Hz 以下，要按中心越界报错，而不是静默失败。
  #[test]
  fn a_centre_that_drops_the_sync_tone_below_zero_is_rejected() {
    let ok = vec![0u8; 59 * 74 * 3];
    // THOR 56 的带宽约 993 Hz：中心 500 Hz 时音调带还放得下（最低约 31 Hz），同步音在 −96 Hz。
    assert_eq!(
      encode_picture_message("", "56", Some(500.0), (59, 74), true, &ok).unwrap_err(),
      EncodeError::CenterOutOfRange(500.0)
    );
    assert!(encode_picture_message("", "56", Some(1500.0), (59, 74), true, &ok).is_ok());
  }

  /// 640 × 480 彩色在 8 kHz 下要 920 万个样本，超过上限：是「太长」，不是尺寸错；灰度（307 万）能发。
  #[test]
  fn a_picture_over_the_sample_budget_is_reported_as_too_long() {
    let rgb = vec![128u8; 640 * 480 * 3];
    assert_eq!(
      encode_picture_message("", "16", None, (640, 480), true, &rgb).unwrap_err(),
      EncodeError::TooLong
    );
    assert!(encode_picture_message("", "16", None, (640, 480), false, &rgb).is_ok());
  }
}

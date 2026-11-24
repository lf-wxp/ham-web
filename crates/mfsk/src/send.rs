//! 发端入口校验：Worker 的 `encode` 调这里。
//!
//! MFSK / THOR / DominoEX 三个 crate 的发端校验规则一样（可发字符、长度、中心频率、样本上限），
//! 放在这一份里共用；每个 crate 只在自己的 `encode_message` 里查子模式并调自己的 `encode_audio`。
//! 校验只在这里做（有原生单测），Worker 与页面都不再抄一份规则。
//!
//! 为什么这些发端要走 Worker：三个 crate 都是 GPL-3.0-only（转写自 fldigi），链进 MIT 的主包
//! wasm 会让整个主包带上 GPL，见 `docs/engineering/licensing.md`。

/// 报文字符数上限，与页面共用的 `ham_web_core::mode_audio::MAX_TEXT_CHARS` 取同一个值。
///
/// 本 crate 是 GPL，不能依赖 MIT 的 core，所以数值在这里再写一次；页面先按它拦一遍，Worker 这里是兜底。
pub const MAX_TEXT_CHARS: usize = 1024;

/// 合成结果的样本数上限，与页面共用的 `ham_web_core::mode_audio::MAX_SAMPLES`（8 Mi）取同一个值：
/// 再长的音频要整块转移给页面、再拷成 WAV，浏览器标签页会先撑不住。
pub const MAX_ENCODE_SAMPLES: usize = 8 * 1024 * 1024;

/// 缺省的信号中心频率（Hz）：电台音频窗口的常用中心，全站的发端都用它。
pub const DEFAULT_CENTER_HZ: f64 = 1500.0;

/// 发端入参不合法。[`EncodeError::code`] 是 Worker 回给页面的英文错误码，沿用解码侧已有的码
/// （`unsupported-submode` / `center-hz-out-of-range`），页面的本地化与提示一份就够。
#[derive(Debug, Clone, PartialEq)]
pub enum EncodeError {
  /// 子模式不在标准表内（带上收到的名字）。
  UnsupportedSubmode(String),
  /// 中心频率放不下整个信号带（带上收到的值；非数字时是 `NaN`）。
  CenterOutOfRange(f64),
  /// 报文为空（过滤后没有可发字符）或超过 [`MAX_TEXT_CHARS`]。
  Text,
  /// 合成后的音频超过 [`MAX_ENCODE_SAMPLES`]（文本太长，或图片太大 / 太慢）。
  TooLong,
  /// 图片发端的入参不合法：这个子模式没有图片模式、尺寸不被支持、像素数据与尺寸对不上。
  Picture,
  /// THOR 的 ID 信标（二次信道）文本过长。
  Id,
}

impl EncodeError {
  /// 英文错误码。
  #[must_use]
  pub fn code(&self) -> String {
    match self {
      Self::UnsupportedSubmode(name) => format!("unsupported-submode:{name}"),
      Self::CenterOutOfRange(c) => format!("center-hz-out-of-range:{c}"),
      Self::Text => "encode-params:text".to_owned(),
      Self::TooLong => "encode-params:length".to_owned(),
      Self::Picture => "encode-params:picture".to_owned(),
      Self::Id => "encode-params:id".to_owned(),
    }
  }
}

/// 过滤出能发的字符：可打印 ASCII（0x20–0x7E）与换行，其余跳过。
///
/// 控制字符不发：它们在对端终端里会被当成控制动作，THOR 里的 `\x01` 还是二次信道的开关
/// （其后的字节会被当成 ID 信标）。非 ASCII 的字符按 UTF-8 逐字节发出去对端也只会读成乱码，
/// 与 CW / RTTY 一致：不认识的字符直接跳过，全军覆没才报错。
///
/// # Errors
///
/// 原文超过 [`MAX_TEXT_CHARS`] 个字符，或过滤后只剩空白时返回 [`EncodeError::Text`]。
pub fn sendable_text(text: &str) -> Result<String, EncodeError> {
  let kept = lead_in_text(text)?;
  if kept.trim().is_empty() {
    return Err(EncodeError::Text);
  }
  Ok(kept)
}

/// 同 [`sendable_text`]，但**允许空**：图片发射前面的引导文本（`CQ DE BG1AAA ` 之类）可有可无。
///
/// # Errors
///
/// 原文超过 [`MAX_TEXT_CHARS`] 个字符。
pub fn lead_in_text(text: &str) -> Result<String, EncodeError> {
  if text.chars().count() > MAX_TEXT_CHARS {
    return Err(EncodeError::Text);
  }
  Ok(
    text
      .chars()
      .filter(|c| matches!(c, ' '..='~' | '\n'))
      .collect(),
  )
}

/// 决定信号中心频率。`fits` 判断某个中心能否放下整个信号带（各模式自己的 `center_in_range`）。
///
/// 调用方给了值就只认它（放不下报 [`EncodeError::CenterOutOfRange`]，不悄悄挪）；没给就用
/// [`DEFAULT_CENTER_HZ`]，放不下时退到采样率的四分之一。
///
/// # Errors
///
/// 给定的中心放不下，或缺省中心与退路都放不下。
pub fn resolve_center(
  requested: Option<f64>,
  rate: u32,
  fits: impl Fn(f64) -> bool,
) -> Result<f64, EncodeError> {
  if let Some(c) = requested {
    return if fits(c) {
      Ok(c)
    } else {
      Err(EncodeError::CenterOutOfRange(c))
    };
  }
  [DEFAULT_CENTER_HZ, f64::from(rate) / 4.0]
    .into_iter()
    .find(|c| fits(*c))
    .ok_or(EncodeError::CenterOutOfRange(DEFAULT_CENTER_HZ))
}

/// `encode_audio` 的结果落实：`None`（合成前的估算已超限）或超过样本上限，都是 [`EncodeError::TooLong`]。
///
/// 其余参数（子模式、中心频率、尺寸）调用方应已先校验过，所以 `None` 只会是「太长」。
///
/// # Errors
///
/// 见上。
pub fn within_budget(samples: Option<Vec<f32>>) -> Result<Vec<f32>, EncodeError> {
  samples
    .filter(|s| s.len() <= MAX_ENCODE_SAMPLES)
    .ok_or(EncodeError::TooLong)
}

/// MFSK：校验后合成一段基带，返回 `(样本, 采样率)`。采样率取该模式的标称值（fldigi 里的值），
/// 页面写 WAV 头时用它；中心频率不给就用缺省。
///
/// # Errors
///
/// 子模式不在表内、中心放不下、报文不可发或过长。
pub fn encode_message(
  text: &str,
  mode_name: &str,
  center_hz: Option<f64>,
) -> Result<(Vec<f32>, u32), EncodeError> {
  let mode = crate::parse_mode(mode_name)
    .ok_or_else(|| EncodeError::UnsupportedSubmode(mode_name.to_owned()))?;
  let text = sendable_text(text)?;
  let rate = mode.rate;
  let center = resolve_center(center_hz, rate, |c| crate::center_in_range(mode, rate, c))?;
  let samples = within_budget(crate::encode_audio(&text, mode, rate, center))?;
  Ok((samples, rate))
}

/// 图片发端的入参（页面 / Worker 给的原始形态）。
#[derive(Debug, Clone, Copy)]
pub struct PictureTx<'a> {
  pub width: usize,
  pub height: usize,
  /// 彩色每行发 R、G、B 三遍；灰度只发一遍（取 R 通道，调用方应已做过灰度）。
  pub color: bool,
  /// 每个像素的样本数：8 标准、4 快、2 最快（越快越糊）。THOR 不用这个字段，它固定 10。
  pub spp: usize,
  /// `宽 × 高 × 3` 字节的 RGB。
  pub rgb: &'a [u8],
}

/// MFSK：校验后合成一次图片发射（引导文本 + `Pic:宽x高[C][pN];` 报头 + 模拟调频图像），返回 `(样本, 采样率)`。
///
/// 只有 MFSK 16 / 32 / 64 / 128 有图片模式；采样率取该模式的标称值（图片的序言与每像素样本数是按它给的，
/// 别的采样率发不了）。`text` 是报头前面的引导文本，可以为空。
///
/// # Errors
///
/// 子模式不在表内、没有图片模式、尺寸 / 像素数据 / `spp` 不合法、中心放不下、引导文本过长、合成后超限。
pub fn encode_picture_message(
  text: &str,
  mode_name: &str,
  center_hz: Option<f64>,
  pic: &PictureTx<'_>,
) -> Result<(Vec<f32>, u32), EncodeError> {
  let mode = crate::parse_mode(mode_name)
    .ok_or_else(|| EncodeError::UnsupportedSubmode(mode_name.to_owned()))?;
  let pixels = pic.width.checked_mul(pic.height);
  if !mode.has_picture()
    || !matches!(pic.spp, 2 | 4 | 8)
    || pixels.is_none_or(|n| n == 0 || n > crate::picture::MAX_PIXELS)
    || pixels.and_then(|n| n.checked_mul(3)) != Some(pic.rgb.len())
  {
    return Err(EncodeError::Picture);
  }
  let text = lead_in_text(text)?;
  let rate = mode.rate;
  let center = resolve_center(center_hz, rate, |c| crate::center_in_range(mode, rate, c))?;
  let header = crate::picture::Header {
    width: pic.width,
    height: pic.height,
    color: pic.color,
    spp: pic.spp,
  };
  let samples = within_budget(crate::encode_picture_audio(
    &text, &header, pic.rgb, mode, rate, center,
  ))?;
  Ok((samples, rate))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{MODES, decode_audio};

  #[test]
  fn sendable_text_keeps_printable_ascii_and_newline_only() {
    assert_eq!(sendable_text("CQ DE BG1AAA\n").unwrap(), "CQ DE BG1AAA\n");
    // 中文、全角标点、制表符、NUL、DEL、`\x01`（THOR 的二次信道开关）都跳过。
    assert_eq!(
      sendable_text("CQ你好\tDE\u{0}BG1\u{7f}A\u{1}AA，").unwrap(),
      "CQDEBG1AAA"
    );
  }

  #[test]
  fn unsendable_or_oversized_text_is_rejected() {
    for bad in ["", "   ", "\n\n", "你好，世界", "\u{1}\u{2}"] {
      assert_eq!(sendable_text(bad), Err(EncodeError::Text), "{bad:?}");
    }
    assert!(sendable_text(&"A".repeat(MAX_TEXT_CHARS)).is_ok());
    assert_eq!(
      sendable_text(&"A".repeat(MAX_TEXT_CHARS + 1)),
      Err(EncodeError::Text)
    );
    // 长度按原文算：过滤前就超限的，不能靠「过滤后变短」混过去。
    let padded = format!("A{}", "你".repeat(MAX_TEXT_CHARS));
    assert_eq!(sendable_text(&padded), Err(EncodeError::Text));
  }

  #[test]
  fn error_codes_reuse_the_decoder_side_codes() {
    assert_eq!(
      EncodeError::UnsupportedSubmode("99".into()).code(),
      "unsupported-submode:99"
    );
    assert_eq!(
      EncodeError::CenterOutOfRange(100000.0).code(),
      "center-hz-out-of-range:100000"
    );
    assert_eq!(
      EncodeError::CenterOutOfRange(f64::NAN).code(),
      "center-hz-out-of-range:NaN"
    );
    assert_eq!(EncodeError::Text.code(), "encode-params:text");
  }

  #[test]
  fn center_is_never_moved_silently_when_the_caller_gave_one() {
    let fits = |c: f64| (500.0..=3000.0).contains(&c);
    assert_eq!(resolve_center(Some(2500.0), 8000, fits), Ok(2500.0));
    assert_eq!(
      resolve_center(Some(100.0), 8000, fits),
      Err(EncodeError::CenterOutOfRange(100.0))
    );
    assert!(resolve_center(Some(f64::NAN), 8000, |c| c.is_finite()).is_err());
    // 缺省：1500 放得下就用它；放不下退到 rate / 4；都放不下报错。
    assert_eq!(resolve_center(None, 8000, fits), Ok(DEFAULT_CENTER_HZ));
    assert_eq!(resolve_center(None, 8000, |c| c > 1900.0), Ok(2000.0));
    assert!(resolve_center(None, 8000, |_| false).is_err());
  }

  /// 每个子模式在缺省中心与标称采样率下都放得下、能合成；采样率是模式表里的值。
  #[test]
  fn every_mode_encodes_with_the_default_center_at_its_nominal_rate() {
    for mode in MODES {
      let (samples, rate) = encode_message("CQ", mode.name, None)
        .unwrap_or_else(|e| panic!("MFSK {}: {e:?}", mode.name));
      assert_eq!(rate, mode.rate, "MFSK {}", mode.name);
      assert!(!samples.is_empty(), "MFSK {}", mode.name);
      assert!(
        samples.iter().all(|v| v.is_finite() && v.abs() <= 1.0),
        "MFSK {}",
        mode.name
      );
    }
  }

  /// 发端的输出交给本 crate 的解码器能读回原文（页面再在解码页里读一遍，这里先在内核里钉住）。
  #[test]
  fn synthesised_audio_decodes_back_to_the_text() {
    for name in ["16", "32", "8", "11", "64"] {
      let mode = crate::parse_mode(name).unwrap();
      let (samples, rate) = encode_message("CQ CQ DE BG1AAA", name, None).unwrap();
      let decoded = decode_audio(&samples, rate, mode, None, false)
        .unwrap_or_else(|| panic!("MFSK {name}: 没有解出信号"));
      assert!(
        decoded.text.contains("CQ CQ DE BG1AAA"),
        "MFSK {name}: {:?}",
        decoded.text
      );
    }
  }

  #[test]
  fn bad_parameters_are_reported_with_their_own_codes() {
    assert_eq!(
      encode_message("CQ", "99", None).unwrap_err(),
      EncodeError::UnsupportedSubmode("99".into())
    );
    assert_eq!(
      encode_message("CQ", "16", Some(100_000.0)).unwrap_err(),
      EncodeError::CenterOutOfRange(100_000.0)
    );
    assert_eq!(
      encode_message("CQ", "16", Some(f64::NAN))
        .unwrap_err()
        .code(),
      "center-hz-out-of-range:NaN"
    );
    assert_eq!(
      encode_message("你好", "16", None).unwrap_err(),
      EncodeError::Text
    );
    // 子模式先于报文校验：两个都错时报子模式，与解码侧先查模式的顺序一致。
    assert_eq!(
      encode_message("", "99", None).unwrap_err(),
      EncodeError::UnsupportedSubmode("99".into())
    );
  }

  /// 四象限的彩色测试图：左右、上下各用不同的颜色，解出来逐象限比对。
  fn quadrant_picture(w: usize, h: usize) -> Vec<u8> {
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

  fn pic(width: usize, height: usize, color: bool, spp: usize, rgb: &[u8]) -> PictureTx<'_> {
    PictureTx {
      width,
      height,
      color,
      spp,
      rgb,
    }
  }

  /// 发出去的图片交给本 crate 的解码器：尺寸、彩色标志与四个象限的颜色都读得回（FM 解调容差 ±4），
  /// 引导文本与 `Pic:` 报头留在正文里。
  #[test]
  fn a_synthesised_picture_decodes_back_with_its_colours() {
    let (w, h) = (32, 24);
    let rgb = quadrant_picture(w, h);
    for name in ["16", "32"] {
      let mode = crate::parse_mode(name).unwrap();
      let (samples, rate) =
        encode_picture_message("CQ ", name, None, &pic(w, h, true, 8, &rgb)).unwrap();
      assert_eq!(rate, mode.rate);
      let decoded = decode_audio(&samples, rate, mode, None, false)
        .unwrap_or_else(|| panic!("MFSK {name}: 没有解出信号"));
      assert_eq!(decoded.pictures.len(), 1, "MFSK {name}");
      let got = &decoded.pictures[0];
      assert_eq!(
        (got.width, got.height, got.color),
        (w, h, true),
        "MFSK {name}"
      );
      assert!(
        decoded.text.contains("CQ Pic:32x24C;"),
        "MFSK {name}: {:?}",
        decoded.text
      );
      for (x, y) in [(8, 6), (24, 6), (8, 18), (24, 18)] {
        for c in 0..3 {
          let want = rgb[3 * (y * w + x) + c];
          let have = got.rgb[3 * (y * w + x) + c];
          assert!(
            have.abs_diff(want) <= 4,
            "MFSK {name} ({x},{y}) 通道 {c}: {have} vs {want}"
          );
        }
      }
    }
  }

  /// 灰度只发一遍，所以比同尺寸的彩色少两个平面的像素；引导文本可以为空。
  ///
  /// 一帧 MFSK（序言、报头、冲刷）有约十万个样本的固定开销，小图的总长不成比例，
  /// 所以比的是**差值**：彩色与灰度之差 ≈ 两个平面（报头里多一个 `C` 字符，留几千样本的余量）。
  #[test]
  fn a_grey_picture_drops_two_planes_and_needs_no_lead_in_text() {
    let (w, h) = (64, 48);
    let rgb = quadrant_picture(w, h);
    let (colour, _) = encode_picture_message("", "16", None, &pic(w, h, true, 8, &rgb)).unwrap();
    // 灰度取 R 通道：把每个像素的三个通道都设成同一个值（页面里就是这么做的）。
    let grey: Vec<u8> = rgb.chunks(3).flat_map(|p| [p[0], p[0], p[0]]).collect();
    let (gray, rate) = encode_picture_message("", "16", None, &pic(w, h, false, 8, &grey)).unwrap();
    let two_planes = (2 * w * h * 8) as i64;
    let saved = colour.len() as i64 - gray.len() as i64;
    assert!(
      (saved - two_planes).abs() < 4000,
      "少了 {saved} 个样本，两个平面是 {two_planes}"
    );
    let decoded = decode_audio(&gray, rate, crate::parse_mode("16").unwrap(), None, false).unwrap();
    assert_eq!(decoded.pictures.len(), 1);
    assert!(!decoded.pictures[0].color);
  }

  /// 快速档（p4 / p2）样本更少；`spp` 只认 2 / 4 / 8。
  ///
  /// 图要够大：快速档的报头多两个字符（`p4`），小图上这点开销会盖过省下的像素样本。
  #[test]
  fn faster_pixel_rates_make_shorter_audio() {
    let (w, h) = (64, 48);
    let rgb = quadrant_picture(w, h);
    let len = |spp| {
      encode_picture_message("", "16", None, &pic(w, h, true, spp, &rgb))
        .unwrap()
        .0
        .len()
    };
    assert!(
      len(2) < len(4) && len(4) < len(8),
      "{} {} {}",
      len(2),
      len(4),
      len(8)
    );
    for bad in [0, 1, 3, 5, 16] {
      assert_eq!(
        encode_picture_message("", "16", None, &pic(w, h, true, bad, &rgb)).unwrap_err(),
        EncodeError::Picture,
        "spp={bad}"
      );
    }
  }

  #[test]
  fn picture_parameters_are_checked_before_anything_is_synthesised() {
    let rgb = quadrant_picture(16, 12);
    let ok = pic(16, 12, true, 8, &rgb);
    // 只有 16 / 32 / 64 / 128 有图片模式。
    for name in ["4", "8", "11", "22", "31", "64L", "128L"] {
      assert_eq!(
        encode_picture_message("", name, None, &ok).unwrap_err(),
        EncodeError::Picture,
        "MFSK {name}"
      );
    }
    for name in ["64", "128"] {
      assert!(
        encode_picture_message("", name, None, &ok).is_ok(),
        "MFSK {name}"
      );
    }
    // 子模式先于图片校验：两个都错时报子模式。
    assert_eq!(
      encode_picture_message("", "99", None, &pic(0, 0, true, 8, &[])).unwrap_err(),
      EncodeError::UnsupportedSubmode("99".into())
    );
    // 像素数据与尺寸对不上、尺寸为零、溢出。
    assert_eq!(
      encode_picture_message("", "16", None, &pic(16, 12, true, 8, &rgb[1..])).unwrap_err(),
      EncodeError::Picture
    );
    assert_eq!(
      encode_picture_message("", "16", None, &pic(0, 12, true, 8, &[])).unwrap_err(),
      EncodeError::Picture
    );
    assert_eq!(
      encode_picture_message("", "16", None, &pic(usize::MAX, 2, true, 8, &rgb)).unwrap_err(),
      EncodeError::Picture
    );
    // 引导文本过长是 `Text`，中心放不下是 `CenterOutOfRange`。
    assert_eq!(
      encode_picture_message(&"A".repeat(MAX_TEXT_CHARS + 1), "16", None, &ok).unwrap_err(),
      EncodeError::Text
    );
    assert_eq!(
      encode_picture_message("", "16", Some(100_000.0), &ok).unwrap_err(),
      EncodeError::CenterOutOfRange(100_000.0)
    );
  }

  /// 合成后超过样本上限是「太长」，不是图片参数错：1024 × 1024 的彩色图在 MFSK16 下要一个多小时。
  #[test]
  fn a_picture_that_would_take_too_long_is_reported_as_such() {
    let (w, h) = (1024, 1024);
    let rgb = vec![128u8; w * h * 3];
    assert_eq!(
      encode_picture_message("", "16", None, &pic(w, h, true, 8, &rgb)).unwrap_err(),
      EncodeError::TooLong
    );
    // 同一张图用最快档、灰度：样本数落在上限内，应当能发（证明上面的拒绝来自长度，不是尺寸）。
    let grey = vec![128u8; w * h * 3];
    let (samples, _) = encode_picture_message("", "16", None, &pic(w, h, false, 2, &grey)).unwrap();
    assert!(samples.len() <= MAX_ENCODE_SAMPLES);
  }

  #[test]
  fn lead_in_text_may_be_empty_but_obeys_the_same_filter_and_limit() {
    assert_eq!(lead_in_text("").unwrap(), "");
    assert_eq!(lead_in_text("CQ你好 ").unwrap(), "CQ ");
    assert_eq!(
      lead_in_text(&"A".repeat(MAX_TEXT_CHARS + 1)),
      Err(EncodeError::Text)
    );
    // 与 `sendable_text` 的差别只在「允许空」。
    assert_eq!(sendable_text(""), Err(EncodeError::Text));
  }

  #[test]
  fn too_long_picture_and_id_have_their_own_protocol_codes() {
    assert_eq!(EncodeError::TooLong.code(), "encode-params:length");
    assert_eq!(EncodeError::Picture.code(), "encode-params:picture");
    assert_eq!(EncodeError::Id.code(), "encode-params:id");
  }
}

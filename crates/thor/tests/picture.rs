//! THOR 图片模式：本 crate 的发端 → 收端往返，加上分段（图像前后的文本）、同步音、采样率与报头的边界。
//!
//! fldigi 自己的发端 / 真实桌面录音的对拍在 `real_recordings.rs`（需要 `THOR_REAL_DIR`）。

use ham_web_mfsk::picture::Header;
use ham_web_thor::picture::{SPP, SYNC_OFFSET};
use ham_web_thor::{decode_audio, encode_audio, encode_picture_audio, parse_mode};

/// 四块纯色（与 fldigi-web 自己的图片测试同款）：左右 R 220 / 40，上下 G 180 / 70，左右 B 30 / 210。
fn bands(w: usize, h: usize) -> Vec<u8> {
  let mut rgb = Vec::with_capacity(w * h * 3);
  for y in 0..h {
    for x in 0..w {
      rgb.extend([
        if x < w / 2 { 220 } else { 40 },
        if y < h / 2 { 180 } else { 70 },
        if x < w / 2 { 30 } else { 210 },
      ]);
    }
  }
  rgb
}

fn mean_error(got: &[u8], want: &[u8]) -> f64 {
  got
    .iter()
    .zip(want)
    .map(|(&a, &b)| f64::from(i32::from(a).abs_diff(i32::from(b))))
    .sum::<f64>()
    / want.len() as f64
}

fn header(w: usize, h: usize, color: bool) -> Header {
  Header {
    width: w,
    height: h,
    color,
    spp: SPP,
  }
}

#[test]
fn every_picture_mode_round_trips_in_color_and_gray() {
  for name in ["11", "16", "22", "25", "32", "44", "56"] {
    let mode = parse_mode(name).unwrap();
    for color in [true, false] {
      let (w, h) = (59usize, 74usize);
      let mut rgb = bands(w, h);
      if !color {
        for px in rgb.as_chunks_mut::<3>().0.iter_mut() {
          let g =
            ((31 * u32::from(px[0]) + 61 * u32::from(px[1]) + 8 * u32::from(px[2])) / 100) as u8;
          px.fill(g);
        }
      }
      let audio =
        encode_picture_audio("CQ ", &header(w, h, color), &rgb, mode, mode.rate, 1500.0).unwrap();
      let d = decode_audio(&audio, mode.rate, mode, Some(1500.0), false)
        .unwrap_or_else(|| panic!("THOR{name}：应解出信号"));
      assert_eq!(d.pictures.len(), 1, "THOR{name} 彩色 {color}：{:?}", d.text);
      let p = &d.pictures[0];
      assert_eq!((p.width, p.height, p.color), (w, h, color));
      assert!(
        mean_error(&p.rgb, &rgb) < 6.0,
        "THOR{name} 彩色 {color}：平均误差 {}",
        mean_error(&p.rgb, &rgb)
      );
      // 正文里报头照常保留，同步音与图像段的乱码已经去掉。
      assert!(
        d.text.trim_start().starts_with("CQ pic%") || d.text.contains("CQ pic%"),
        "THOR{name}：{:?}",
        d.text
      );
      assert!(
        d.text.contains(if color { "pic%T" } else { "pic%t" }),
        "THOR{name}：{:?}",
        d.text
      );
    }
  }
}

#[test]
fn the_other_sizes_are_read_from_the_header_code() {
  let mode = parse_mode("32").unwrap();
  for (w, h) in [(120usize, 150usize), (160, 120)] {
    let rgb = bands(w, h);
    let audio = encode_picture_audio("", &header(w, h, true), &rgb, mode, 8000, 1500.0).unwrap();
    let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
    assert_eq!(d.pictures.len(), 1, "{w}×{h}：{:?}", d.text);
    assert_eq!((d.pictures[0].width, d.pictures[0].height), (w, h));
    assert!(mean_error(&d.pictures[0].rgb, &rgb) < 8.0);
  }
}

/// 同步音的位置本身就是图像起点，所以不给中心频率也行：自动检测在整段录音上可能被图像带偏，
/// 退一步用开头一段，哪个中心频率找得到同步音就用哪个。
#[test]
fn the_center_frequency_is_found_without_being_told() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (59usize, 74usize);
  let rgb = bands(w, h);
  for center in [1337.5, 1500.0, 1800.0] {
    let audio = encode_picture_audio("", &header(w, h, true), &rgb, mode, 8000, center).unwrap();
    let d = decode_audio(&audio, 8000, mode, None, false).unwrap_or_else(|| panic!("{center}"));
    assert_eq!(d.pictures.len(), 1, "{center}：{:?}", d.text);
    assert!(mean_error(&d.pictures[0].rgb, &rgb) < 6.0, "{center}");
  }
}

#[test]
fn text_before_and_after_the_picture_are_both_read() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (59usize, 74usize);
  let rgb = bands(w, h);
  let mut audio =
    encode_picture_audio("DE BG1AAA ", &header(w, h, true), &rgb, mode, 8000, 1500.0).unwrap();
  audio.extend(std::iter::repeat_n(0.0f32, 777));
  audio.extend(encode_audio("73 SK", mode, 8000, 1500.0).unwrap());
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
  assert_eq!(d.pictures.len(), 1);
  assert!(d.text.contains("DE BG1AAA pic%T"), "{:?}", d.text);
  assert!(d.text.contains("73 SK"), "图像之后的文本：{:?}", d.text);
}

#[test]
fn two_pictures_in_one_recording() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (59usize, 74usize);
  let first = bands(w, h);
  let mut second = bands(w, h);
  for px in second.as_chunks_mut::<3>().0.iter_mut() {
    px.swap(0, 2);
  }
  let mut audio =
    encode_picture_audio("", &header(w, h, true), &first, mode, 8000, 1500.0).unwrap();
  audio.extend(encode_picture_audio("", &header(w, h, true), &second, mode, 8000, 1500.0).unwrap());
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
  assert_eq!(d.pictures.len(), 2, "{:?}", d.text);
  assert!(mean_error(&d.pictures[0].rgb, &first) < 6.0);
  assert!(mean_error(&d.pictures[1].rgb, &second) < 6.0);
}

#[test]
fn the_picture_survives_noise() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (59usize, 74usize);
  let rgb = bands(w, h);
  let mut audio = encode_picture_audio("", &header(w, h, true), &rgb, mode, 8000, 1500.0).unwrap();
  ham_web_dsp::add_noise(&mut audio, 0.4);
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
  assert_eq!(d.pictures.len(), 1, "{:?}", d.text);
  assert!(mean_error(&d.pictures[0].rgb, &rgb) < 25.0);
}

/// 只在模式的标称采样率上解图：每像素 10 个样本是按它给的，别的采样率没有重采样。
#[test]
fn other_sample_rates_give_text_only() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (59usize, 74usize);
  let rgb = bands(w, h);
  assert!(encode_picture_audio("", &header(w, h, true), &rgb, mode, 12000, 1500.0).is_none());
  let audio = encode_picture_audio("", &header(w, h, true), &rgb, mode, 8000, 1500.0).unwrap();
  let d = decode_audio(&audio, 11025, mode, Some(1500.0), false);
  assert!(d.is_none_or(|d| d.pictures.is_empty()));
}

#[test]
fn modes_without_a_picture_mode_never_report_one() {
  for name in ["micro", "4", "5", "8", "25x4", "50x1", "50x2", "100"] {
    assert!(
      parse_mode(name).is_some_and(|m| !m.has_picture()),
      "THOR{name}"
    );
  }
  let mode = parse_mode("8").unwrap();
  let (w, h) = (59usize, 74usize);
  assert!(
    encode_picture_audio("", &header(w, h, true), &bands(w, h), mode, 8000, 1500.0).is_none()
  );
  let audio = encode_audio("pic%T", mode, 8000, 1500.0).unwrap();
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
  assert!(d.pictures.is_empty());
  assert!(d.text.contains("pic%T"));
}

#[test]
fn sizes_the_header_cannot_name_are_not_sent() {
  let mode = parse_mode("16").unwrap();
  assert!(
    encode_picture_audio(
      "",
      &header(40, 30, true),
      &bands(40, 30),
      mode,
      8000,
      1500.0
    )
    .is_none()
  );
  let mut wrong_speed = header(59, 74, true);
  wrong_speed.spp = 8;
  assert!(encode_picture_audio("", &wrong_speed, &bands(59, 74), mode, 8000, 1500.0).is_none());
  assert!(encode_picture_audio("", &header(59, 74, true), &[0; 10], mode, 8000, 1500.0).is_none());
}

#[test]
fn centers_that_would_put_the_sync_tone_below_zero_are_not_sent() {
  // 同步音在 center − 0.6 × 带宽，比最低音调还低 0.1 × 带宽：音调带还放得下时它可能已经掉到 0 Hz 以下，
  // 那样的音频接收端找不回来（负频率的音调会镜像回正半轴）。
  let mode = parse_mode("16").unwrap();
  let bw = mode.bandwidth();
  // 最低音调刚好在 0 之上（音调带的校验会放行），同步音却是负的。
  let center = bw * 0.55;
  assert!(center + mode.tone_offset(0) > 0.0);
  assert!(center + SYNC_OFFSET * bw < 0.0);
  assert!(
    encode_picture_audio(
      "",
      &header(59, 74, true),
      &bands(59, 74),
      mode,
      8000,
      center
    )
    .is_none()
  );
  // 再高一点就两头都放得下。
  assert!(
    encode_picture_audio(
      "",
      &header(59, 74, true),
      &bands(59, 74),
      mode,
      8000,
      bw * 0.7
    )
    .is_some()
  );
}

#[test]
fn a_header_without_the_sync_tone_is_just_text() {
  // 报头后面没有同步音与图像（录音被截断、或有人手敲了 pic%T）：当普通文本，不 panic。
  let mode = parse_mode("16").unwrap();
  let audio = encode_audio("see pic%T here", mode, 8000, 1500.0).unwrap();
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
  assert!(d.pictures.is_empty());
  assert!(d.picture_skipped, "有报头没解出图，要让页面知道");
  assert!(d.text.contains("pic%T"), "{:?}", d.text);
  // 没有报头的普通文本不带这个标记。
  let plain = encode_audio("just text", mode, 8000, 1500.0).unwrap();
  assert!(
    !decode_audio(&plain, 8000, mode, Some(1500.0), false)
      .unwrap()
      .picture_skipped
  );
}

#[test]
fn the_picture_is_cut_short_when_the_recording_ends_inside_it() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (59usize, 74usize);
  let audio =
    encode_picture_audio("", &header(w, h, true), &bands(w, h), mode, 8000, 1500.0).unwrap();
  // 截掉图像的后半：前面几行有内容，最后几行是 0。
  let cut = audio.len() - 70_000 - 40_000;
  let d = decode_audio(&audio[..cut], 8000, mode, Some(1500.0), false).unwrap();
  assert_eq!(d.pictures.len(), 1);
  let rgb = &d.pictures[0].rgb;
  assert!(rgb[..3 * w * 4].iter().any(|&v| v > 100));
  assert!(rgb[3 * w * (h - 3)..].iter().all(|&v| v == 0));
}

/// 斜坡 + 棋盘格图案对**起点**很敏感：纯色块图案里起点偏几个像素也看不出来。
/// 误差上限是量过的（本 crate 自己的发端，11 / 16 / 25 / 32 / 44 / 56 分别是 14.0 / 8.0 / 5.0 / 4.1 / 4.1 / 4.7，
/// 误差来自 FM 带宽有限；THOR11 的符号最长、带宽最窄，误差最大）。
/// 变异验证：同步音结束处的像素起点挪晚 8 个样本，这个用例变红。
#[test]
fn a_ramp_with_a_checkerboard_pins_down_the_picture_start() {
  for (name, limit) in [
    ("11", 19.0),
    ("16", 11.0),
    ("25", 8.0),
    ("32", 7.0),
    ("44", 7.0),
    ("56", 8.0),
  ] {
    let mode = parse_mode(name).unwrap();
    let (w, h) = (59usize, 74usize);
    let mut rgb = Vec::new();
    for y in 0..h {
      for x in 0..w {
        rgb.extend([
          (x * 255 / (w - 1)) as u8,
          (y * 255 / (h - 1)) as u8,
          if (x / 4 + y / 4) % 2 == 1 { 230 } else { 25 },
        ]);
      }
    }
    let audio =
      encode_picture_audio("", &header(w, h, true), &rgb, mode, mode.rate, 1500.0).unwrap();
    let d = decode_audio(&audio, mode.rate, mode, Some(1500.0), false).unwrap();
    assert_eq!(d.pictures.len(), 1, "THOR{name}");
    let error = mean_error(&d.pictures[0].rgb, &rgb);
    assert!(error < limit, "THOR{name}：平均误差 {error}，上限 {limit}");
  }
}

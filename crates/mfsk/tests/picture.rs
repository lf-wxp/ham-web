//! MFSK 图片模式：本 crate 的发端 → 收端往返，加上分段（图像前后的文本）、采样率、噪声与报头的边界。
//!
//! fldigi 自己的发端 / 真实桌面录音的对拍在 `real_recordings.rs`（需要 `MFSK_REAL_DIR`）。

use ham_web_mfsk::picture::Header;
use ham_web_mfsk::{decode_audio, encode_audio, encode_picture_audio, parse_mode};

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

/// 图像四个象限中心的像素是不是预期的颜色（±容差）。
fn quadrants_match(rgb: &[u8], w: usize, h: usize, tolerance: i32) -> bool {
  let want = bands(w, h);
  [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)]
    .iter()
    .all(|&(xr, yr)| {
      let at = 3 * (((yr * h as f64) as usize) * w + (xr * w as f64) as usize);
      (0..3).all(|c| (i32::from(rgb[at + c]) - i32::from(want[at + c])).abs() <= tolerance)
    })
}

fn header(w: usize, h: usize, color: bool, spp: usize) -> Header {
  Header {
    width: w,
    height: h,
    color,
    spp,
  }
}

#[test]
fn every_picture_mode_round_trips_in_color_and_gray() {
  for name in ["16", "32", "64", "128"] {
    let mode = parse_mode(name).unwrap();
    for color in [true, false] {
      let (w, h) = (32usize, 24usize);
      let mut rgb = bands(w, h);
      if !color {
        // 灰度：三个通道相同（fldigi 的灰度公式 31 / 61 / 8）。
        for px in rgb.as_chunks_mut::<3>().0.iter_mut() {
          let g =
            ((31 * u32::from(px[0]) + 61 * u32::from(px[1]) + 8 * u32::from(px[2])) / 100) as u8;
          px.fill(g);
        }
      }
      let audio =
        encode_picture_audio("CQ ", &header(w, h, color, 8), &rgb, mode, 8000, 1500.0).unwrap();
      let d = decode_audio(&audio, 8000, mode, Some(1500.0), false)
        .unwrap_or_else(|| panic!("MFSK{name}：应解出信号"));
      assert_eq!(d.pictures.len(), 1, "MFSK{name} 彩色 {color}：{:?}", d.text);
      let p = &d.pictures[0];
      assert_eq!((p.width, p.height, p.color), (w, h, color));
      assert_eq!(p.rgb.len(), w * h * 3);
      assert!(
        mean_error(&p.rgb, &rgb) < 12.0,
        "MFSK{name} 彩色 {color}：平均误差 {}",
        mean_error(&p.rgb, &rgb)
      );
      if color {
        assert!(
          quadrants_match(&p.rgb, w, h, 3),
          "MFSK{name}：四个象限的颜色不对"
        );
      }
      // 正文里报头照常保留，图像段的乱码已经去掉。
      let suffix = format!("Pic:{w}x{h}{};", if color { "C" } else { "" });
      assert_eq!(
        d.text.trim(),
        format!("CQ {suffix}"),
        "MFSK{name}：{:?}",
        d.text
      );
    }
  }
}

#[test]
fn faster_speeds_and_the_header_variants_are_read() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (40usize, 30usize);
  let rgb = bands(w, h);
  for spp in [8usize, 4] {
    let audio =
      encode_picture_audio("", &header(w, h, true, spp), &rgb, mode, 8000, 1500.0).unwrap();
    let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
    assert_eq!(d.pictures.len(), 1, "spp {spp}：{:?}", d.text);
    // spp 8 与 4 的像素宽度不一样；4 个样本一个像素时带宽不够，边沿很糊，只要求四个象限的颜色大致对。
    assert!(quadrants_match(&d.pictures[0].rgb, w, h, 12), "spp {spp}");
    let expected = if spp == 8 {
      "Pic:40x30C;"
    } else {
      "Pic:40x30Cp4;"
    };
    assert_eq!(d.text.trim(), expected);
  }
}

/// 图像前后是两段独立的 MFSK：它们的符号对齐不一样（图像段的长度不是符号长度的整数倍），
/// 各自重新对齐、各自解码，图像后面的文本也要读出来。
#[test]
fn text_before_and_after_the_picture_are_both_read() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (32usize, 24usize);
  let rgb = bands(w, h);
  let mut audio = encode_picture_audio(
    "DE BG1AAA ",
    &header(w, h, true, 8),
    &rgb,
    mode,
    8000,
    1500.0,
  )
  .unwrap();
  // 图像之后再接一段（从一个不是符号整数倍的位置开始）普通文本。
  audio.extend(std::iter::repeat_n(0.0f32, 777));
  audio.extend(encode_audio("73 SK", mode, 8000, 1500.0).unwrap());
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
  assert_eq!(d.pictures.len(), 1);
  assert!(quadrants_match(&d.pictures[0].rgb, w, h, 3));
  assert!(d.text.contains("DE BG1AAA Pic:32x24C;"), "{:?}", d.text);
  assert!(d.text.contains("73 SK"), "图像之后的文本：{:?}", d.text);
}

#[test]
fn two_pictures_in_one_recording() {
  let mode = parse_mode("32").unwrap();
  let (w, h) = (24usize, 16usize);
  let first = bands(w, h);
  let mut second = bands(w, h);
  for px in second.as_chunks_mut::<3>().0.iter_mut() {
    px.swap(0, 2);
  }
  let mut audio =
    encode_picture_audio("", &header(w, h, true, 8), &first, mode, 8000, 1500.0).unwrap();
  audio
    .extend(encode_picture_audio("", &header(w, h, true, 8), &second, mode, 8000, 1500.0).unwrap());
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
  assert_eq!(d.pictures.len(), 2, "{:?}", d.text);
  assert!(mean_error(&d.pictures[0].rgb, &first) < 12.0);
  assert!(mean_error(&d.pictures[1].rgb, &second) < 12.0);
}

#[test]
fn the_picture_survives_noise() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (32usize, 24usize);
  let rgb = bands(w, h);
  let mut audio =
    encode_picture_audio("", &header(w, h, true, 8), &rgb, mode, 8000, 1500.0).unwrap();
  ham_web_dsp::add_noise(&mut audio, 0.4);
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
  assert_eq!(d.pictures.len(), 1, "{:?}", d.text);
  assert!(quadrants_match(&d.pictures[0].rgb, w, h, 20));
}

/// 只在模式的标称采样率上解图：序言（352 个样本）与每像素样本数都是按它给的，别的采样率没有重采样。
#[test]
fn other_sample_rates_give_text_only() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (32usize, 24usize);
  let audio =
    encode_picture_audio("", &header(w, h, true, 8), &bands(w, h), mode, 8000, 1500.0).unwrap();
  assert!(
    encode_picture_audio(
      "",
      &header(w, h, true, 8),
      &bands(w, h),
      mode,
      12000,
      1500.0
    )
    .is_none()
  );
  // 8 kHz 的采样当成 8 kHz 解是有图的；只有 `rate != 标称` 才没有（这里用同一个采样当 11.025 kHz 喂）。
  let d = decode_audio(&audio, 11025, mode, Some(1500.0), false);
  assert!(d.is_none_or(|d| d.pictures.is_empty()));
}

#[test]
fn modes_without_a_picture_mode_never_report_one() {
  assert!(parse_mode("11").is_some_and(|m| !m.has_picture()));
  assert!(parse_mode("64L").is_some_and(|m| !m.has_picture()));
  let mode = parse_mode("11").unwrap();
  let (w, h) = (16usize, 12usize);
  assert!(
    encode_picture_audio(
      "",
      &header(w, h, true, 8),
      &bands(w, h),
      mode,
      11025,
      1500.0
    )
    .is_none()
  );
  // 文本里碰巧有一个像报头的东西，没有图片模式的子模式不当回事。
  let audio = encode_audio("Pic:16x12C;", mode, 11025, 1500.0).unwrap();
  let d = decode_audio(&audio, 11025, mode, Some(1500.0), false).unwrap();
  assert!(d.pictures.is_empty());
  assert!(d.text.contains("Pic:16x12C;"));
}

#[test]
fn a_header_without_its_picture_keeps_the_text_and_says_so() {
  // 只有文本报头、后面没有图像：没有图可出（不是一幅按报头尺寸分配的黑图），正文原样保留，并标明图片没解出。
  let mode = parse_mode("16").unwrap();
  // 640 × 480 的图像段要 7 百多万个样本，这段录音里远远没有：不输出一幅几乎全黑的大图。
  let audio = encode_audio("CQ Pic:640x480C;", mode, 8000, 1500.0).unwrap();
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
  assert!(d.pictures.is_empty());
  assert!(d.picture_skipped);
  assert!(d.text.contains("CQ Pic:640x480C;"), "{:?}", d.text);
}

#[test]
fn a_header_at_a_non_nominal_rate_is_reported_as_skipped() {
  // 图片的样本数是按标称采样率给的：别的采样率下有报头也不解图，但要让页面知道。
  let mode = parse_mode("16").unwrap();
  let audio = encode_audio("CQ Pic:32x24C;", mode, 11025, 1500.0).unwrap();
  let d = decode_audio(&audio, 11025, mode, Some(1500.0), false).unwrap();
  assert!(d.pictures.is_empty());
  assert!(d.picture_skipped);
  assert!(d.text.contains("Pic:32x24C;"), "{:?}", d.text);
}

#[test]
fn a_decoded_picture_and_plain_text_are_not_reported_as_skipped() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (32usize, 24usize);
  let audio =
    encode_picture_audio("", &header(w, h, true, 8), &bands(w, h), mode, 8000, 1500.0).unwrap();
  let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
  assert_eq!(d.pictures.len(), 1);
  assert!(!d.picture_skipped);
  let text = encode_audio("CQ CQ DE BG1AAA", mode, 8000, 1500.0).unwrap();
  assert!(
    !decode_audio(&text, 8000, mode, Some(1500.0), false)
      .unwrap()
      .picture_skipped
  );
}

#[test]
fn headers_the_decoder_would_not_read_back_are_not_sent() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (8usize, 8usize);
  let send = |spp: usize| {
    encode_picture_audio(
      "",
      &header(w, h, true, spp),
      &bands(w, h),
      mode,
      8000,
      1500.0,
    )
  };
  // 报头只能写 p2 / p4 / p8；p3 解码端会当成 8，往返悄悄对不上。
  assert!(send(8).is_some() && send(4).is_some() && send(2).is_some());
  assert!(send(3).is_none() && send(0).is_none() && send(16).is_none());
  // 总像素超过上限：不按它分配（`rgb` 的长度也对不上，两道都拦）。
  assert!(
    encode_picture_audio("", &header(4095, 4095, true, 8), &[], mode, 8000, 1500.0).is_none()
  );
  assert!(
    encode_picture_audio("", &header(usize::MAX, 2, true, 8), &[], mode, 8000, 1500.0).is_none()
  );
}

#[test]
fn the_picture_is_cut_short_when_the_recording_ends_inside_it() {
  let mode = parse_mode("16").unwrap();
  let (w, h) = (32usize, 24usize);
  let audio =
    encode_picture_audio("", &header(w, h, true, 8), &bands(w, h), mode, 8000, 1500.0).unwrap();
  // 截掉一半的图像段：前面几行有内容，后面是 0。
  let cut = audio.len() - 27_000 - 9_000;
  let d = decode_audio(&audio[..cut], 8000, mode, Some(1500.0), false).unwrap();
  assert_eq!(d.pictures.len(), 1);
  let rgb = &d.pictures[0].rgb;
  assert!(rgb[..3 * w * 4].iter().any(|&v| v > 100));
  assert!(rgb[3 * w * (h - 3)..].iter().all(|&v| v == 0));
}

/// 斜坡 + 棋盘格图案对**起点**很敏感：纯色块图案里起点偏几十个样本（几个像素）也看不出来。
/// 误差上限是量过的（本 crate 自己的发端：16 / 32 / 64 / 128 分别是 18.2 / 9.3 / 4.2 / 2.1，
/// 误差来自 FM 带宽有限，像素之间的大跳变被抹圆；符号越短带宽越宽，误差越小）。
/// 变异验证：序言样本数改成 330，这个用例变红。
#[test]
fn a_ramp_with_a_checkerboard_pins_down_the_picture_start() {
  for (name, limit) in [("16", 22.0), ("32", 13.0), ("64", 7.0), ("128", 4.0)] {
    let mode = parse_mode(name).unwrap();
    let (w, h) = (32usize, 24usize);
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
    let audio = encode_picture_audio("", &header(w, h, true, 8), &rgb, mode, 8000, 1500.0).unwrap();
    let d = decode_audio(&audio, 8000, mode, Some(1500.0), false).unwrap();
    assert_eq!(d.pictures.len(), 1, "MFSK{name}");
    let error = mean_error(&d.pictures[0].rgb, &rgb);
    assert!(error < limit, "MFSK{name}：平均误差 {error}，上限 {limit}");
  }
}

/// 长图 + 自动检测中心频率：整段的音调梳会被那一大段模拟调频带偏（实测 1000 → 1015.6 Hz，半个音调），
/// 图像必须用**图像之前那一小段**重新估出来的中心去解，否则整幅图的误差从 7.9 涨到 57.6。
///
/// 复现要 237 秒录音（320 × 240 彩色在 MFSK32 上是 1.84 M 个样本）：release 约 0.8 秒、debug 约 5 秒，
/// 是这一组用例里最慢的一条，换来的是「长图 + 自动检测」这条路上唯一的守卫。
/// 变异验证：把 `with_picture` 里那一次重估改回 `Some(seg.nominal_center_hz)`，中心读到 1015.6、用例变红。
#[test]
fn long_picture_uses_the_center_found_before_the_image() {
  let mode = parse_mode("32").unwrap();
  let (w, h) = (320usize, 240usize);
  // 与 `generate_picture_sample` 的 `ramp` 同款：R 随 x、G 随 y 线性变化，B 是 8 像素的棋盘格。
  let (dx, dy) = ((w - 1) as f64, (h - 1) as f64);
  let mut rgb = Vec::with_capacity(w * h * 3);
  for y in 0..h {
    for x in 0..w {
      let b = if (x / 4 + y / 4) % 2 == 1 { 230 } else { 25 };
      rgb.extend([
        (x as f64 * 255.0 / dx).round() as u8,
        (y as f64 * 255.0 / dy).round() as u8,
        b,
      ]);
    }
  }
  let audio =
    encode_picture_audio("CQ ", &header(w, h, true, 8), &rgb, mode, 8000, 1000.0).unwrap();
  let d = decode_audio(&audio, 8000, mode, None, false).unwrap();
  assert_eq!(d.pictures.len(), 1, "{:?}", d.text);
  // 中心频率要落在发端的 1000 Hz 上（整段检测会偏到 1015.6）。
  assert!((d.center_hz - 1000.0).abs() < 3.0, "中心 {}", d.center_hz);
  let error = mean_error(&d.pictures[0].rgb, &rgb);
  assert!(error < 12.0, "平均误差 {error}");
}

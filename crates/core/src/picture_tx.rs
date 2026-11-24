//! 图片发端的前处理：用户选一张图 → 要发的尺寸与像素。
//!
//! 浏览器负责解码图片与把它画到目标尺寸的 canvas 上，这里只放**不依赖浏览器**的部分：
//! 缩放后的尺寸怎么定、裁哪一块、RGBA 怎么变成发端要的 RGB / 灰度。
//! 发射本身（MFSK / THOR 的图片模式）在各自的 GPL worker 里，见 `docs/domain/decode-worker-protocol.md`「编码」。

/// 把 `src_w × src_h` 的图等比缩进 `max_edge × max_edge` 的方框里（不裁剪），返回目标尺寸。
///
/// 长边等于 `max_edge`，短边按比例四舍五入且至少 1 像素；输入为零返回 `None`。
/// 比原图大也照缩放（小图放大），方框就是用户选的「最大边长」。
#[must_use]
pub fn fit_within(src_w: usize, src_h: usize, max_edge: usize) -> Option<(usize, usize)> {
  if src_w == 0 || src_h == 0 || max_edge == 0 {
    return None;
  }
  let (w, h) = if src_w >= src_h {
    (max_edge, scaled(src_h, max_edge, src_w))
  } else {
    (scaled(src_w, max_edge, src_h), max_edge)
  };
  Some((w, h))
}

/// `value × num / den`，四舍五入、至少 1。
fn scaled(value: usize, num: usize, den: usize) -> usize {
  (value * num + den / 2).checked_div(den).unwrap_or(1).max(1)
}

/// 把 `src_w × src_h` 的图铺满 `dst_w × dst_h`（保持比例），多出来的部分从中间裁掉：
/// 返回要从原图里取的矩形 `(x, y, 宽, 高)`，单位是原图像素，宽高比与目标一致。
///
/// THOR 的图片尺寸只能是固定几种，用户的图比例不一定对得上；铺满裁剪比拉伸变形、也比留黑边都更接近
/// 「发一张合适的图」。输入为零返回 `None`。
#[must_use]
pub fn cover_rect(
  src_w: usize,
  src_h: usize,
  dst_w: usize,
  dst_h: usize,
) -> Option<(usize, usize, usize, usize)> {
  if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
    return None;
  }
  // 比较 src_w / src_h 与 dst_w / dst_h，用交叉相乘避免浮点。
  if src_w * dst_h > src_h * dst_w {
    // 原图更宽：裁左右。
    let w = (src_h * dst_w / dst_h).clamp(1, src_w);
    Some(((src_w - w) / 2, 0, w, src_h))
  } else {
    let h = (src_w * dst_h / dst_w).clamp(1, src_h);
    Some((0, (src_h - h) / 2, src_w, h))
  }
}

/// RGBA（canvas 的 `getImageData`）→ 发端要的 `宽 × 高 × 3` 字节 RGB。
///
/// - 半透明像素铺在白底上（PNG 的透明区域发出去应当是白的，不是黑的）；
/// - `gray` 为真时每个像素取 BT.601 亮度，三个通道都设成它（发端只发一遍，取 R 通道）。
///
/// 输入长度不是 4 的倍数时，多出来的尾部字节被忽略。
#[must_use]
pub fn rgba_to_rgb(rgba: &[u8], gray: bool) -> Vec<u8> {
  let over_white = |c: u8, a: u8| -> u8 {
    // (c·a + 255·(255−a)) / 255，四舍五入；a = 255 时恒等于 c。
    let v = (u32::from(c) * u32::from(a) + 255 * (255 - u32::from(a)) + 127) / 255;
    v as u8
  };
  let mut out = Vec::with_capacity(rgba.len() / 4 * 3);
  for px in rgba.as_chunks::<4>().0 {
    let (r, g, b) = (
      over_white(px[0], px[3]),
      over_white(px[1], px[3]),
      over_white(px[2], px[3]),
    );
    if gray {
      let y = ((299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b) + 500) / 1000) as u8;
      out.extend_from_slice(&[y, y, y]);
    } else {
      out.extend_from_slice(&[r, g, b]);
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fit_within_keeps_the_aspect_ratio_with_the_long_edge_at_the_box() {
    assert_eq!(fit_within(640, 480, 160), Some((160, 120)));
    assert_eq!(fit_within(480, 640, 160), Some((120, 160)));
    assert_eq!(fit_within(100, 100, 80), Some((80, 80)));
    // 小图按方框放大。
    assert_eq!(fit_within(40, 30, 160), Some((160, 120)));
    // 四舍五入：1000×333 → 320 × 106.56 → 107。
    assert_eq!(fit_within(1000, 333, 320), Some((320, 107)));
    // 极端比例：短边至少 1 像素，不会变成 0。
    assert_eq!(fit_within(10_000, 1, 80), Some((80, 1)));
    assert_eq!(fit_within(1, 10_000, 80), Some((1, 80)));
    for zero in [(0, 5, 80), (5, 0, 80), (5, 5, 0)] {
      assert_eq!(fit_within(zero.0, zero.1, zero.2), None, "{zero:?}");
    }
  }

  #[test]
  fn cover_rect_crops_the_overflowing_axis_from_the_middle() {
    // 4:3 的原图铺进 59×74（竖）：裁左右，取中间一条。
    let (x, y, w, h) = cover_rect(640, 480, 59, 74).unwrap();
    assert_eq!((y, h), (0, 480));
    assert_eq!(x, (640 - w) / 2);
    // 取出的矩形比例要与目标一致（整数除法至多差 1 个原图像素）。
    assert!((w * 74).abs_diff(h * 59) <= 74, "{w}×{h}");
    // 竖图铺进 320×240（横）：裁上下。
    let (x, y, w, h) = cover_rect(480, 640, 320, 240).unwrap();
    assert_eq!((x, w), (0, 480));
    assert_eq!(y, (640 - h) / 2);
    assert!((w * 240).abs_diff(h * 320) <= 320, "{w}×{h}");
    // 比例相同：整张图。
    assert_eq!(cover_rect(640, 480, 160, 120), Some((0, 0, 640, 480)));
    // 矩形永远落在原图里。
    for (sw, sh, dw, dh) in [(1, 1, 59, 74), (1000, 7, 320, 240), (7, 1000, 160, 120)] {
      let (x, y, w, h) = cover_rect(sw, sh, dw, dh).unwrap();
      assert!(
        w >= 1 && h >= 1 && x + w <= sw && y + h <= sh,
        "{sw}×{sh} → {dw}×{dh}"
      );
    }
    assert_eq!(cover_rect(0, 1, 1, 1), None);
    assert_eq!(cover_rect(1, 1, 1, 0), None);
  }

  #[test]
  fn opaque_pixels_pass_through_unchanged() {
    let rgba = [10, 20, 30, 255, 200, 100, 50, 255];
    assert_eq!(rgba_to_rgb(&rgba, false), vec![10, 20, 30, 200, 100, 50]);
  }

  /// 透明区域铺在白底上；半透明按比例混合。
  #[test]
  fn transparency_is_composited_over_white() {
    // 全透明的黑像素 → 白；半透明的黑（a = 128）→ 约 127；半透明的白仍是白。
    let rgba = [0, 0, 0, 0, 0, 0, 0, 128, 255, 255, 255, 128];
    let rgb = rgba_to_rgb(&rgba, false);
    assert_eq!(&rgb[0..3], &[255, 255, 255]);
    assert!(
      rgb[3..6].iter().all(|&v| (126..=128).contains(&v)),
      "{:?}",
      &rgb[3..6]
    );
    assert_eq!(&rgb[6..9], &[255, 255, 255]);
  }

  /// BT.601 亮度：纯红 76、纯绿 150、纯蓝 29（0.299 / 0.587 / 0.114 × 255），三个通道相同。
  #[test]
  fn grey_uses_bt601_luma_in_all_three_channels() {
    let rgba = [
      255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
    ];
    let rgb = rgba_to_rgb(&rgba, true);
    assert_eq!(
      rgb,
      vec![
        76, 76, 76, 150, 150, 150, 29, 29, 29, 255, 255, 255, 0, 0, 0
      ]
    );
  }

  #[test]
  fn output_length_is_three_bytes_per_pixel_and_ignores_a_ragged_tail() {
    assert_eq!(rgba_to_rgb(&[], false), Vec::<u8>::new());
    assert_eq!(rgba_to_rgb(&[1, 2, 3, 255, 9, 9], true).len(), 3);
    assert_eq!(rgba_to_rgb(&vec![0; 4 * 100], false).len(), 300);
  }
}

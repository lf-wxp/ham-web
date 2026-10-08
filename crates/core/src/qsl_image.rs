//! QSL 卡片影像的领域规则：存储 key 与上传校验（纯逻辑，可脱离浏览器单测）。
//!
//! 影像本体（缩放后的 dataURL）由前端存进 IndexedDB —— 扫描件动辄几百 KB，塞进 5MB 的
//! localStorage 快照会立刻把快照写挂，所以它走的是「只有权威层」的那条路
//! （`kv::save_store_only`）。
//!
//! 这里放不该依赖浏览器的部分：key 怎么拼（它是**持久化格式**，改了就读不出老数据）、
//! 以及上传前的类型 / 体积校验。

/// 单张扫描件的原始文件大小上限（字节）。
///
/// 手机直出照片常在 3–8 MB，按 12 MB 卡一道：更大的多半不是扫描件（或忘了裁剪），
/// 与其在浏览器里慢慢解码，不如当场告诉用户。
pub const MAX_SOURCE_BYTES: f64 = 12.0 * 1024.0 * 1024.0;

/// 单张图片**解码后**的像素总量上限（像素）。
///
/// 只卡压缩体积挡不住「解压炸弹」：几百 KB 的高压缩 PNG / WebP 可以解成几十亿像素，
/// 一进 canvas 就把 WASM 堆打爆。50 MP 是常见的手机 / 扫描仪上限之上的一档，
/// 正常照片不会碰到。
pub const MAX_SOURCE_PIXELS: u64 = 50_000_000;

/// 上传被拒绝的原因。
///
/// 只给原因、不给文案：文案由界面层按调用点的 `t("…")` 字面量取（静态扫描才认得出来），
/// 核心 crate 里也不该出现中文。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QslImageReject {
  /// 不是受支持的图片类型。
  UnsupportedType,
  /// 超过体积上限。
  TooLarge,
}

/// 影像在存储层的 key —— 一条通联一张，重新上传即替换。
#[must_use]
pub fn key_for(entry_id: u64) -> String {
  format!("qsl-image:{entry_id}")
}

/// 是否受支持的图片类型。
///
/// 只收 JPEG / PNG / WebP：`image/svg+xml` 可以内嵌脚本，而我们会把它当图片渲染，
/// 宁可少收一种也不给自己开一个注入口子。
#[must_use]
pub fn is_supported_mime(mime: &str) -> bool {
  matches!(
    mime.trim().to_ascii_lowercase().as_str(),
    "image/jpeg" | "image/jpg" | "image/png" | "image/webp"
  )
}

/// 上传前校验：类型与体积。
///
/// `bytes` 用 `f64`（与 `web_sys::File::size()` 一致）；非有限值一律按「过大」拒绝，
/// 免得把 `NaN` 放进来当成「没有大小」。
pub fn check(mime: &str, bytes: f64) -> Result<(), QslImageReject> {
  if !is_supported_mime(mime) {
    return Err(QslImageReject::UnsupportedType);
  }
  if !bytes.is_finite() || bytes > MAX_SOURCE_BYTES {
    return Err(QslImageReject::TooLarge);
  }
  Ok(())
}

/// 解码后校验像素总量（见 [`MAX_SOURCE_PIXELS`]）。
///
/// 与 [`check`] 分开是因为**解码前后**拿得到的信息不同：先按压缩体积挡一道（不浪费
/// 解码时间），解码出实际尺寸后再挡一道（防解压炸弹）。宽高为 0 也算失败 —— 那是
/// 解码没成功，不该继续往下走。
pub fn check_dimensions(width: u32, height: u32) -> Result<(), QslImageReject> {
  let pixels = u64::from(width) * u64::from(height);
  if width == 0 || height == 0 || pixels > MAX_SOURCE_PIXELS {
    return Err(QslImageReject::TooLarge);
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn keys_are_stable_and_distinct_per_entry() {
    // key 是持久化格式：改了就读不出用户已经存下的扫描件，所以逐字钉住。
    assert_eq!(key_for(1), "qsl-image:1");
    assert_eq!(key_for(42), "qsl-image:42");
    assert_ne!(key_for(4), key_for(44));
    // 两条通联不能共用一张影像。
    assert_ne!(key_for(u64::MAX), key_for(u64::MAX - 1));
  }

  #[test]
  fn supported_types_are_case_and_space_insensitive() {
    for mime in ["image/jpeg", "image/jpg", "image/png", "image/webp"] {
      assert!(is_supported_mime(mime), "{mime} 应被接受");
      assert!(
        is_supported_mime(&mime.to_uppercase()),
        "{mime} 大写也应被接受"
      );
    }
    assert!(is_supported_mime("  image/png  "));
    // SVG 可内嵌脚本；GIF / HEIC / 非图片一律拒绝。
    for mime in ["image/svg+xml", "image/gif", "image/heic", "text/plain", ""] {
      assert!(!is_supported_mime(mime), "{mime} 不应被接受");
    }
  }

  #[test]
  fn decoded_pixel_count_is_capped() {
    // 正常照片放行。
    assert_eq!(check_dimensions(4032, 3024), Ok(())); // 12 MP
    assert_eq!(check_dimensions(8000, 6250), Ok(())); // 正好 50 MP
    // 解压炸弹拒绝：20000 × 20000 = 400 MP。
    assert_eq!(
      check_dimensions(20000, 20000),
      Err(QslImageReject::TooLarge)
    );
    assert_eq!(
      check_dimensions(10000, 10000),
      Err(QslImageReject::TooLarge)
    );
    // 维度为 0 = 解码没成功，也要拒绝。
    assert_eq!(check_dimensions(0, 100), Err(QslImageReject::TooLarge));
    assert_eq!(check_dimensions(100, 0), Err(QslImageReject::TooLarge));
  }

  #[test]
  fn check_reports_the_first_problem_it_finds() {
    assert_eq!(check("image/png", 1024.0), Ok(()));
    assert_eq!(
      check("image/jpeg", MAX_SOURCE_BYTES),
      Ok(()),
      "刚好到上限应放行"
    );
    assert_eq!(
      check("image/gif", 10.0),
      Err(QslImageReject::UnsupportedType),
      "类型不对时先报类型"
    );
    assert_eq!(
      check("image/png", MAX_SOURCE_BYTES + 1.0),
      Err(QslImageReject::TooLarge)
    );
    // 空文件（0 字节）不算超限，交给解码阶段去失败。
    assert_eq!(check("image/png", 0.0), Ok(()));
    // 非有限值按过大处理。
    assert_eq!(check("image/png", f64::NAN), Err(QslImageReject::TooLarge));
    assert_eq!(
      check("image/png", f64::INFINITY),
      Err(QslImageReject::TooLarge)
    );
  }
}

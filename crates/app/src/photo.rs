//! 报名照片处理（纯前端、本地完成），算法对齐原项目使用的 compressorjs 1.2 配置：
//! 质量 0.8、PNG/JPEG 统一转为 JPEG、按最大/最小宽高等比缩放、JPEG 白底、
//! 同格式且无需缩放时若压缩后更大则保留原图（strict）。

use js_sys::{Function, Promise};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  Blob, CanvasRenderingContext2d, File, FileReader, HtmlCanvasElement, HtmlImageElement, Url,
};

use crate::i18n::{t, tf};
use crate::util::{document, js_error_message};

/// 照片类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoKind {
  /// 证件照。
  Id,
  /// 人像照。
  Profile,
}

struct Params {
  quality: f64,
  convert_types: &'static [&'static str],
  convert_size: f64,
  max_width: f64,
  max_height: f64,
  min_width: f64,
  min_height: f64,
}

impl PhotoKind {
  const fn params(self) -> Params {
    match self {
      Self::Id => Params {
        quality: 0.8,
        convert_types: &["image/png", "image/jpeg"],
        convert_size: 1.0,
        max_width: 4096.0,
        max_height: 3072.0,
        min_width: 1024.0,
        min_height: 768.0,
      },
      Self::Profile => Params {
        quality: 0.8,
        convert_types: &["image/png", "image/jpeg"],
        convert_size: 1.0,
        max_width: 3375.0,
        max_height: 4500.0,
        min_width: 300.0,
        min_height: 400.0,
      },
    }
  }
}

/// 处理结果。
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoResult {
  pub data_url: String,
  pub width: u32,
  pub height: u32,
  pub size: f64,
  pub mime: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fit {
  Contain,
  Cover,
}

fn adjusted(aspect: f64, width: f64, height: f64, fit: Fit) -> (f64, f64) {
  let valid = |v: f64| v > 0.0 && v.is_finite();
  match (valid(width), valid(height)) {
    (true, true) => {
      let adjusted_width = height * aspect;
      let use_height = match fit {
        Fit::Contain => adjusted_width > width,
        Fit::Cover => adjusted_width < width,
      };
      if use_height {
        (width, width / aspect)
      } else {
        (height * aspect, height)
      }
    }
    (true, false) => (width, width / aspect),
    (false, true) => (height * aspect, height),
    (false, false) => (width, height),
  }
}

fn normalize(v: f64) -> f64 {
  let r = (v * 1e11).round() / 1e11;
  if (v - r).abs() < 1e-9 { r } else { v }
}

/// 计算输出尺寸（与 compressorjs `draw()` 一致）。
fn target_size(p: &Params, natural_w: f64, natural_h: f64) -> (u32, u32) {
  let aspect = natural_w / natural_h;
  let (max_w, max_h) = adjusted(aspect, p.max_width, p.max_height, Fit::Contain);
  let (min_w, min_h) = adjusted(aspect, p.min_width, p.min_height, Fit::Cover);
  let w = normalize(natural_w.max(min_w).min(max_w)).floor();
  let h = normalize(natural_h.max(min_h).min(max_h)).floor();
  (w.max(1.0) as u32, h.max(1.0) as u32)
}

async fn promise<F>(f: F) -> Result<JsValue, JsValue>
where
  F: FnOnce(Function, Function),
{
  let mut f = Some(f);
  JsFuture::from(Promise::new(&mut |resolve, reject| {
    if let Some(f) = f.take() {
      f(resolve, reject);
    }
  }))
  .await
}

async fn load_image(src: &str) -> Result<HtmlImageElement, String> {
  let img = HtmlImageElement::new().map_err(|e| js_error_message(&e))?;
  let target = img.clone();
  let src = src.to_owned();
  promise(move |resolve, reject| {
    let ok = Closure::once_into_js(move || {
      let _ = resolve.call0(&JsValue::NULL);
    });
    let err = Closure::once_into_js(move || {
      let _ = reject.call1(&JsValue::NULL, &"Failed to load the image.".into());
    });
    target.set_onload(Some(ok.unchecked_ref()));
    target.set_onerror(Some(err.unchecked_ref()));
    target.set_src(&src);
  })
  .await
  .map_err(|e| js_error_message(&e))?;
  Ok(img)
}

async fn canvas_to_blob(
  canvas: &HtmlCanvasElement,
  mime: &str,
  quality: f64,
) -> Result<Blob, String> {
  let canvas = canvas.clone();
  let mime = mime.to_owned();
  let value = promise(move |resolve, reject| {
    let reject2 = reject.clone();
    let cb = Closure::once_into_js(move |blob: JsValue| {
      if blob.is_null() || blob.is_undefined() {
        let _ = reject.call1(&JsValue::NULL, &"Failed to compress the image.".into());
      } else {
        let _ = resolve.call1(&JsValue::NULL, &blob);
      }
    });
    if let Err(e) =
      canvas.to_blob_with_type_and_encoder_options(cb.unchecked_ref(), &mime, &quality.into())
    {
      let _ = reject2.call1(&JsValue::NULL, &e);
    }
  })
  .await
  .map_err(|e| js_error_message(&e))?;
  Ok(value.unchecked_into())
}

async fn read_data_url(blob: &Blob) -> Result<String, String> {
  let reader = FileReader::new().map_err(|e| js_error_message(&e))?;
  let target = reader.clone();
  let blob = blob.clone();
  promise(move |resolve, reject| {
    let reject2 = reject.clone();
    let ok = Closure::once_into_js(move || {
      let _ = resolve.call0(&JsValue::NULL);
    });
    let err = Closure::once_into_js(move || {
      let _ = reject.call1(&JsValue::NULL, &"无法读取处理后的图片数据".into());
    });
    target.set_onload(Some(ok.unchecked_ref()));
    target.set_onerror(Some(err.unchecked_ref()));
    if let Err(e) = target.read_as_data_url(&blob) {
      let _ = reject2.call1(&JsValue::NULL, &e);
    }
  })
  .await
  .map_err(|e| js_error_message(&e))?;
  reader
    .result()
    .ok()
    .and_then(|v| v.as_string())
    .ok_or_else(|| t("无法读取处理后的图片数据"))
}

async fn compress(kind: PhotoKind, file: &File) -> Result<Blob, String> {
  let p = kind.params();
  let url = Url::create_object_url_with_blob(file).map_err(|e| js_error_message(&e))?;
  let img = load_image(&url).await;
  let _ = Url::revoke_object_url(&url);
  let img = img?;
  let (nw, nh) = (
    f64::from(img.natural_width()),
    f64::from(img.natural_height()),
  );
  if nw <= 0.0 || nh <= 0.0 {
    return Err("Failed to load the image.".to_owned());
  }

  let file_type = file.type_();
  let mime = if p.convert_types.contains(&file_type.as_str()) && file.size() > p.convert_size {
    "image/jpeg".to_owned()
  } else if file_type.starts_with("image/") {
    file_type.clone()
  } else {
    "image/png".to_owned()
  };
  let (w, h) = target_size(&p, nw, nh);

  let canvas: HtmlCanvasElement = document()
    .create_element("canvas")
    .map_err(|e| js_error_message(&e))?
    .unchecked_into();
  canvas.set_width(w);
  canvas.set_height(h);
  let ctx: CanvasRenderingContext2d = canvas
    .get_context("2d")
    .map_err(|e| js_error_message(&e))?
    .ok_or("Canvas 2D context unavailable")?
    .unchecked_into();
  ctx.set_fill_style_str(if mime == "image/jpeg" {
    "#fff"
  } else {
    "transparent"
  });
  ctx.fill_rect(0.0, 0.0, f64::from(w), f64::from(h));
  ctx
    .draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, f64::from(w), f64::from(h))
    .map_err(|e| js_error_message(&e))?;
  let blob = canvas_to_blob(&canvas, &mime, p.quality).await?;

  let resized = p.min_width > nw || p.min_height > nh || p.max_width < nw || p.max_height < nh;
  if blob.size() > file.size() && mime == file_type && !resized {
    return Ok(file.clone().into());
  }
  Ok(blob)
}

/// 处理照片。
pub async fn process_photo(kind: PhotoKind, file: File) -> Result<PhotoResult, String> {
  let blob = compress(kind, &file).await.map_err(|e| {
    tf(
      "处理失败，请尝试更换照片格式，或对着照片截图再上传。详细信息：{}",
      &[&(e).to_string()],
    )
  })?;
  let data_url = read_data_url(&blob).await?;
  let img = load_image(&data_url)
    .await
    .map_err(|_| t("无法读取处理后的图片尺寸"))?;
  Ok(PhotoResult {
    data_url,
    width: img.natural_width(),
    height: img.natural_height(),
    size: blob.size(),
    mime: blob.type_(),
  })
}

/// 是否为图片文件。
pub fn is_image(file: &File) -> bool {
  file.type_().starts_with("image/")
}

/// 格式化文件大小（KB，两位小数）。
pub fn format_size(bytes: f64) -> String {
  format!("{:.2} KB", bytes / 1024.0)
}

/// 下载处理后的照片：`{原文件名}_processed.{扩展名}`。
pub fn download(result: &PhotoResult, original_name: &str) {
  let ext = result
    .mime
    .split('/')
    .nth(1)
    .filter(|s| !s.is_empty())
    .unwrap_or("jpg");
  let base = original_name
    .rfind('.')
    .filter(|&i| i > 0)
    .map_or(original_name, |i| &original_name[..i]);
  let Ok(el) = document().create_element("a") else {
    return;
  };
  let a: web_sys::HtmlAnchorElement = el.unchecked_into();
  a.set_href(&result.data_url);
  a.set_download(&format!("{base}_processed.{ext}"));
  if let Some(body) = document().body() {
    let _ = body.append_child(&a);
    a.click();
    let _ = body.remove_child(&a);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn scales_within_bounds() {
    let id = PhotoKind::Id.params();
    assert_eq!(target_size(&id, 8000.0, 6000.0), (4096, 3072));
    assert_eq!(target_size(&id, 400.0, 300.0), (1024, 768));
    assert_eq!(target_size(&id, 2000.0, 1500.0), (2000, 1500));
    let profile = PhotoKind::Profile.params();
    assert_eq!(target_size(&profile, 150.0, 200.0), (300, 400));
  }
}

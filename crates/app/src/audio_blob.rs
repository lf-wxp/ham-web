//! 把一段音频 `Blob`（站内录的或用户选的文件）读成 `data:` URL，供存进 IndexedDB。

use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, FileReader};

/// 读成 dataURL；读取失败为 `None`。
pub async fn data_url_of(blob: &Blob) -> Option<String> {
  let reader = FileReader::new().ok()?;
  let target = reader.clone();
  let blob = blob.clone();
  let done = Promise::new(&mut |resolve, reject| {
    target.set_onload(Some(&resolve));
    target.set_onerror(Some(&reject));
    if let Err(e) = target.read_as_data_url(&blob) {
      let _ = reject.call1(&JsValue::NULL, &e);
    }
  });
  JsFuture::from(done).await.ok()?;
  reader.result().ok()?.as_string()
}

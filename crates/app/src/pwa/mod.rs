//! PWA：注册 Service Worker（仅 release 构建）；发现新版本时提示刷新并列出更新内容，
//! 更新后展示「本次更新」，题库内容变化时提示具体改动。

mod notice;
mod update_notices;

pub use update_notices::UpdateNotices;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::ServiceWorkerRegistration;

use crate::util::window;

/// 开发构建不注册 Service Worker，并注销同源下残留的旧 SW（例如之前 release 版留下的），
/// 避免缓存与“发现新版本”弹窗干扰调试。
pub fn register_service_worker() {
  let container = window().navigator().service_worker();
  if cfg!(debug_assertions) {
    // 在 Leptos 挂载之前调用，此时 Leptos 执行器尚未初始化，直接使用 wasm-bindgen-futures
    wasm_bindgen_futures::spawn_local(async move {
      let Ok(list) = JsFuture::from(container.get_registrations()).await else {
        return;
      };
      for reg in js_sys::Array::from(&list).iter() {
        let reg: ServiceWorkerRegistration = reg.unchecked_into();
        if let Ok(p) = reg.unregister() {
          let _ = JsFuture::from(p).await;
        }
      }
    });
    return;
  }
  let _ = container.register("/sw.js");
}

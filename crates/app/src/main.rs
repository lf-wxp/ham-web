//! 业余无线电执照考试模拟：Leptos CSR 前端入口。

mod achievements;
mod app;
mod bank_updates;
mod cn;
mod components;
mod data;
mod exam_history;
mod gesture;
mod i18n;
mod icons;
mod idb;
mod kv;
mod morse_audio;
mod morse_settings;
mod motion;
mod pages;
mod photo;
mod push;
mod pwa;
#[cfg(test)]
mod registry_check;
mod sat_alert;
mod share_score;
mod shortcuts;
mod speech;
mod store;
mod study;
mod theme;
pub mod ui;
mod util;
mod web_threads;

/// panic 时除了打印到控制台，还调用 `index.html` 中的 `__hamFatal` 显示兜底页。
fn install_panic_hook() {
  std::panic::set_hook(Box::new(|info| {
    console_error_panic_hook::hook(info);
    let window = util::window();
    if let Ok(f) = js_sys::Reflect::get(&window, &"__hamFatal".into())
      && let Some(f) = wasm_bindgen::JsCast::dyn_ref::<js_sys::Function>(&f)
    {
      let _ = f.call2(
        &window,
        &i18n::t("页面出错了").into(),
        &info.to_string().into(),
      );
    }
  }));
}

fn main() {
  install_panic_hook();
  pwa::register_service_worker();
  // 多标签页同时打开时，各自的内存缓存需感知对方的写入。
  store::install_cross_tab_sync();
  leptos::mount::mount_to_body(app::App);
  if let Some(boot) = util::window()
    .document()
    .and_then(|d| d.get_element_by_id("boot"))
  {
    boot.remove();
  }
}

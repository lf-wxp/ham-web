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
mod radio_law_check;
#[cfg(test)]
mod registry_check;
mod rpg;
mod sat_alert;
mod share_score;
mod shortcuts;
mod speech;
mod store;
mod study;
mod theme;
pub mod ui;
mod util;

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
        &i18n::t("common.something-went-wrong").into(),
        &info.to_string().into(),
      );
    }
  }));
}

/// 撤掉 `index.html` 里的启动占位（骨架屏 / 首屏文案），露出真正的应用。
fn remove_boot() {
  if let Some(boot) = util::window()
    .document()
    .and_then(|d| d.get_element_by_id("boot"))
  {
    boot.remove();
  }
}

/// 挂载兜底超时（毫秒）：语言包拉取是首屏唯一的网络依赖，卡住也要放人进去。
///
/// 取「包超时 + 余量」而不是更小的值：兜底**不能**早于包超时，否则非内嵌域会先渲染
/// 成中文。包迟到的极端场景（如超时机制本身失效）由 `i18n::pack` 的版本信号自愈 ——
/// 包到达后已渲染的视图会重算；拉取失败另有 `pack::schedule_retry` 的有界重试。
///
/// 包超时本身已从 6 秒收紧到 3 秒（`pack.rs` 有取舍说明），因此这里的兜底是 4 秒。
const BOOT_TIMEOUT_MS: i32 = i18n::PACK_TIMEOUT_MS + 1000;

thread_local! {
  /// 是否已经挂载：`mount_to_body` 只能调一次，而语言包加载与兜底定时器会各试一次。
  static MOUNTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 挂载应用并撤掉骨架屏；重复调用是空操作。
fn mount_app() {
  if MOUNTED.with(|m| m.replace(true)) {
    return;
  }
  leptos::mount::mount_to_body(app::App);
  remove_boot();
}

fn main() {
  install_panic_hook();
  pwa::register_service_worker();
  // 多标签页同时打开时，各自的内存缓存需感知对方的写入。
  store::install_cross_tab_sync();
  // 先载语言包再挂载：`t()` 是同步查表，非内嵌域的译文在包里（`i18n::pack`），
  // 先挂载会让首屏先渲染中文、包到达后再翻成译文。中文没有包，这里直接往下走。
  //
  // 用 `wasm_bindgen_futures` 而不是 `leptos::task::spawn_local`：后者的执行器要等
  // Leptos 运行时起来才注册，而这一步恰恰发生在挂载之前。
  wasm_bindgen_futures::spawn_local(async {
    i18n::load_stored().await;
    mount_app();
  });

  // 兜底：`fetch_text_with_timeout` 的 6 秒上限靠 `set_timeout` 生效，万一它没起来
  // （或 fetch 永不 settle），用户会被永久关在骨架屏里。到点直接挂载，退化成
  // 「内嵌域有译文、其余中文」——总好过白屏，也好过点了没反应。
  use wasm_bindgen::JsCast;
  let cb = wasm_bindgen::closure::Closure::once_into_js(mount_app);
  let _ = util::window().set_timeout_with_callback_and_timeout_and_arguments_0(
    cb.as_ref().unchecked_ref(),
    BOOT_TIMEOUT_MS,
  );
}

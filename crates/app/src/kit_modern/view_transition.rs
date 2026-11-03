//! 原生 View Transitions 封装。
//!
//! `document.startViewTransition(cb)` 会在 cb 执行前后各拍一次快照，然后做差异过渡。
//! 注意：Leptos 的 DOM 更新是异步的，所以回调必须返回一个「等一帧」的 Promise，
//! 否则快照拍在更新之前，等于什么都没过渡。
//!
//! 浏览器不支持时直接执行 `f()`，行为与没有这个能力完全一致。

use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;

/// 在 View Transition 中执行 `f`（不支持则直接执行）。
pub fn with_view_transition<F: FnOnce() + 'static>(f: F) {
  let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
    f();
    return;
  };
  let start = js_sys::Reflect::get(&doc, &JsValue::from_str("startViewTransition"))
    .ok()
    .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
  let Some(start) = start else {
    f();
    return;
  };

  let cb = Closure::once_into_js(move || {
    f();
    // 等一帧再 resolve：让 Leptos 把新 DOM 写进去，快照才拍得到「之后」的样子。
    js_sys::Promise::new(&mut |resolve, _reject| {
      let done = Closure::once_into_js(move || {
        let _ = resolve.call0(&JsValue::NULL);
      });
      if let Some(w) = web_sys::window() {
        let _ = w.request_animation_frame(done.unchecked_ref());
      }
    })
  });

  let _ = start.call1(&doc, &cb);
}

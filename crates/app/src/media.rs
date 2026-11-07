//! 媒体查询的响应式镜像：给「两套布局只建一套」用（见 [`use_media_query`]）。

use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// 与 Tailwind `md:` 同一条断点（`48rem`；媒体查询里的 rem 按浏览器初始字号算，与 CSS 一致）。
pub const MD_UP: &str = "(min-width: 48rem)";

/// `query` 当前是否命中，并在命中状态变化时更新（窗口缩放、横竖屏切换）。
///
/// 给「桌面一套、手机一套，用 `hidden md:block` / `md:hidden` 二选一显示」的页面用：
/// 两套都建出来再藏一套，等于每次进页面都白建一半（`/bands` 的波段表与波段卡片各几百个节点）。
/// 改成按这个信号只建一套，跨断点时再换。
///
/// 必须在组件体里调用：监听器随组件卸载移除。不支持 `matchMedia` 时按 `false` 处理。
pub fn use_media_query(query: &'static str) -> Signal<bool> {
  let Some(mql) = crate::util::window().match_media(query).ok().flatten() else {
    return Signal::from(false);
  };
  let matches = RwSignal::new(mql.matches());
  let target = mql.clone();
  let on_change = Closure::<dyn Fn()>::new(move || {
    let _ = matches.try_set(target.matches());
  });
  let _ = mql.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
  let keep = SendWrapper::new((mql, on_change));
  on_cleanup(move || {
    let (mql, on_change) = keep.take();
    let _ = mql.remove_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
  });
  matches.into()
}

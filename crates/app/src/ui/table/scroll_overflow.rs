//! 横向滚动容器的「是否溢出」检测：[`Table`](super::Table) 与 [`ScrollRegion`](super::ScrollRegion) 共用。

use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::closure::Closure;

/// 盯着 `container` 的横向溢出（`scrollWidth > clientWidth`），返回「当前是否溢出」。
///
/// 溢出才可聚焦：既保住键盘可达，又不给每张短表加一个多余的 Tab 停靠点。
/// 观察容器与它的第一个子元素（表格）两个盒子 —— 只观察容器的话，列宽变化
/// （`min-w-[…]` 生效、文案折行）不会触发回调。
///
/// 观察器与它的闭包都要活到组件卸载：`on_load` 里注册的 `on_cleanup` 不在组件 Owner 下，
/// 所以句柄存在 `Rc` 里，由调用方组件体里的 `on_cleanup` 统一释放（本函数必须在组件体内调用）。
/// `on_cleanup` 要求 `Send + Sync`：句柄过不了线程边界（`ResizeObserver` 是 JS 对象），
/// 用 `SendWrapper` 声明「只在主线程用」，与 `tooltip` 里的共享气泡同一手法。
pub(super) fn use_scroll_overflow(container: NodeRef<leptos::html::Div>) -> RwSignal<bool> {
  let scrollable = RwSignal::new(false);
  let observer = SendWrapper::new(std::rc::Rc::new(std::cell::RefCell::new(
    None::<(web_sys::ResizeObserver, Closure<dyn FnMut()>)>,
  )));
  {
    let observer = observer.clone();
    container.on_load(move |el| {
      let target = el.clone();
      let check = Closure::<dyn FnMut()>::new(move || {
        // `clientWidth` 不含滚动条，正好与 `scrollWidth` 同口径。
        scrollable.set(target.scroll_width() > target.client_width());
      });
      let Ok(ro) = web_sys::ResizeObserver::new(check.as_ref().unchecked_ref()) else {
        return;
      };
      ro.observe(&el);
      if let Some(table) = el.first_element_child() {
        ro.observe(&table);
      }
      // 首帧先算一次：`ResizeObserver` 的回调是异步的，首屏不能等它。
      scrollable.set(el.scroll_width() > el.client_width());
      *observer.borrow_mut() = Some((ro, check));
    });
  }
  on_cleanup(move || {
    if let Some((ro, _)) = observer.borrow_mut().take() {
      ro.disconnect();
    }
  });
  scrollable
}

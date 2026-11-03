//! 现代风的动效原语。
//!
//! 像素风的动效是「逐帧跳」（`steps()`），现代风要的是「有惯性」：指针聚光、滚动落点高亮
//! 由这里提供，页面只负责把它们挂到元素上。（数字滚动在 [`crate::motion::use_count_up`]，
//! 两套皮肤共用一份。）三条共同约束：
//!
//! 1. **降级必须是无害的**：动效被关（系统「减少动态效果」或设置里关掉动效）时，
//!    数字直接落位、聚光不写变量 —— 不会留下「卡在半路」的状态；
//! 2. **不订阅不该订阅的信号**：这些 helper 在组件体里调用一次，内部自带 `Effect`；
//! 3. **不产生常驻开销**：监听在卸载时移除，滚动落点用 rAF 合帧。

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// 指针聚光：把指针位置写成 `--mx / --my`（百分比），配合 `.md-spotlight::after`。
///
/// 为什么用百分比而不是像素：聚光跟着卡片的圆角一起缩放，卡片在网格里改宽度时
/// 不必重新算；`radial-gradient` 的半径另用 `px`，两者互不干扰。
///
/// 写成普通函数而不是返回闭包：函数项是 `Copy`，同一页挂十张卡也不用逐个 `clone()`。
pub fn spotlight(ev: web_sys::PointerEvent) {
  let Some(target) = ev.current_target() else {
    return;
  };
  let Ok(el) = target.dyn_into::<web_sys::Element>() else {
    return;
  };
  let rect = el.get_bounding_client_rect();
  if rect.width() <= 0.0 || rect.height() <= 0.0 {
    return;
  }
  let Some(style) = el.dyn_ref::<web_sys::HtmlElement>().map(|h| h.style()) else {
    return;
  };
  let x = (f64::from(ev.client_x()) - rect.left()) / rect.width() * 100.0;
  let y = (f64::from(ev.client_y()) - rect.top()) / rect.height() * 100.0;
  let _ = style.set_property("--mx", &format!("{x:.2}%"));
  let _ = style.set_property("--my", &format!("{y:.2}%"));
}

/// 视口上沿往下多少像素算「正在读这一节」。
///
/// 用一条线而不是可见面积：长计算器面板的可见面积永远比标题大，按占比判会让高亮
/// 停在上一节，滚到面板底部都换不过去。
const SPY_LINE: f64 = 140.0;

/// 滚动落点：观察一组 `#id`，返回「当前阅读到的那一节」。
///
/// 传进来的 id 全都不在 DOM 里时返回 `None`（页面主体可能还没渲染）。
#[must_use]
pub fn use_scroll_spy(ids: Vec<&'static str>) -> Signal<Option<&'static str>> {
  let active = RwSignal::new(Option::<&'static str>::None);

  Effect::new(move |_| {
    let Some(window) = web_sys::window() else {
      return;
    };
    let Some(doc) = window.document() else {
      return;
    };
    // 每次运行都从捕获的列表复制一份：直接把它 move 进 `measure` 会让外层闭包
    // 只能实现 `FnOnce`，而 `Effect` 的回调要能被反复调用。
    let ids = ids.clone();

    let spy_window = window.clone();
    let measure: Rc<dyn Fn()> = Rc::new(move || {
      let y = spy_window.scroll_y().unwrap_or(0.0) + SPY_LINE;
      let mut current = None;
      for id in &ids {
        let Some(el) = doc.get_element_by_id(id) else {
          continue;
        };
        let top = el.get_bounding_client_rect().top() + spy_window.scroll_y().unwrap_or(0.0);
        if top <= y {
          current = Some(*id);
        }
      }
      // `try_*`：已排队的 rAF 可能在页面卸载后才跑，`active` 那时已被释放。
      if active.try_get_untracked() != Some(current) {
        active.try_set(current);
      }
    });

    // 滚动的合帧：不做这个，长页面上每帧会跑十几次 `get_bounding_client_rect`（强制重排）。
    let ticking = Rc::new(Cell::new(false));
    let on_scroll = {
      let ticking = Rc::clone(&ticking);
      let measure = Rc::clone(&measure);
      Closure::<dyn Fn()>::new(move || {
        if ticking.replace(true) {
          return;
        }
        let ticking = Rc::clone(&ticking);
        let measure = Rc::clone(&measure);
        let cb = Closure::once_into_js(move || {
          ticking.set(false);
          measure();
        });
        if let Some(w) = web_sys::window() {
          let _ = w.request_animation_frame(cb.unchecked_ref());
        }
      })
    };

    let _ = window.add_event_listener_with_callback("scroll", on_scroll.as_ref().unchecked_ref());
    measure();

    let handle = SendWrapper::new((window, on_scroll));
    on_cleanup(move || {
      let (window, on_scroll) = handle.take();
      let _ =
        window.remove_event_listener_with_callback("scroll", on_scroll.as_ref().unchecked_ref());
      drop(on_scroll);
    });
  });

  active.into()
}

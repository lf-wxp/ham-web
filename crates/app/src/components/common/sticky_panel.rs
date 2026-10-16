//! 页内粘性面板：目录导航、侧栏这类「第二根吸顶栏」的通用外壳。

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use leptos::{ev, html, prelude::*};
use send_wrapper::SendWrapper;
use wasm_bindgen::prelude::*;
use web_sys::ResizeObserver;

use crate::cn::cn;

/// `ResizeObserver` 及其回调闭包。`Closure` 不是 Send，而 `on_cleanup` 要求 Send + 'static，
/// 用 `SendWrapper` 送进去（leptos 清理回调实际只在主线程执行，见 nav_bar 的同款用法）。
type PanelSizeWatcher = SendWrapper<Option<(ResizeObserver, Closure<dyn FnMut(js_sys::Array)>)>>;

/// `animationend` 监听器与它的回调闭包（同样只在主线程执行）。
type AnimationWatcher =
  SendWrapper<Option<(web_sys::EventTarget, Closure<dyn FnMut(web_sys::Event)>)>>;

/// 页头的定位钩子：`PageHeader` 打在自己 `<header>` 上的标记。
///
/// 只按 `header` 元素名找不行：页面内容里也有 `<header>`（如 `diy_projects` 的分区标题），
/// 万一它排在页头前面就会量错元素 —— 面板会吸在没有页头的位置。
const PAGE_HEADER_MARKER: &str = "[data-page-header]";

/// 页内粘性面板：吸在吸顶页头正下方，偏移随页头高度自适应。
///
/// 手写 `top-[136px]` 这类固定偏移有两个坑：页头随宽度 / 译文长度折行，高度一变，
/// 固定偏移要么在吸顶瞬间相对页头错位滑动（偏移 ≠ 自然位置），要么直接插进页头底下被盖住。
/// 本组件量出「页头底边 + 面板与页头底边之间的原始间距」作为吸顶偏移，页头怎么变
/// （折行、切语言、断点切换）都自动跟随，吸顶前后面板位置零错位。
///
/// 面板元素上同时写一个 CSS 变量 `--sticky-top`（px），面板内需要限高的滚动子元素
/// 用它计算剩余高度，如 `max-h-[calc(100vh-var(--sticky-top)-16px)]`。
///
/// 测量方式：把面板临时设为 `position: static` 读它的自然位置（吸顶状态下读
/// `getBoundingClientRect` 得到的是吸顶后的位置，会被旧偏移污染），
/// 与页头底边做差得到间距 —— 两者的 rect 受同一入场 translate 影响，差值不受污染，
/// 所以何时量、滚动到哪了量都对。
#[component]
pub fn StickyPanel(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
  let node = NodeRef::<html::Div>::new();
  Effect::new(move |_| {
    // `node.get()` 是响应式的：挂载后从 None 变 Some 时本效果重跑并完成接线。
    let Some(el) = node.get() else { return };
    let Some(header) = crate::util::document()
      .query_selector(PAGE_HEADER_MARKER)
      .ok()
      .flatten()
      .or_else(|| {
        // 兜底：没打标记的页头（手写 `PAGE_HEADER_BAR` 骨架的页面）。
        crate::util::document()
          .query_selector("main header")
          .ok()
          .flatten()
      })
    else {
      return;
    };
    // 面板与页头底边之间的原始间距：量一次后固定（PageContainer 的内边距是静态的）。
    // 偏移 = 页头底边 + 间距；页头 rect.bottom 在自然位与吸顶位都在同一处（66px 起），
    // 不受滚动影响，任何时刻重算都不会被吸顶状态干扰。
    let gap: Rc<Cell<Option<f64>>> = Rc::new(Cell::new(None));
    let measure_gap = {
      let header = header.clone();
      let el = el.clone();
      let gap = Rc::clone(&gap);
      move || {
        if gap.get().is_some() {
          return;
        }
        // 临时 static 读自然位置（吸顶中直接读会得到吸顶后的位置）。
        // 注意 `style` 要用 web_sys 的完整路径：tachys 的 ElementExt 也有同名 `style(_)`,
        // 抢解析后少一个参数（见 motion.rs 里的同款说明）。
        let style = web_sys::HtmlElement::style(&el);
        let prev = style.get_property_value("position").unwrap_or_default();
        let header_style = header
          .clone()
          .unchecked_into::<web_sys::HtmlElement>()
          .style();
        let header_prev = header_style
          .get_property_value("position")
          .unwrap_or_default();
        let _ = style.set_property("position", "static");
        let _ = header_style.set_property("position", "static");
        let rect = el.get_bounding_client_rect();
        let header_bottom = header.get_bounding_client_rect().bottom();
        if header_prev.is_empty() {
          let _ = header_style.remove_property("position");
        } else {
          let _ = header_style.set_property("position", &header_prev);
        }
        if prev.is_empty() {
          let _ = style.remove_property("position");
        } else {
          let _ = style.set_property("position", &prev);
        }
        // 隐藏（如移动端 aside）时 rect 为 0，量出的 gap 是垃圾值：保持 None，下次 apply 重试。
        if rect.width() > 0.0 || rect.height() > 0.0 {
          // 入场动效的 translate 对面板与页头同样生效，差值不受污染。
          gap.set(Some(rect.top() - header_bottom));
        }
      }
    };
    let apply = {
      let header = header.clone();
      let el = el.clone();
      let gap = Rc::clone(&gap);
      move || {
        measure_gap();
        let top = (header.get_bounding_client_rect().bottom() + gap.get().unwrap_or(0.0)).max(0.0);
        let style = web_sys::HtmlElement::style(&el);
        let _ = style.set_property("top", &format!("{top}px"));
        let _ = style.set_property("--sticky-top", &format!("{top}px"));
      }
    };

    // 首次测量延后到路由入场动效结束后，避免页头底边还带着动效偏移。
    // 400 ms 只是兜底（页面没有入场动效时 `animationend` 不会来）；动效比 400 ms 长时，
    // 下面的 `animationend` 会再补一次正确测量 —— `apply` 幂等，重复调用无害。
    // set_timeout 无可取消句柄；若面板已随路由卸载，回调只是对游离节点读 rect / 设样式，无害。
    set_timeout(
      {
        let apply = apply.clone();
        move || apply()
      },
      Duration::from_millis(400),
    );

    // 页头折行 / 切语言 → 页头高度变化 → ResizeObserver 触发重算
    // （ResizeObserver 观察的是布局尺寸，不受 transform 干扰）。
    let observer_cb = {
      let apply = apply.clone();
      Closure::<dyn FnMut(js_sys::Array)>::new(move |_entries: js_sys::Array| apply())
    };
    if let Ok(obs) = ResizeObserver::new(observer_cb.as_ref().unchecked_ref::<js_sys::Function>()) {
      obs.observe(&header);
      let keep: PanelSizeWatcher = SendWrapper::new(Some((obs, observer_cb)));
      on_cleanup(move || {
        if let Some((obs, _cb)) = keep.take() {
          obs.disconnect();
        }
      });
    }
    // 入场动效结束时补量一次：动效时长由 `--motion-normal` 决定，比上面那个固定
    // 400 ms 更可信；将来把动效调长也不会量到「动到一半」的页头底边。
    let anim_cb = {
      let apply = apply.clone();
      Closure::<dyn FnMut(web_sys::Event)>::new(move |_e: web_sys::Event| apply())
    };
    let anim_target: web_sys::EventTarget = header.clone().unchecked_into();
    let _ = anim_target
      .add_event_listener_with_callback("animationend", anim_cb.as_ref().unchecked_ref());
    let keep_anim: AnimationWatcher = SendWrapper::new(Some((anim_target, anim_cb)));
    on_cleanup(move || {
      if let Some((target, cb)) = keep_anim.take() {
        let _ = target.remove_event_listener_with_callback(
          "animationend",
          cb.as_ref().unchecked_ref::<js_sys::Function>(),
        );
      }
    });
    // 窗口缩放也重算：跨断点后页头高度会变，且给 ResizeObserver 失灵的环境兜底
    // （与上面的 observer 并存是双保险，apply 幂等，重复触发无害）。
    let resize_handle = window_event_listener(ev::resize, {
      let apply = apply.clone();
      move |_| apply()
    });
    on_cleanup(move || resize_handle.remove());
  });

  view! {
    // `z-10`：页头骨架是 `z-20`，面板自带一层，才不会被后面渲染的内容（卡片、粘性表头）
    // 盖住；同时仍在页头之下，吸顶时不会挡住页头。
    <div node_ref=node class=cn(&["sticky z-10", &class])>{children()}</div>
  }
}

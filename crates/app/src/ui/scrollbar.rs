//! 窗口滚动条（自绘）：隐藏原生的，改画一条只覆盖**内容区**的像素滚动条。
//!
//! 为什么不用原生窗口滚动条：站点的导航是吸顶的，而原生滚动条属于视口 —— 它从 y=0 就开始，
//! 会贴着导航右端一路到页脚（再配上 12px 的粗描边，看起来像在头部上切了一刀）。像素风里
//! 更合适的读法是「导航是机身的边框，滚动条属于屏幕」：滚动条从导航下沿开始。
//!
//! 为什么只换滚动条、不动滚动容器：把滚动容器从窗口挪到内容区（app shell）会连带改
//! `util::scroll_to_element`、路由复位、iOS 地址栏收起、嵌套吸顶的全部计算；而这里要的只是
//! 「滚动条别盖住头部」。隐藏的原生窗口滚动条由 CSS 完成，根元素上两条并存 ——
//! `html { scrollbar-width: none }`（Chromium 121+ / Firefox）与
//! `html::-webkit-scrollbar { display: none }`（老 Chromium / Safari）；少任何一条，
//! 对应内核就会多出第二条原生滚动条。嵌套滚动区（表格 / 弹层 / 侧栏）仍是原生样式。
//!
//! 只做三件事：按 `scrollY` 摆 thumb、拖动 thumb 改 `scrollY`、内容不够长就不显示。
//! 键盘 / 滚轮 / 触屏滚动完全走原生，这里不做拦截。触屏设备没有常驻滚动条，整条隐藏。

use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use wasm_bindgen::JsCast;

use crate::util::{document, window};

/// thumb 的最小高度（px）：太短不好拖，也看不出是「一段」。
const MIN_THUMB: f64 = 24.0;
/// 浮点误差容差：`scrollHeight` 与 `clientHeight` 相等时不该显示滚动条。
const EPS: f64 = 1.0;

#[component]
pub fn ScrollBar() -> impl IntoView {
  let host = NodeRef::<html::Div>::new();
  let thumb = NodeRef::<html::Div>::new();
  // 导航高度：吸顶条，滚动条从它下沿开始。缓存起来，避免每帧读 `offsetHeight`。
  let nav_h = StoredValue::new(0.0);
  // 拖动状态：按下时的指针 y 与当时的 `scrollY`。
  let dragging = StoredValue::new(false);
  let grab_y = StoredValue::new(0.0);
  let grab_scroll = StoredValue::new(0.0);
  // 上一帧算出的 thumb 高度与可移动行程：拖动时要用，避免每帧重算。
  let thumb_h = StoredValue::new(MIN_THUMB);
  let travel = StoredValue::new(1.0);

  // 量一次导航高度并写到 `top`：导航在断点间会换高度（桌面两行 / 手机一行）。
  let measure_nav = move || {
    let h = document()
      .query_selector("[data-nav]")
      .ok()
      .flatten()
      .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
      .map(|el| f64::from(el.offset_height()))
      .unwrap_or(0.0);
    nav_h.set_value(h);
    if let Some(host) = host.get_untracked() {
      let _ = web_sys::HtmlElement::from(host)
        .style()
        .set_property("top", &format!("{h}px"));
    }
  };

  // 按当前 `scrollY` 摆 thumb；页面不够长就把整条隐藏。
  let sync = move || {
    let (Some(host_el), Some(thumb_el)) = (host.get_untracked(), thumb.get_untracked()) else {
      return;
    };
    // `NodeRef` 拿到的是 tachys 的 `html::Div`，它的 `.style()` 是 builder（要传值）；
    // 转成 `web_sys::HtmlElement` 才能拿到可增量修改的 `CssStyleDeclaration`。
    let host_el = web_sys::HtmlElement::from(host_el);
    let thumb_el = web_sys::HtmlElement::from(thumb_el);
    let Some(root) = document().document_element() else {
      return;
    };
    let viewport = f64::from(root.client_height());
    let page = f64::from(root.scroll_height());
    let avail = viewport - nav_h.get_value();
    let scrollable = page > viewport + EPS && avail > MIN_THUMB * 2.0;
    let _ = host_el
      .style()
      .set_property("display", if scrollable { "block" } else { "none" });
    if !scrollable {
      return;
    }
    // thumb 高度按「可见比例」算，但不小于 MIN_THUMB；行程是轨道减去 thumb。
    let h = (avail * viewport / page).clamp(MIN_THUMB, avail);
    let t = (avail - h).max(1.0);
    let progress =
      (window().scroll_y().unwrap_or(0.0) / (page - viewport).max(1.0)).clamp(0.0, 1.0);
    thumb_h.set_value(h);
    travel.set_value(t);
    let _ = thumb_el.style().set_property("height", &format!("{h}px"));
    let _ = thumb_el
      .style()
      .set_property("transform", &format!("translateY({}px)", progress * t));
  };

  window_event_listener(leptos::ev::scroll, move |_| {
    request_animation_frame(sync);
  });
  window_event_listener(leptos::ev::resize, move |_| {
    request_animation_frame(move || {
      measure_nav();
      sync();
    });
  });

  // 首次挂载：等字体 / 图标落位后再量（一次是挂载当帧，一次是稍后）。
  request_animation_frame(move || {
    measure_nav();
    sync();
  });
  set_timeout(
    move || {
      measure_nav();
      sync();
    },
    std::time::Duration::from_millis(600),
  );

  // 路由切换 / 异步数据到位都会改文档高度：跟着重算（此时用户还没滚动）。
  let location = use_location();
  Effect::new(move |_| {
    let _ = location.pathname.get();
    request_animation_frame(move || {
      measure_nav();
      sync();
    });
    set_timeout(
      move || {
        measure_nav();
        sync();
      },
      std::time::Duration::from_millis(400),
    );
  });

  // 拖动：按下记起点，移动按行程比例换算成 `scrollY`，抬起结束。
  let start_drag = move |e: web_sys::PointerEvent| {
    dragging.set_value(true);
    grab_y.set_value(f64::from(e.client_y()));
    grab_scroll.set_value(window().scroll_y().unwrap_or(0.0));
    e.prevent_default();
  };
  let drag = move |e: web_sys::PointerEvent| {
    if !dragging.get_value() {
      return;
    }
    let dy = f64::from(e.client_y()) - grab_y.get_value();
    let Some(root) = document().document_element() else {
      return;
    };
    let max_scroll = f64::from(root.scroll_height()) - f64::from(root.client_height());
    let delta = dy / travel.get_value() * max_scroll.max(1.0);
    window().scroll_to_with_x_and_y(0.0, grab_scroll.get_value() + delta);
  };
  let stop_drag = move |_: web_sys::PointerEvent| dragging.set_value(false);

  view! {
    // `aria-hidden`：它是原生滚动的视觉替身，读屏与键盘用户不需要它（键盘滚动照常）。
    // `pointer-events-none` 在外壳上：只有 thumb 接指针，别挡住右边缘的内容点击。
    <div node_ref=host aria-hidden="true" class="pxl-scrollbar">
      <div
        node_ref=thumb
        class="pxl-scrollbar-thumb"
        on:pointerdown=start_drag
        on:pointermove=drag
        on:pointerup=stop_drag
        on:pointercancel=stop_drag
        on:pointerleave=stop_drag
      ></div>
    </div>
  }
}

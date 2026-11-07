//! 窗口滚动条（自绘）：隐藏原生的，改画一条只覆盖**内容区**的滚动条（两套皮肤共用）。
//!
//! 为什么不用原生窗口滚动条：站点的导航是吸顶的、底部还有贴底的固定层（底部 Tab / 答题操作栏 /
//! 底部 dock），而原生滚动条属于视口 —— 它从 y=0 就开始、一直画到视口底，会贴着导航右端和底栏
//! 右端（再配上粗描边，看起来像在头部和底栏上各切了一刀）。两套皮肤都采用同一种读法：
//! 「导航与底栏是机身的边框，滚动条属于屏幕」—— 滚动条只画在这两条固定层之间。
//!
//! 为什么只换滚动条、不动滚动容器：把滚动容器从窗口挪到内容区（app shell）会连带改
//! `util::scroll_to_element`、路由复位、iOS 地址栏收起、嵌套吸顶的全部计算；而这里要的只是
//! 「滚动条别盖住头部和底栏」。隐藏的原生窗口滚动条由 CSS 完成，根元素上两条并存 ——
//! `html { scrollbar-width: none }`（Chromium 121+ / Firefox）与
//! `html::-webkit-scrollbar { display: none }`（老 Chromium / Safari）；少任何一条，
//! 对应内核就会多出第二条原生滚动条。嵌套滚动区（表格 / 弹层 / 侧栏）仍是原生样式。
//!
//! 只做四件事：按 `scrollY` 摆 thumb、拖动 thumb 改 `scrollY`、量出上下两条固定层的占位、
//! 内容不够长就不显示。
//! 键盘 / 滚轮 / 触屏滚动完全走原生，这里不做拦截。触屏设备没有常驻滚动条，整条隐藏。
//!
//! 外观按风格分派：像素风 `.pxl-scrollbar`（12px 方块滑块），现代风 `.md-scrollbar`（10px 细圆头）。
//! 两套都只是**外壳 + thumb**，几何与行为全在这里，CSS 只管长相。

use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::ResizeObserver;

use crate::util::{document, window};

/// 吸顶导航的标记（两套顶栏都打）。
const NAV_SELECTOR: &str = "[data-nav]";
/// 贴底固定层的标记：新增 `fixed` 贴底的元素必须带它（`check-structure` 有守卫）。
const FIXED_BOTTOM_SELECTOR: &str = "[data-fixed-bottom]";

/// thumb 的最小高度（px）：太短不好拖，也看不出是「一段」。
const MIN_THUMB: f64 = 24.0;
/// 浮点误差容差：`scrollHeight` 与 `clientHeight` 相等时不该显示滚动条。
const EPS: f64 = 1.0;

/// 贴底固定层占掉的高度：取所有可见 `[data-fixed-bottom]` 里最高的那一条。
///
/// 外壳上同时挂着多条候选是常态（底部 Tab / dock 常驻，答题页用 `body.no-site-footer`
/// 把它们 `display: none` 掉、换成操作栏），所以按实测取最大：不可见的那条 `rect.height` 是 0，
/// 自然落选。底边只允许一层（见 `docs/ui/modern.md` §3），取最大而不是求和，避免哪天两条
/// 同时可见时把滚动条压到屏幕中间去。
///
/// 用 `rect.top` 而不是 `offsetHeight`：现代风的练习底栏在窄屏上被抬到 dock 之上
/// （`inset-block-end: calc(66px + …)`），只量高度会让滚动条伸进它与屏幕底边之间的空档。
fn bottom_inset() -> f64 {
  let viewport = window().inner_height().ok().and_then(|h| h.as_f64());
  let Some(viewport) = viewport else {
    return 0.0;
  };
  fixed_bottom_elements()
    .iter()
    .map(web_sys::Element::get_bounding_client_rect)
    .filter(|rect| rect.height() > 0.0)
    .map(|rect| f64::max(viewport - rect.top(), 0.0))
    .fold(0.0, f64::max)
}

/// 页面上所有标了 `data-fixed-bottom` 的贴底固定层（含当前不可见的）。
fn fixed_bottom_elements() -> Vec<web_sys::Element> {
  let Ok(list) = document().query_selector_all(FIXED_BOTTOM_SELECTOR) else {
    return Vec::new();
  };
  (0..list.length())
    .filter_map(|i| list.get(i))
    .filter_map(|el| el.dyn_into::<web_sys::Element>().ok())
    .collect()
}

/// 需要用 `ResizeObserver` 盯着的元素：吸顶导航 + 全部贴底固定层。
fn edge_elements() -> Vec<web_sys::Element> {
  let mut all = Vec::new();
  if let Ok(Some(nav)) = document().query_selector(NAV_SELECTOR) {
    all.push(nav);
  }
  all.extend(fixed_bottom_elements());
  all
}

#[component]
pub fn ScrollBar() -> impl IntoView {
  let host = NodeRef::<html::Div>::new();
  let thumb = NodeRef::<html::Div>::new();
  // 上下两条固定层的占位：滚动条只画在两者之间。缓存起来，避免每帧读它们的几何。
  let top_h = StoredValue::new(0.0);
  let bottom_h = StoredValue::new(0.0);
  // 拖动状态：按下时的指针 y 与当时的 `scrollY`。
  let dragging = StoredValue::new(false);
  let grab_y = StoredValue::new(0.0);
  let grab_scroll = StoredValue::new(0.0);
  // 上一帧算出的 thumb 高度与可移动行程：拖动时要用，避免每帧重算。
  let thumb_h = StoredValue::new(MIN_THUMB);
  let travel = StoredValue::new(1.0);

  // 量一次上下两条固定层的占位并写到 `top` / `bottom`：顶部导航在断点间会换高度
  // （桌面两行 / 手机一行），底部的 Tab 与操作栏高度也不同（操作栏在手机上多一行按钮）。
  //
  // 下面所有 `measure_edges` / `sync` 都可能在组件卸载之后才被执行（rAF、`set_timeout`、
  // 尚未摘除的窗口监听）：那时 `NodeRef` / `StoredValue` 已经随 Owner 释放，直接读会 panic，
  // 所以入口先判一次释放。
  let measure_edges = move || {
    if host.is_disposed() {
      return;
    }
    let top = document()
      .query_selector(NAV_SELECTOR)
      .ok()
      .flatten()
      .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
      .map(|el| f64::from(el.offset_height()))
      .unwrap_or(0.0);
    let bottom = bottom_inset();
    top_h.set_value(top);
    bottom_h.set_value(bottom);
    if let Some(host) = host.get_untracked() {
      let style = web_sys::HtmlElement::from(host).style();
      let _ = style.set_property("top", &format!("{top}px"));
      let _ = style.set_property("bottom", &format!("{bottom}px"));
    }
  };

  // 按当前 `scrollY` 摆 thumb；页面不够长就把整条隐藏。
  let sync = move || {
    if host.is_disposed() || thumb.is_disposed() {
      return;
    }
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
    // 可画高度 = 视口减去上下两条固定层；两者加起来超过视口时按 0 算（页面被遮严了）。
    let avail = (viewport - top_h.get_value() - bottom_h.get_value()).max(0.0);
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

  // 重量一次并摆 thumb。
  let remeasure = move || {
    measure_edges();
    sync();
  };

  // 固定层自己变高 / 变矮（语言包到位后按钮换行、安全区变化、晚挂载的横幅）不会触发
  // 滚动或 `resize`，只靠事件驱动会一直停在旧落位，所以另用 `ResizeObserver` 盯着它们。
  //
  // 回调里**只重量，不重新登记观察对象**：`observe` 每次都会立刻再触发一次回调，
  // 在回调里 `disconnect` + `observe` 会自激成逐帧循环。重新登记只放在下面的 `refresh`
  // （挂载 / 路由切换 / 换皮肤），那是固定层可能被换掉的时机。
  let observer_cb =
    Closure::<dyn FnMut(js_sys::Array)>::new(move |_entries: js_sys::Array| remeasure());
  let observer = ResizeObserver::new(observer_cb.as_ref().unchecked_ref::<js_sys::Function>()).ok();
  let watcher = StoredValue::new(SendWrapper::new(observer.clone()));
  let keep_observer = SendWrapper::new(Some((observer, observer_cb)));
  on_cleanup(move || {
    if let Some((Some(obs), _cb)) = keep_observer.take() {
      obs.disconnect();
    }
  });

  // 重新登记观察对象 + 重量：固定层随路由 / 皮肤被换掉后，旧节点不再需要盯，新节点要补上。
  let refresh = move || {
    if host.is_disposed() || watcher.is_disposed() {
      return;
    }
    watcher.with_value(|w| {
      if let Some(obs) = w.as_ref() {
        obs.disconnect();
        for el in edge_elements() {
          obs.observe(&el);
        }
      }
    });
    remeasure();
  };

  // `ResizeObserver` 只认得**已登记**的元素：晚挂载的横幅 / 路由带来的底栏，在下一次 `refresh`
  // 之前既没被观察、也不会触发任何事件。所以再盯一次 body 的子树增删，只在「固定层的集合真的变了」
  // 时才 `refresh`（每批变动只做一次 `querySelectorAll`，不会因页面里无关的 DOM 变化而重量）。
  // 观察的是 `childList` 而不是属性：`refresh` / `sync` 自己写样式不会反过来触发它。
  let mutation_cb = {
    let mut known = fixed_bottom_elements();
    Closure::<dyn FnMut(js_sys::Array)>::new(move |_records: js_sys::Array| {
      let now = fixed_bottom_elements();
      if now != known {
        known = now;
        refresh();
      }
    })
  };
  let mutation =
    web_sys::MutationObserver::new(mutation_cb.as_ref().unchecked_ref::<js_sys::Function>()).ok();
  if let (Some(obs), Some(body)) = (mutation.as_ref(), document().body()) {
    let init = web_sys::MutationObserverInit::new();
    init.set_child_list(true);
    init.set_subtree(true);
    let _ = obs.observe_with_options(&body, &init);
  }
  let keep_mutation = SendWrapper::new(Some((mutation, mutation_cb)));
  on_cleanup(move || {
    if let Some((Some(obs), _cb)) = keep_mutation.take() {
      obs.disconnect();
    }
  });

  // 句柄必须在卸载时摘除：`window_event_listener` 返回的句柄 drop 时**不会**自动移除监听，
  // 丢掉它等于监听器永久留在 `window` 上（切到现代风后每次滚动都会唤醒一个死组件）。
  let scroll_handle = window_event_listener(leptos::ev::scroll, move |_| {
    request_animation_frame(sync);
  });
  let resize_handle = window_event_listener(leptos::ev::resize, move |_| {
    request_animation_frame(remeasure);
  });
  let handles = SendWrapper::new((scroll_handle, resize_handle));
  on_cleanup(move || {
    let (scroll_handle, resize_handle) = handles.take();
    scroll_handle.remove();
    resize_handle.remove();
  });

  // 首次挂载：等字体 / 图标落位后再量（一次是挂载当帧，一次是稍后）。
  request_animation_frame(refresh);
  set_timeout(refresh, std::time::Duration::from_millis(600));

  // 路由切换 / 异步数据到位都会改文档高度、也会换掉贴底固定层（答题页的操作栏随页面挂载）：
  // 跟着重算（此时用户还没滚动）。
  let location = use_location();
  Effect::new(move |_| {
    let _ = location.pathname.get();
    request_animation_frame(refresh);
    set_timeout(refresh, std::time::Duration::from_millis(400));
  });

  // 切皮肤时两条固定层的高度都会变（两套顶栏 / 底栏的高度不同），必须重量一次：
  // 否则滚动条会按上一套皮肤的量测结果落位，错开几个像素压到新顶栏或新底栏上。
  Effect::new(move |_| {
    let _ = crate::kit::is_modern();
    request_animation_frame(refresh);
  });

  // 拖动：按下记起点，移动按行程比例换算成 `scrollY`，抬起结束。
  //
  // 按下时捕获指针：thumb 只有 10–12px 宽，拖动中指针稍微横向偏出就会脱靶，
  // 不捕获的话 `pointermove` 不再送达、`pointerleave` 还会把拖拽直接结束。
  // 捕获之后事件始终发给 thumb，松手（`pointerup` / `pointercancel`）才是唯一的结束条件。
  let start_drag = move |e: web_sys::PointerEvent| {
    dragging.set_value(true);
    grab_y.set_value(f64::from(e.client_y()));
    grab_scroll.set_value(window().scroll_y().unwrap_or(0.0));
    if let Some(target) = e
      .current_target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
      let _ = target.set_pointer_capture(e.pointer_id());
    }
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

  // 皮肤只差外观，类名分派放在属性闭包里（两套 CSS 各写一份，不共用 token）。
  let shell_class = move || {
    if crate::kit::is_modern() {
      "md-scrollbar"
    } else {
      "pxl-scrollbar"
    }
  };
  let thumb_class = move || {
    if crate::kit::is_modern() {
      "md-scrollbar-thumb"
    } else {
      "pxl-scrollbar-thumb"
    }
  };

  view! {
    // `aria-hidden`：它是原生滚动的视觉替身，读屏与键盘用户不需要它（键盘滚动照常）。
    // `pointer-events-none` 在外壳上：只有 thumb 接指针，别挡住右边缘的内容点击。
    <div node_ref=host aria-hidden="true" class=shell_class>
      <div
        node_ref=thumb
        class=thumb_class
        on:pointerdown=start_drag
        on:pointermove=drag
        on:pointerup=stop_drag
        on:pointercancel=stop_drag
      ></div>
    </div>
  }
}

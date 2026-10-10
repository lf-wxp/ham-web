//! 动效基础设施：滚动浮现、路由过渡与光斑跟随。
//!
//! 曲线、时长与关键帧全部定义在 `style/input.css` 的「动效语言」一节，这里只做三件
//! 浏览器必须参与的事：
//!
//! 1. **滚动浮现**：用 `IntersectionObserver` 给进入视口的元素打上 `data-reveal="in"`；
//! 2. **路由过渡**：切换路径时重放主内容区的入场动画，并在顶栏扫过一道信号光；
//! 3. **光斑跟随**：鼠标划过 `.spotlight` 卡片时，把指针位置写进 `--mx` / `--my`，
//!    由 CSS 用径向渐变画出跟手的柔光。
//!
//! # 降级策略
//!
//! 滚动浮现的初始态是「透明 + 位移」，一旦 JS 没跑起来就会把内容永久藏住 —— 这是最
//! 不能接受的失败模式。因此整套浮现有两级开关：
//!
//! - 不支持 `IntersectionObserver`，或用户开了「减少动态效果」→ 不给 `<html>` 加
//!   `.js-motion`，CSS 里依赖该类名的预隐藏规则整体失效；
//! - 即便加了 `.js-motion`，元素也只在**成功注册进观察器之后**才写入 `data-reveal`。
//!
//! 两条都满足时内容才可能处于隐藏态，此时 IntersectionObserver 一定会把它唤醒。
//! 结果是：老浏览器 / 无障碍偏好下动效「没有」，而不是内容「卡住」。

use std::cell::RefCell;

use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// `<html>` 上的标记类：存在才启用 JS 驱动的入场动效。
const MOTION_READY: &str = "js-motion";

/// 路由内容容器的 `id`，[`RouteTransition`] 靠它重放入场动画。
pub const ROUTE_CONTENT_ID: &str = "route-content";

/// 路由内容容器的入场类名。
const PAGE_ENTER: &str = "motion-page";

/// 顶栏扫光条的类名（播放完自动停在透明态）。
const ROUTE_SWEEP: &str = "route-sweep";

/// 错峰步长与档位上限：同批进入视口的第 n 个元素延迟 `n * STAGGER_MS` 入场，
/// 最多累计 6 档（360ms），避免长列表末尾「排队等太久」。
///
/// 写在 Rust 侧而不是 CSS 变量里，是因为只有观察器知道元素是哪一批进来的。
const STAGGER_MS: u32 = 60;
const STAGGER_MAX: u32 = 6;

type RevealCallback = Closure<dyn Fn(js_sys::Array, web_sys::IntersectionObserver)>;

thread_local! {
  /// 全站共用一个观察器：180+ 页面逐个 new 既浪费内存，也让错峰序号失去意义。
  static REVEAL: RefCell<Option<SendWrapper<web_sys::IntersectionObserver>>> =
    const { RefCell::new(None) };
  /// 回调闭包必须活得比观察器久，否则 `IntersectionObserver` 触发时就悬垂了。
  static CALLBACK: RefCell<Option<SendWrapper<RevealCallback>>> = const { RefCell::new(None) };
}

/// 光斑跟随宿主的类名（对应 `style/input.css` 的 `.spotlight`）。
const SPOTLIGHT_HOST: &str = ".spotlight";

/// 初始化动效环境。应在 [`crate::app::App`] 里最先调用。
pub fn provide_motion() {
  if !motion_allowed() {
    return;
  }
  install_spotlight();
  let Some(observer) = create_observer() else {
    return;
  };
  // 先拿到观察器再挂标记：拿不到就保持「无动效但内容可见」的基线。
  set_motion_ready(true);
  REVEAL.with(|c| *c.borrow_mut() = Some(SendWrapper::new(observer)));
  on_cleanup(|| {
    set_motion_ready(false);
    REVEAL.with(|c| *c.borrow_mut() = None);
    CALLBACK.with(|c| *c.borrow_mut() = None);
  });
}

/// 挂一个全局的指针监听，把鼠标在 `.spotlight` 宿主内的位置写进 `--mx` / `--my`。
///
/// 用事件委托（一个监听管全站）而不是给每张卡片各挂一个：卡片会随路由反复创建销毁，
/// 逐个挂 / 摘既容易漏清理，也让监听数量随页面内容膨胀。非 `.spotlight` 区域的移动
/// 只会走一次 `closest` 就返回，开销可以忽略。
///
/// 只响应鼠标：触屏与手写笔没有「悬停」，CSS 侧也用 `@media (hover: hover)` 屏蔽了
/// 光斑，这里提前返回是为了省掉无意义的样式写入。
fn install_spotlight() {
  let handle = window_event_listener(ev::pointermove, move |e| {
    if e.pointer_type() != "mouse" {
      return;
    }
    let Some(host) = e
      .target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
      .and_then(|el| el.closest(SPOTLIGHT_HOST).ok().flatten())
      .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
    else {
      return;
    };
    let rect = host.get_bounding_client_rect();
    // 显式 `HtmlElement::style`：理由同 `set_reveal_delay`。
    let style = web_sys::HtmlElement::style(&host);
    let _ = style.set_property(
      "--mx",
      &format!("{:.0}px", f64::from(e.client_x()) - rect.left()),
    );
    let _ = style.set_property(
      "--my",
      &format!("{:.0}px", f64::from(e.client_y()) - rect.top()),
    );
  });
  let handle = SendWrapper::new(Some(handle));
  on_cleanup(move || {
    if let Some(h) = handle.take() {
      h.remove();
    }
  });
}

/// 是否允许播放动效（用户未开启「减少动态效果」）。
///
/// 供 JS 驱动的非 CSS 动效（如 [`crate::ui::Stat` 的数字滚动）在启动前判断，
/// 避免在「减少动态效果」下仍跑计时器动画。
pub(crate) fn motion_allowed() -> bool {
  !crate::util::window()
    .match_media("(prefers-reduced-motion: reduce)")
    .ok()
    .flatten()
    .is_some_and(|m| m.matches())
}

fn set_motion_ready(on: bool) {
  if let Some(root) = crate::util::document().document_element() {
    let list = root.class_list();
    let _ = if on {
      list.add_1(MOTION_READY)
    } else {
      list.remove_1(MOTION_READY)
    };
  }
}

/// 创建共享的浮现观察器。
fn create_observer() -> Option<web_sys::IntersectionObserver> {
  let supported = js_sys::Reflect::get(&crate::util::window(), &"IntersectionObserver".into())
    .map(|v| !v.is_undefined() && !v.is_null())
    .unwrap_or(false);
  if !supported {
    return None;
  }

  let cb = RevealCallback::new(
    |entries: js_sys::Array, obs: web_sys::IntersectionObserver| {
      // 同一批进入视口的元素按文档顺序依次错峰，形成自上而下的入场节奏。
      let mut step = 0u32;
      for entry in entries.iter() {
        let Ok(entry) = entry.dyn_into::<web_sys::IntersectionObserverEntry>() else {
          continue;
        };
        if !entry.is_intersecting() {
          continue;
        }
        let Ok(target) = entry.target().dyn_into::<web_sys::Element>() else {
          continue;
        };
        let delay = step.min(STAGGER_MAX) * STAGGER_MS;
        set_reveal_delay(&target, delay);
        let _ = target.set_attribute("data-reveal", "in");
        // 一次性动效：播完即注销，长页面滚动时不再产生回调。
        obs.unobserve(&target);
        step += 1;
      }
    },
  );

  let init = web_sys::IntersectionObserverInit::new();
  // 底部留 5%：元素刚露头就开始入场，滚到底时最后一行不会「等一下才出现」。
  init.set_root_margin("0px 0px -5% 0px");
  init.set_threshold(&wasm_bindgen::JsValue::from_f64(0.08));
  let observer =
    web_sys::IntersectionObserver::new_with_options(cb.as_ref().unchecked_ref(), &init).ok()?;
  CALLBACK.with(|c| *c.borrow_mut() = Some(SendWrapper::new(cb)));
  Some(observer)
}

/// 让单个元素在滚动进入视口时浮现。
///
/// 只有成功注册进观察器才写入 `data-reveal`，否则元素会停在透明态。
pub fn reveal_on_scroll(el: &web_sys::Element, delay_ms: u32) {
  let registered = REVEAL.with(|c| match c.borrow().as_ref() {
    Some(obs) => {
      obs.observe(el);
      true
    }
    None => false,
  });
  if !registered {
    return;
  }
  set_reveal_delay(el, delay_ms);
  let _ = el.set_attribute("data-reveal", "");
}

/// 写入错峰延迟。
///
/// 显式指定 `web_sys::HtmlElement::style`：`web_sys::Element` 上没有 `style()`，
/// 而 leptos 的 `ElementExt` 扩展特质也提供了一个 `style(_)`，两者同名会抢解析。
fn set_reveal_delay(el: &web_sys::Element, delay_ms: u32) {
  if delay_ms == 0 {
    return;
  }
  let Some(el) = el.dyn_ref::<web_sys::HtmlElement>() else {
    return;
  };
  let _ = web_sys::HtmlElement::style(el).set_property("--reveal-delay", &format!("{delay_ms}ms"));
}

/// 让容器的**直接子元素**逐个浮现（页面主体、卡片列表这类纵向排布）。
///
/// 只取 `:scope > *`，不递归：嵌套层级各自决定自己的节奏，避免深层元素被一次性点亮。
pub fn reveal_children(parent: &web_sys::Element) {
  let Ok(nodes) = parent.query_selector_all(":scope > *") else {
    return;
  };
  for i in 0..nodes.length() {
    let Some(node) = nodes.item(i) else { continue };
    let Some(el) = node.dyn_ref::<web_sys::Element>() else {
      continue;
    };
    reveal_on_scroll(el, 0);
  }
}

/// 路由过渡：切换路径时重放主内容区入场动画，并在顶栏扫过一道信号光。
///
/// 必须放在 `<Router>` 内（要读 `use_location`），且排在 `app::MainContent`
/// 之后，这样首次运行时容器已经挂载。
#[component]
pub fn RouteTransition() -> impl IntoView {
  let location = use_location();
  // 每次递增就换掉扫光条这个 DOM 节点 —— 换节点即重播动画，比手动摘加类名可靠。
  let beat = RwSignal::new(0u32);

  Effect::new(move |_| {
    let _ = location.pathname.get();
    beat.update(|n| *n += 1);
    let Some(el) = crate::util::document().get_element_by_id(ROUTE_CONTENT_ID) else {
      return;
    };
    // 重启 CSS 动画的三步：摘类 → 强制回流 → 加类。少了中间的强制回流，浏览器会把
    // 两次改动合并成「类名没变」，动画不会重播。
    let list = el.class_list();
    let _ = list.remove_1(PAGE_ENTER);
    let _ = el.get_bounding_client_rect();
    let _ = list.add_1(PAGE_ENTER);
  });

  view! {
    {move || {
      beat.get();
      view! { <div aria-hidden="true" class=ROUTE_SWEEP></div> }
    }}
  }
}

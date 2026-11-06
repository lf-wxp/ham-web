//! 动效基础设施：滚动浮现与路由过渡。
//!
//! 缓动、时长与关键帧全部定义在 `style/pixel/motion.css`（逐帧 `steps()`），这里只做两件
//! 浏览器必须参与的事：
//!
//! 1. **滚动浮现**：用 `IntersectionObserver` 给进入视口的元素打上 `data-reveal="in"`；
//! 2. **路由过渡**：切换路径时重放主内容区的像素溶解入场，并在顶栏走一格一格的读条。
//!
//! # 降级策略
//!
//! 滚动浮现的初始态是「透明 + 位移」，一旦 JS 没跑起来就会把内容永久藏住 —— 这是最
//! 不能接受的失败模式。因此整套浮现有两级开关：
//!
//! - 不支持 `IntersectionObserver`，或动效被关（系统「减少动态效果」，或设置里的
//!   「像素动效：关」，即 `<html data-pixel-motion="off">`）→ 不给 `<html>` 加
//!   `.js-motion`，CSS 里依赖该类名的预隐藏规则整体失效；
//! - 即便加了 `.js-motion`，元素也只在**成功注册进观察器之后**才写入 `data-reveal`。
//!
//! 两条都满足时内容才可能处于隐藏态，此时 IntersectionObserver 一定会把它唤醒。
//! 结果是：老浏览器 / 无障碍偏好下动效「没有」，而不是内容「卡住」。

use std::cell::RefCell;
use std::time::Duration;

use leptos::prelude::*;
use leptos_router::hooks::use_location;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

/// 数字滚动：把目标值包成「逐步逼近」的展示值。**全站唯一一份。**
///
/// 以前两份：`ui::Stat`（立方缓出、600ms / 12 步）与 `kit_modern::motion::use_count_up`
/// （归一化 expo-out、760ms / 16 步）。骨架完全一样（代际号防串台、`try_get_value` 防卸载后访问），
/// 只有曲线不同 —— 而曲线才是出过 bug 的地方（未归一化的 expo-out 末步差 1，XP 5000 永久停在 4995）。
/// 现在曲线只在 `core::count_up`（有单测守着「末步恒等于目标」），这里只负责排定时器。
///
/// - `from_zero`：首次挂载从 0 起跳（数据条这类「报数滚上来」的场景）；`false` 则直接显示当前值，
///   之后每次变化才滚动（计数 +1 这类反馈）。动效被关时两者都直接落位。
/// - 时长 / 步数按风格取：现代风 760ms / 16 步（更缓的收尾），像素风 600ms / 12 步。
#[must_use]
pub fn use_count_up(value: Signal<usize>, from_zero: bool) -> Signal<usize> {
  let (total_ms, steps): (u64, u32) = if crate::kit::is_modern_at_mount() {
    (760, 16)
  } else {
    (600, 12)
  };
  let start = if from_zero && motion_allowed() {
    0
  } else {
    value.get_untracked()
  };
  let display = RwSignal::new(start);
  let generation = StoredValue::new(0u32);

  Effect::new(move |_| {
    let target = value.get();
    let from = display.get_untracked();
    generation.update_value(|g| *g += 1);
    let epoch = generation.get_value();
    if from == target {
      return;
    }
    if !motion_allowed() {
      display.set(target);
      return;
    }
    for i in 1..=steps {
      let step = ham_web_core::count_up::count_step(from, target, i, steps);
      set_timeout(
        move || {
          if generation.try_get_value() == Some(epoch) {
            display.set(step);
          }
        },
        Duration::from_millis(total_ms * u64::from(i) / u64::from(steps)),
      );
    }
  });

  display.into()
}

/// `<html>` 上的标记类：存在才启用 JS 驱动的入场动效。
const MOTION_READY: &str = "js-motion";

/// 路由内容容器的 `id`，[`RouteTransition`] 靠它重放入场动画。
pub const ROUTE_CONTENT_ID: &str = "route-content";

/// 路由内容容器的入场类名。
const PAGE_ENTER: &str = "motion-page";

/// 顶栏读条的类名（播放完自动停在透明态）。
const ROUTE_SWEEP: &str = "route-sweep";

/// 错峰步长与档位上限：同批进入视口的第 n 个元素延迟 `n * STAGGER_MS` 入场，
/// 最多累计 6 档（360ms），避免长列表末尾「排队等太久」。
///
/// 写在 Rust 侧而不是 CSS 变量里，是因为只有观察器知道元素是哪一批进来的。
const STAGGER_MS: u32 = 60;
const STAGGER_MAX: u32 = 6;

/// 现代风的错峰：更短的步长、更少的档位（最多晚 120ms）。
/// 平滑曲线下 60ms × 6 档的排队一眼就能看出来，读起来像「滚到了还要等一下」。
const MODERN_STAGGER_MS: u32 = 30;
const MODERN_STAGGER_MAX: u32 = 4;

type RevealCallback = Closure<dyn Fn(js_sys::Array, web_sys::IntersectionObserver)>;

thread_local! {
  /// 全站共用一个观察器：180+ 页面逐个 new 既浪费内存，也让错峰序号失去意义。
  static REVEAL: RefCell<Option<SendWrapper<web_sys::IntersectionObserver>>> =
    const { RefCell::new(None) };
  /// 回调闭包必须活得比观察器久，否则 `IntersectionObserver` 触发时就悬垂了。
  static CALLBACK: RefCell<Option<SendWrapper<RevealCallback>>> = const { RefCell::new(None) };
}

/// 初始化动效环境。应在 [`crate::app::App`] 里最先调用。
pub fn provide_motion() {
  if !motion_allowed() {
    return;
  }
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

/// 是否允许播放动效：系统没开「减少动态效果」，且设置里的「像素动效」没被关掉。
///
/// 供 JS 驱动的非 CSS 动效（如 [`crate::ui::Stat`] 的数字滚动）在启动前判断，
/// 避免在动效被关时仍跑计时器动画。
/// 系统层面是否允许动效（`prefers-reduced-motion`），与设置里的开关无关。
pub(crate) fn system_allows_motion() -> bool {
  !crate::util::window()
    .match_media("(prefers-reduced-motion: reduce)")
    .ok()
    .flatten()
    .is_some_and(|m| m.matches())
}

/// 动效是否允许（系统偏好 + 设置开关）。
///
/// **别在事件回调里用**：它会同步读一次 `localStorage`。高频路径（`pointerdown` /
/// 滚动 / 逐帧）请用 [`system_allows_motion`] 加 `DisplayPrefs::pixel_motion_now`，
/// 后者读的是已经加载好的信号。
pub(crate) fn motion_allowed() -> bool {
  system_allows_motion() && crate::theme::pixel_motion_enabled()
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
      let modern = crate::theme::DisplayPrefs::style_is_modern_dom();
      let (stagger, stagger_max) = if modern {
        (MODERN_STAGGER_MS, MODERN_STAGGER_MAX)
      } else {
        (STAGGER_MS, STAGGER_MAX)
      };
      // 视口高度每批只读一次（`innerHeight` 不触发布局）。
      let viewport_h = viewport_height();
      // 同一批进入视口的元素按文档顺序依次错峰，形成自上而下的入场节奏。
      let mut step = 0u32;
      for entry in entries.iter() {
        let Ok(entry) = entry.dyn_into::<web_sys::IntersectionObserverEntry>() else {
          continue;
        };
        let Ok(target) = entry.target().dyn_into::<web_sys::Element>() else {
          continue;
        };
        // 还没有 `data-reveal` 的是现代风登记的候选（见 [`reveal_below_fold`]）：这是它的第一次通知，
        // 观察器顺带给出了几何信息，不用再调 `getBoundingClientRect` 强制布局。
        if target.get_attribute("data-reveal").is_none() {
          let rect = entry.bounding_client_rect();
          let on_screen = rect.top() < viewport_h;
          // 比视口还高的块阈值永远达不到（「自身 8% 可见」）：藏起来就再也出不来。
          let too_tall = rect.height() > viewport_h * 0.92;
          if on_screen || too_tall {
            // 已经在屏幕上（或在屏幕上方）的直接算「已入场」：它们在登记之前就画出来了，
            // 再藏起来重播一次，用户看到的就是「出现 → 消失 → 再浮上来」。
            let _ = target.set_attribute("data-reveal", "in");
            obs.unobserve(&target);
          } else {
            // 折叠线以下：先藏起来（无过渡，见 `kit.css`），等滚到了再浮现。
            let _ = target.set_attribute("data-reveal", "");
          }
          continue;
        }
        if !entry.is_intersecting() {
          continue;
        }
        let delay = step.min(stagger_max) * stagger;
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
/// 现代风走 [`reveal_below_fold`]：只有登记时在折叠线以下的元素才会被藏起来。
pub fn reveal_on_scroll(el: &web_sys::Element, delay_ms: u32) {
  if crate::theme::DisplayPrefs::style_is_modern_dom() {
    reveal_below_fold(el);
    return;
  }
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

/// 现代风的登记：先只交给观察器、不写 `data-reveal`，由第一次通知决定藏不藏。
///
/// 像素风的做法是「先藏起来、等观察器说它在视口里再放出来」：那套是 4 格跳变，
/// 第一格要停 60ms，藏 / 放之间的一两帧看不出来。现代风是平滑曲线，藏下去的那一下
/// 立刻可见 —— 已经画在屏幕上的内容会「闪一下再浮上来」。所以这里反过来：
/// 观察器的第一次通知带着元素的位置，在屏幕上的直接记为已入场，只有折叠线以下的才藏，
/// 等滚到了再浮现（回调里的分支见 [`create_observer`]）。
///
/// 顺带的好处：不用再逐个 `getBoundingClientRect` 量高度（每量一个都是一次强制同步布局），
/// 「比视口还高、永远触发不了阈值」的判断也改用通知里的几何信息。
fn reveal_below_fold(el: &web_sys::Element) {
  if el.get_attribute("data-reveal").is_some() {
    return;
  }
  REVEAL.with(|c| {
    if let Some(obs) = c.borrow().as_ref() {
      // 同一元素重复 `observe` 是空操作（规范如此），三趟登记扫到同一批候选也不会排两次队。
      obs.observe(el);
    }
  });
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
  // 现代风：高度判断交给观察器的第一次通知（见 [`reveal_below_fold`]），这里不量。
  let modern = crate::theme::DisplayPrefs::style_is_modern_dom();
  let viewport_h = viewport_height();
  for i in 0..nodes.length() {
    let Some(node) = nodes.item(i) else { continue };
    let Some(el) = node.dyn_ref::<web_sys::Element>() else {
      continue;
    };
    // 比视口高的子块不登记：观察器阈值是「自身 8% 可见」，一个整屏都装不下的块
    // 永远不会触发 —— 登记了就是把它永久藏起来（工具页的 46 个计算器踩过一次）。
    if !modern && el.get_bounding_client_rect().height() > viewport_h * 0.92 {
      continue;
    }
    reveal_on_scroll(el, 0);
  }
}

/// 视口高度（拿不到时按 800 兜底：常见笔记本视口，宁可少登记也不要多登记）。
fn viewport_height() -> f64 {
  crate::util::window()
    .inner_height()
    .ok()
    .and_then(|v| v.as_f64())
    .unwrap_or(800.0)
}

/// 现代风页面的入场编排：路由切换后把页面主体与常见列表项逐个登记进浮现观察器。
///
/// 像素风靠 `PageContainer` 在组件里登记（它拿得到 `NodeRef`）；现代风页面是
/// 一层 `md-page` 平铺，逐页加 `NodeRef` 只会让每个页面都多一行样板，所以统一在这里
/// 按类名扫一遍。选择器只写现代风的 `md-*`，像素页上一条也匹配不到。
///
/// 已经带 `data-reveal` 的元素跳过：`.md-page > *` 与 `.md-panel` 会重叠，重复登记
/// 会让同一元素在观察器里排队两次（错峰序号被吃掉，后面的元素集体晚一拍）。
///
/// 不在这里量高度：「比视口还高」与「是否已在屏幕上」都由观察器的第一次通知判断
/// （见 [`reveal_below_fold`]），登记本身不触发布局。查询限定在 `#route-content` 内，
/// 弹层与导航不该被登记。
pub fn reveal_modern_page() {
  // `.md-page > *` 管迁移过的现代页；桥接页的内容块由 `PageContainer` 在挂载时自己登记
  // （见 `components/common/page_container.rs`）—— 两条合起来才是「所有页面同一套换页
  // 手感」，只做前者的话桥接页只有整页淡入、现代页却是「淡入 + 内容上浮」，一眼能看出差别。
  const TARGETS: [&str; 6] = [
    ".md-page > *",
    ".md-q-row",
    ".md-term",
    ".md-tool-link",
    ".md-log-row",
    ".md-panel",
  ];
  let Some(scope) = crate::util::document().get_element_by_id(ROUTE_CONTENT_ID) else {
    return;
  };
  for sel in TARGETS {
    let Ok(nodes) = scope.query_selector_all(sel) else {
      continue;
    };
    for i in 0..nodes.length() {
      let Some(node) = nodes.item(i) else { continue };
      let Some(el) = node.dyn_ref::<web_sys::Element>() else {
        continue;
      };
      reveal_below_fold(el);
    }
  }
}

/// 路由过渡：切换路径时重放主内容区的入场，并落位交互签名与换页方向。
///
/// 必须放在 `<Router>` 内（要读 `use_location`），且排在 `app::MainContent`
/// 之后，这样首次运行时容器已经挂载。
///
/// 现代风的换页由 `data-nav-dir` 分流（前进自下、后退自上、平级只淡入）；
/// 像素风继续走 4px 网点溶解，不受影响（样式层的分支都带 `[data-ui-style="modern"]`）。
#[component]
pub fn RouteTransition() -> impl IntoView {
  let location = use_location();
  // 每次递增就换掉读条这个 DOM 节点 —— 换节点即重播动画，比手动摘加类名可靠。
  let beat = RwSignal::new(0u32);
  // 上一次的路径：换页方向要比对「从哪来」，首屏没有上一页。
  let previous = StoredValue::new(Option::<String>::None);

  Effect::new(move |_| {
    let path = location.pathname.get();
    let from = previous.get_value();
    crate::kit_modern::interaction::apply_route_signature(&path, from.as_deref());
    previous.set_value(Some(path));
    beat.update(|n| *n += 1);
    // 现代风的入场登记要等 DOM 写完之后再做：Leptos 的更新是异步的，
    // 这里同步查 `.md-page` 只会拿到上一页的节点。
    //
    // 三趟而不是一趟：题库浏览 / 术语表的列表要等本地 JSON 取回才渲染，
    // 只扫首帧的话整批 `.md-q-row` 会漏掉（于是它们干脆不做入场，看起来像「有的页有动效、
    // 有的页没有」）。后两趟成本很低：`reveal_modern_page` 对已登记的元素直接跳过。
    {
      let cb = Closure::once_into_js(move || {
        let again = Closure::once_into_js(reveal_modern_page);
        if let Some(w) = web_sys::window() {
          let _ = w.request_animation_frame(again.unchecked_ref());
        }
      });
      if let Some(w) = web_sys::window() {
        let _ = w.request_animation_frame(cb.unchecked_ref());
      }
      set_timeout(reveal_modern_page, std::time::Duration::from_millis(220));
      set_timeout(reveal_modern_page, std::time::Duration::from_millis(640));
    }
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

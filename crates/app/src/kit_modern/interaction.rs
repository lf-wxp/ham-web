//! 现代风的交互编排：路由签名落位 + 按压「声呐点」。
//!
//! 两件事都不该由页面各自实现：
//!
//! 1. **签名落位**：`#route-content` 上的 `data-sig` / `data-nav-dir` 由这里统一写，
//!    这样 180 条路由（含还没迁移的桥接页）都拿到同一份动作族，样式层只按属性分支；
//! 2. **按压反馈**：用一次事件委托给「所有可点元素」加反馈，而不是给每个组件加一行
//!    `on:pointerdown` —— 后者会让每新增一个控件就漏一次反馈。
//!
//! 降级口径与动效层一致：动效被关（系统偏好 / 设置）时一个节点都不注入，
//! 元素停在浏览器原生的 `:active` 反馈上，不会出现「点下去没反应」或「卡着不消失」。

use std::cell::Cell;

use ham_web_core::interaction::{NavDirection, nav_direction, signature_for};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::motion::ROUTE_CONTENT_ID;

/// 涟漪节点类名（注入后 560ms 自删）。
const PING_CLASS: &str = "md-ping";
/// 按压期间挂在宿主元素上的类名：建立定位上下文并裁掉溢出的涟漪。
const PRESS_CLASS: &str = "md-pressing";
/// 整块卡片用的是「边缘声呐环」而不是点状涟漪 —— 卡片内部常有绝对定位的装饰，
/// 注入子节点容易把它们挤掉。
///
/// 类名**不能**叫 `md-ring`：`kit.css` 里曾有一条同名的首页「等级环」规则（132×132 的橙色圆环），
/// 按压时临时挂上这个类，选项行 / 题行 / 答题卡格子就会在 560ms 内缩成一个 132px 的圆，
/// `mouseup` 落到别处，`click` 被丢掉 —— 表现为「点选项偶尔没反应」。
/// 按压反馈的类名必须全局唯一，别和任何组件类共名。
const RING_CLASS: &str = "md-sonar";
/// 与 `interaction.css` 里的动画时长保持一致；改一处要同时改另一处。
const PING_MS: u32 = 560;

/// 走「点状涟漪」的控件：形状规整、内部没有装饰层。
///
/// 按钮类带 `:not(:disabled)`：禁用的控件按下去不该有任何反馈（它不会响应，涟漪等于骗人去点）。
/// 分页按钮与图标按钮同属「小而规整的按钮」，原先漏在列表外，是点了没有反馈的唯一一类。
const RIPPLE_TARGETS: &str = ".md-btn:not(:disabled), .md-chip:not(:disabled), \
                              .md-icon-btn:not(:disabled), .md-page-btn:not(:disabled), \
                              .md-dock-item, .md-nav-link, .md-link, .md-link-row";

/// 走「边缘声呐环」的整块表面：**只有真的可点**的才给反馈。
///
/// 术语卡与内容面板刻意不在列：它们不可点击，闪一圈环等于骗人去点
/// （同一条理由见 `kit.css` 里「术语卡悬停不上抬」的注释）。
const RING_TARGETS: &str = ".md-opt, .md-q-row, .md-tool-link, .md-feature, .md-mod, \
                            .md-bookmark, .md-cell";

/// 宿主上记「最新一次按压序号」的属性：连点时让旧定时器认出自己已经过期。
const PRESS_SEQ_ATTR: &str = "data-press-seq";

thread_local! {
  /// 按压序号（全局递增即可，只用来比较「是不是最新一次」）。
  static PRESS_SEQ: Cell<u32> = const { Cell::new(0) };
  /// 全局按压监听只装一次：根组件重挂（热更新 / 测试里反复挂载）不能叠出多份监听。
  static PRESS_INSTALLED: Cell<bool> = const { Cell::new(false) };
  /// 上一次 `popstate` 的时间戳。外壳与路由过渡都要读它，所以不设「谁消费谁清空」——
  /// 两处各读各的，用一个时间窗口判断这次导航是不是历史回放。
  static LAST_POP_MS: Cell<f64> = const { Cell::new(0.0) };
}

/// 窗口内发生的导航算「历史回放」：`popstate` 先于路由更新触发，两者之间不会隔很久。
const POP_WINDOW_MS: f64 = 400.0;

/// 记录一次历史回放（由外壳的 `popstate` 监听调用）。
pub fn note_history_navigation() {
  LAST_POP_MS.with(|cell| cell.set(js_sys::Date::now()));
}

/// 这次路由变化是否来自浏览器后退 / 前进。
fn is_history_navigation() -> bool {
  LAST_POP_MS.with(|cell| js_sys::Date::now() - cell.get() < POP_WINDOW_MS)
}

/// 把路由签名与换页方向写到 `#route-content`，返回这次的方向。
///
/// `previous` 为 `None`（首屏）时写 `data-nav-dir="initial"`：首屏已经有整页入场编排，
/// 再叠一条换页动画会变成「淡入两次」。
///
/// 签名对两套风格都写：像素风的 `data-sig` 规则全在 `html[data-ui-style="modern"]` 门禁之内，
/// 写了不会有副作用，而桥接页（像素 DOM 跑在现代风外壳里）也必须拿到同一份动作族。
pub fn apply_route_signature(path: &str, previous: Option<&str>) -> NavDirection {
  let direction = nav_direction(previous, path, is_history_navigation());
  let signature = signature_for(path);
  let Some(el) = crate::util::document().get_element_by_id(ROUTE_CONTENT_ID) else {
    return direction;
  };
  let _ = el.set_attribute("data-sig", signature.attr());
  let _ = el.set_attribute("data-nav-dir", direction.attr());
  direction
}

/// 安装全局按压反馈。在根组件里调用一次。
///
/// 监听**无条件装**：所有判断都在回调里现读。曾经在挂载时先查一次动效开关，结果是
/// 「启动时关着动效 → 后来在设置里打开 → 要刷新才有按压反馈」，而回调里又写着同一个
/// 判断 —— 两处口径互相矛盾，只有一个是对的。
pub fn install_press_feedback() {
  if PRESS_INSTALLED.with(|flag| flag.replace(true)) {
    return;
  }
  let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
    return;
  };
  // 偏好在这里取一次：事件回调跑在反应式所有者之外，在里面 `expect_context` 会直接 panic，
  // 而且那一下 panic 会把同一个指针序列上的 click / 导航一起带走。`DisplayPrefs` 是 `Copy`，
  // 捕获进闭包后仍然读得到「切换风格后的当前值」。
  let prefs = crate::theme::use_display_prefs();
  let on_press = Closure::<dyn Fn(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
    // 像素风不播这套反馈（它的按压是 `steps()` 的 2px 下沉）。每次事件现读：`DisplayPrefs`
    // 是 `Copy`，切到现代风后不必重新装监听，下一次按下就生效。
    // 这里不能用 `motion_allowed()` —— 它每次同步读 localStorage，而 `pointerdown` 是
    // 高频路径；信号里已经是同一个偏好的活值。
    if !prefs.is_modern_at_mount()
      || !crate::motion::system_allows_motion()
      || !prefs.pixel_motion_now()
    {
      return;
    }
    if ev.button() != 0 {
      return;
    }
    let Some(target) = ev
      .target()
      .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    else {
      return;
    };
    if first_match(&target, RIPPLE_TARGETS).is_some() {
      spawn_ping(&target, &ev);
    } else if let Some(surface) = first_match(&target, RING_TARGETS) {
      spawn_ring(&surface);
    }
  });
  let _ = doc.add_event_listener_with_callback("pointerdown", on_press.as_ref().unchecked_ref());
  // 闭包必须活得和文档一样久：注册进 DOM 的监听不会自己悬垂，但 Rust 侧一旦 drop
  // 回调指针就失效了。
  on_press.forget();
}

/// 在目标自身及其祖先里找第一个命中 `selector` 的元素。
fn first_match(target: &web_sys::Element, selector: &str) -> Option<web_sys::Element> {
  target.closest(selector).ok().flatten()
}

/// 点状涟漪：坐标写进 `--md-press-x/y`，节点由 CSS 动画播完自然消失。
fn spawn_ping(target: &web_sys::Element, ev: &web_sys::PointerEvent) {
  let Some(host) = first_match(target, RIPPLE_TARGETS) else {
    return;
  };
  let rect = host.get_bounding_client_rect();
  if rect.width() <= 0.0 || rect.height() <= 0.0 {
    return;
  }
  let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
    return;
  };
  let Ok(span) = doc.create_element("span") else {
    return;
  };
  let _ = span.set_attribute("aria-hidden", "true");
  span.set_class_name(PING_CLASS);
  // 上一次的涟漪还没消失就再点：先清掉，避免叠成一团。
  // 选择器要带 `.`：`PING_CLASS` 是类名，不带的话查的是 `<md-ping>` 标签，永远查不到。
  if let Ok(Some(previous)) = host.query_selector(&format!(".{PING_CLASS}")) {
    previous.remove();
  }
  let x = f64::from(ev.client_x()) - rect.left();
  let y = f64::from(ev.client_y()) - rect.top();
  let _ = span.set_attribute(
    "style",
    &format!("--md-press-x:{x:.0}px;--md-press-y:{y:.0}px"),
  );
  let _ = host.class_list().add_1(PRESS_CLASS);
  if host.append_child(&span).is_err() {
    let _ = host.class_list().remove_1(PRESS_CLASS);
    return;
  }
  let seq = stamp(&host);
  set_timeout(
    move || {
      span.remove();
      // 连点时第一次留下的定时器不能摘掉第二次还在播放的按压态：只认最新一次。
      if is_latest(&host, seq) {
        let _ = host.class_list().remove_1(PRESS_CLASS);
      }
    },
    std::time::Duration::from_millis(u64::from(PING_MS)),
  );
}

/// 边缘声呐环：整块卡片不适合插子节点，改成描边外扩一次。
fn spawn_ring(host: &web_sys::Element) {
  let list = host.class_list();
  let _ = list.remove_1(RING_CLASS);
  // 摘掉再加一次：不重启动画的话，连点时第二下不会有反应。
  let _ = host.get_bounding_client_rect();
  let _ = list.add_1(RING_CLASS);
  let seq = stamp(host);
  let host = host.clone();
  set_timeout(
    move || {
      if is_latest(&host, seq) {
        let _ = host.class_list().remove_1(RING_CLASS);
      }
    },
    std::time::Duration::from_millis(u64::from(PING_MS)),
  );
}

/// 给宿主打上本次按压的序号并返回它：定时器回来时用 [`is_latest`] 判断自己是不是最新一次。
fn stamp(host: &web_sys::Element) -> u32 {
  let seq = PRESS_SEQ.with(|cell| {
    let next = cell.get().wrapping_add(1);
    cell.set(next);
    next
  });
  let _ = host.set_attribute(PRESS_SEQ_ATTR, &seq.to_string());
  seq
}

fn is_latest(host: &web_sys::Element, seq: u32) -> bool {
  host.get_attribute(PRESS_SEQ_ATTR).as_deref() == Some(seq.to_string().as_str())
}

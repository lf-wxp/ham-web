//! 弹层（popover）基座：全站下拉 / 选择面板的**唯一**外观与开合来源。
//!
//! [`crate::ui::Select`] 是下拉的唯一实现（键盘、焦点、ARIA、箭头都在那里），数据式的
//! [`crate::ui::SimpleSelect`] 只是它的一层封装；[`crate::ui::DatePicker`]、
//! [`crate::ui::TimePicker`] 复用这里的类名与开合行为 ——
//! 于是「弹出的选择框和语言切换一致」不是靠人工对齐，而是结构上只有一处定义。
//!
//! 本模块只管外观（类名、选项行、遮罩、焦点移出）；开合行为各有一个来源：
//! 延迟卸载与 `data-state` / `inert` / `aria-hidden` 在 [`crate::presence`]，Esc 在 [`crate::focus`]。
//!
//! 使用方式（组件的 `view!` 里）：
//!
//! ```text
//! let open = RwSignal::new(false);
//! let mounted = crate::presence::use_presence(open).visible;
//! crate::focus::use_escape_close(open);
//! let state = crate::presence::state_attr(open);
//! view! {
//!   <div class="relative">
//!     <button role="combobox" data-state=state on:click=move |_| open.update(|o| *o = !*o)>…</button>
//!     {move || mounted.get().then(|| view! {
//!       <div class=popover::overlay_class() data-state=state inert=inert_attr(open) aria-hidden=aria_hidden_attr(open)
//!         on:click=move |e| { popover::swallow(&e); open.set(false); }></div>
//!       <div role="listbox" data-state=state inert=inert_attr(open) aria-hidden=aria_hidden_attr(open)
//!         class=popover::panel_base()>
//!         <div class="p-1 max-h-96 overflow-y-auto">…</div>
//!       </div>
//!     })}
//!   </div>
//! }
//! ```
//! 其中 `inert_attr` / `aria_hidden_attr` 同样来自 `crate::presence`。

use leptos::html;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::cn::cn;

/// 点击后关闭弹层的透明遮罩：铺满视口、位于弹层之下。
///
/// 遮罩保持透明：像素风里「变暗」只给模态对话框用，下拉是轻量的，不压暗背景。
///
/// 两套皮肤的档位不同，所以是函数而不是常量：现代风面板是 `z-50`，遮罩留在 `z-40`；
/// 像素风面板在 `--z-popover`（75）—— **遮罩必须比面板低**，否则它会盖住选项，
/// 点什么都只是关掉下拉（把遮罩一起改成 75 时实测红过 `select.spec.ts`）。
///
/// 关闭后遮罩还会在 DOM 里留一个退场动画的时间：`data-[state=closed]:pointer-events-none`
/// 让它在这段时间里不再吞点击（需要在遮罩上写 `data-state`，见各调用点）。
#[must_use]
pub fn overlay_class() -> &'static str {
  if crate::kit::is_modern_at_mount() {
    "fixed inset-0 z-40 data-[state=closed]:pointer-events-none"
  } else {
    "fixed inset-0 z-[var(--z-modal)] data-[state=closed]:pointer-events-none"
  }
}

/// 弹层容器类名（与语言切换下拉逐字一致）：`.pxl-popover` 窗口 + 向下「跳」出的两帧动画。
///
/// 动画只有位移与显隐（`pop-in` 3 帧），没有缩放：缩放会让点阵文字在中间帧变糊。
/// 退场靠 [`crate::presence`] 的延迟卸载，期间 `data-state="closed"` 触发 `pop-out`（动画，不是过渡）；
/// 同时 `pointer-events-none`，退场中的面板不拦截点击。
/// 现代风：`md-popover`（表面底 + 1px 描边 + 圆角 + 浮层阴影），入场交给 CSS 动画。
/// 像素风保持 `pxl-popover` + `pxl-enter`（点阵文字不能缩放，只能位移+显隐）。
#[must_use]
pub fn panel_base() -> &'static str {
  if crate::kit::is_modern_at_mount() {
    "md-popover absolute left-0 top-full mt-2 z-50 max-h-96 min-w-[8rem] w-full overflow-hidden data-[state=closed]:pointer-events-none"
  } else {
    // `fixed` 而不是 `absolute`：`absolute` 会被祖先的 `overflow` 裁掉
    //（`Dialog` 面板就是 `overflow-hidden`，在它里面打开下拉会少半截），
    // 而 Tailwind v4 的 `-translate-x-1/2` 走的是独立的 `translate` 属性，
    // 不创建包含块，所以 `fixed` 仍然相对视口 —— 不需要为此把面板 Portal 到 body。
    // 具体落位由 [`use_anchored_panel`] 写内联 `left/top/width`，这里不写 `top-full`。
    "pxl-popover fixed left-0 top-0 z-[var(--z-popover)] max-h-96 min-w-[8rem] overflow-hidden pxl-enter data-[state=closed]:pointer-events-none"
  }
}

/// 让浮层贴着触发器落位，并在放不下时**翻到上方**。
///
/// 返回值写进面板的 `style`（`left/top/width`）。为什么要量而不是用 CSS：
/// 触发器的位置只有运行时知道，而「下方放不下」是移动端最常见的情形
/// （表格最后一行、贴底操作栏、390×844 的下半屏）。纯算术部分在
/// `ham_web_core::panel_place`，带单测。
///
/// 每次打开都先把面板挪到 `-9999px`（下拉同时锁死宽度，`fixed` 下 `w-full` 会铺满视口），
/// 下一帧量到真实高度再落位 —— 否则会先闪一下「贴左上的大面板」，
/// 日历 / 时间面板再次打开时也会先闪一下上次的位置。
///
/// 现代风返回 `None`：面板仍是 `absolute` + CSS 定位，不写 `style` 属性（连空属性也不留）。
pub fn use_anchored_panel(
  open: RwSignal<bool>,
  trigger: NodeRef<html::Button>,
  panel: NodeRef<html::Div>,
  // 面板宽度是否跟触发器一致（下拉 `true`；日历 / 时间面板自带固定宽度用 `false`）。
  match_trigger_width: bool,
) -> Signal<Option<String>> {
  // 现代风的下拉仍是 `absolute` + CSS 定位，写内联 `left/top` 会把版式弄乱。
  if crate::kit::is_modern_at_mount() {
    return Signal::derive(|| None);
  }
  let style = RwSignal::new(String::from("left:-9999px;top:0"));
  let measure = move || {
    let (Some(t), Some(p)) = (trigger.get_untracked(), panel.get_untracked()) else {
      return;
    };
    let a = t.get_bounding_client_rect();
    let r = p.get_bounding_client_rect();
    let vw = crate::util::document()
      .document_element()
      .map_or(0.0, |el| f64::from(el.client_width()));
    let vh = crate::util::window()
      .inner_height()
      .ok()
      .and_then(|v| v.as_f64())
      .unwrap_or(0.0);
    let placed = ham_web_core::panel_place::place(
      ham_web_core::panel_place::Rect {
        left: a.left(),
        top: a.top(),
        width: a.width(),
        height: a.height(),
      },
      r.width(),
      r.height(),
      vw,
      vh,
    );
    let width = if match_trigger_width {
      format!("width:{:.0}px", a.width().round())
    } else {
      String::new()
    };
    style.set(format!(
      "left:{:.0}px;top:{:.0}px;{width}",
      placed.left.round(),
      placed.top.round(),
    ));
  };

  Effect::new(move |_| {
    if !open.get() {
      return;
    }
    // 先挪到屏外（下拉同时锁宽度），避免闪一帧旧位置 / 铺满视口的大面板，下一帧再量高落位。
    match trigger.get_untracked() {
      Some(t) if match_trigger_width => style.set(format!(
        "left:-9999px;top:0;width:{:.0}px",
        t.get_bounding_client_rect().width().round()
      )),
      _ => style.set(String::from("left:-9999px;top:0")),
    }
    set_timeout(measure, std::time::Duration::from_millis(0));
  });

  // 跟随页面滚动（含嵌套滚动区）与窗口缩放。`scroll` 不冒泡，必须用捕获阶段。
  let scroll = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| measure());
  let _ = crate::util::window().add_event_listener_with_callback_and_bool(
    "scroll",
    scroll.as_ref().unchecked_ref(),
    true,
  );
  let resize = window_event_listener(leptos::ev::resize, move |_| measure());
  let keep = SendWrapper::new(scroll);
  on_cleanup(move || {
    resize.remove();
    let scroll = keep.take();
    let _ = crate::util::window().remove_event_listener_with_callback_and_bool(
      "scroll",
      scroll.as_ref().unchecked_ref(),
      true,
    );
  });

  Signal::derive(move || Some(style.get()))
}

/// 弹层内单个可选项的类名（与 [`crate::ui::SelectItem`] 逐字一致）：`.pxl-item`，
/// 悬停 / 聚焦时描边 + 底色，选中项左边是像素三角光标（`data-state=checked`）。
/// 现代风：`md-option`（行高 36、悬停浅底、选中蓝色左轨）。像素风保持 `.pxl-item`。
#[must_use]
pub fn item_base() -> &'static str {
  if crate::kit::is_modern_at_mount() {
    "md-option"
  } else {
    "pxl-item text-sm"
  }
}

/// 下拉选项行的完整类名。
///
/// 键盘高亮（焦点留在触发器上、由 `aria-activedescendant` 指当前行）**不再靠工具类**：
/// `.pxl-item[data-active="true"]` 的底色写在 `style/pixel/controls.css` 里，鼠标悬停与
/// 键盘导航走同一条规则。以前鼠标走 `:hover`、键盘走 `data-[active=true]:*`，
/// `:focus` 在 `div[role="option"]` 上根本不会命中 —— 漏传类名就是「方向键看不见」。
/// 现代风的 `.md-option[data-active="true"]` 同样自带浅底，所以两个分支都不需要额外类。
#[must_use]
pub fn item_class() -> String {
  item_base().to_owned()
}

/// 弹层内「不该影响触发器」的点击：吞掉冒泡与默认行为。
///
/// 表单里的控件通常套在 `<label>` 里，而 `<label>` 会把**非交互元素**上的点击「转发」给
/// 被标注的控件（用 `<span>` 当标签也能点开输入框，就是这条规则）。弹层里的选项行与遮罩
/// 是普通 `<div>`，不拦一下的话：点选项 → 选中并关闭 → 转发出的那次点击又落在触发器上 →
/// 弹层立刻重新打开；点遮罩同理「关不掉」。两个都调，浏览器两种实现都覆盖。
pub fn swallow(e: &web_sys::MouseEvent) {
  e.stop_propagation();
  e.prevent_default();
}

/// 触发器类名（与语言切换下拉逐字一致）。
///
/// `max-w-full` + `[&>span]:min-w-0`：触发器是 `w-auto` 的收缩盒子（按钮的 `width: auto`
/// 是 fit-content），而选中项的文字被 `line-clamp-1` 固定成一行 —— 它的最小内容宽度就是
/// 整行文字宽度，于是「料号 / 版式名」这类长文案会把触发器顶到比容器还宽，手机宽度下
/// 整页横向滚动（`/qsl-labels` 的版式下拉）。两者一起把触发器约束回容器内：外层的
/// `max-w-full` 封顶，内层 `min-w-0` 让文字真的能被截断。
pub fn trigger_class(size: super::control::ControlSize, extra: &str) -> String {
  super::control::control_class(
    size,
    &cn(&[
      "flex items-center justify-between gap-2 max-w-full [&>span]:min-w-0 [&>span]:line-clamp-1",
      extra,
    ]),
  )
}

/// 弹层容器类名 + 调用方覆盖（改宽度 / 内边距等；后写胜出）。
pub fn panel_class(extra: &str) -> String {
  cn(&[panel_base(), extra])
}

/// 焦点移出「弹层 + 触发器」所在的容器时关闭弹层（挂到面板的 `on:focusout`）。
///
/// 适用于面板里有可聚焦元素的弹层（日历 / 时间选择：Tab 会先进入面板再离开）。
/// 下拉列表的选项是 `tabindex="-1"`、焦点始终留在触发器上，Tab 直接越过面板，
/// 面板上的 `focusout` 根本不会触发 —— 那类弹层请用 [`close_on_focus_leave`]。
///
/// 没有这类规则时弹层会一直开着，而 `fixed inset-0 z-40` 的遮罩继续盖住整页 —— 只有 Esc
/// 或再点一下才能收掉（`use_escape_close` 只管 Esc）。
///
/// 判定边界取**面板的父元素**：组件里触发器与面板同在这个 `relative` 容器下，因此
/// 「焦点回到触发器」不会被误判成离开。`relatedTarget` 为 `None`（焦点落到文档外，
/// 例如点了非聚焦元素、或窗口失焦）同样关闭。
///
/// 返回闭包而不是自己挂监听：面板元素由组件的 `view!` 渲染，`node_ref` 只有组件自己
/// 拿得到。整个判定是同步的。
///
/// 但 `focusout` 可能在组件销毁**之后**才到达：选中选项会同步改掉筛选值，宿主视图
/// 随之重建，销毁瞬间的焦点变化会被浏览器排到队列里，此时 `open` 已经释放 ——
/// 普通 `get_untracked` 会 panic（同 `use_presence` 里卸载计时器的坑）。因此这里全部走
/// `try_*`：信号已释放就当「没开、无事发生」。
pub fn close_on_focus_out(
  open: RwSignal<bool>,
  panel: NodeRef<html::Div>,
) -> impl Fn(web_sys::FocusEvent) + Copy + 'static {
  move |e: web_sys::FocusEvent| {
    if !open.try_get_untracked().unwrap_or(false) {
      return;
    }
    let Some(el) = panel.get_untracked() else {
      return;
    };
    let Some(container) = el.parent_element() else {
      return;
    };
    let Some(next) = e.related_target() else {
      open.try_set(false);
      return;
    };
    let next: web_sys::Node = next.unchecked_into();
    if !container.contains(Some(&next)) {
      open.try_set(false);
    }
  }
}

/// 焦点离开 `root`（触发器 + 面板共同的外层容器）时关闭弹层，挂到**容器**的 `on:focusout`。
///
/// 与 [`close_on_focus_out`] 的区别是监听位置：这里挂在容器上，触发器自己失焦也会冒泡上来。
/// 下拉列表（焦点一直在触发器上、选项 `tabindex="-1"`）靠这个才能做到「Tab 离开就收起」；
/// 只挂在面板上的话，键盘用户 `↓` 打开后按 Tab，弹层与全屏遮罩会一直残留。
///
/// 判定边界是 `root` 本身：焦点在触发器与选项之间来回（鼠标点选项时焦点先落到选项上）
/// 都算「还在里面」。`relatedTarget` 为 `None`（点了非聚焦元素 / 遮罩，或窗口失焦）同样关闭。
/// 同 [`close_on_focus_out`]：`focusout` 可能在组件销毁之后才到，全部走 `try_*`。
pub fn close_on_focus_leave(
  open: RwSignal<bool>,
  root: NodeRef<html::Div>,
) -> impl Fn(web_sys::FocusEvent) + Copy + 'static {
  move |e: web_sys::FocusEvent| {
    if !open.try_get_untracked().unwrap_or(false) {
      return;
    }
    let Some(container) = root.get_untracked() else {
      return;
    };
    let Some(next) = e.related_target() else {
      open.try_set(false);
      return;
    };
    let next: web_sys::Node = next.unchecked_into();
    if !container.contains(Some(&next)) {
      open.try_set(false);
    }
  }
}

/// 把 `container` 内命中 `selector` 的子元素滚进**容器自身**的可视区。
///
/// 这里刻意不用 `scroll_into_view`：无参的 `scrollIntoView` 等价于 `{ block: "start" }`，
/// 会把目标元素连同整条祖先滚动链（含页面 `window`）一起滚到顶部 —— 打开下拉时页面就
/// 「跳」到面板处、触发器被顶出视口。改用手动比较 `get_bounding_client_rect`：只在容器
/// 内部改 `scroll_top`，页面纹丝不动。
pub fn scroll_into_view_local(container: &web_sys::Element, selector: &str) {
  let Ok(Some(node)) = container.query_selector(selector) else {
    return;
  };
  scroll_node_into_view_local(container, &node);
}

/// [`scroll_into_view_local`] 的“已拿到元素”版本：调用方已经有 `node`（比如按 id 找到的高亮项）时用，
/// 省掉再查一次选择器，也不依赖 `data-active` 之类的属性此刻是否已经写到 DOM 上。
pub fn scroll_node_into_view_local(container: &web_sys::Element, node: &web_sys::Element) {
  let list = container.get_bounding_client_rect();
  let item = node.get_bounding_client_rect();
  // 越界量：`above` 是「顶出容器上方」的距离，`below` 是「溢出容器下方」的距离。
  let above = list.top() - item.top();
  let below = item.bottom() - list.bottom();
  let delta = if above > 0.0 {
    -above
  } else if below > 0.0 {
    below
  } else {
    0.0
  };
  if delta != 0.0 {
    container.set_scroll_top(container.scroll_top() + delta.round() as i32);
  }
}

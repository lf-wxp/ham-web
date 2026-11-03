//! 弹层（popover）基座：全站下拉 / 选择面板的**唯一**外观与开合来源。
//!
//! [`crate::ui::Select`] 是下拉的唯一实现（键盘、焦点、ARIA、箭头都在那里），数据式的
//! [`crate::ui::SimpleSelect`] 只是它的一层封装；[`crate::ui::DatePicker`]、
//! [`crate::ui::TimePicker`] 复用这里的类名与开合行为 ——
//! 于是「弹出的选择框和语言切换一致」不是靠人工对齐，而是结构上只有一处定义。
//!
//! 使用方式（组件的 `view!` 里）：
//!
//! ```text
//! let open = RwSignal::new(false);
//! let mounted = popover::mount_on_open(open);
//! popover::close_on_escape(open);
//! view! {
//!   <div class="relative">
//!     <button role="combobox" data-state=popover::state(open) on:click=move |_| open.update(|o| *o = !*o)>…</button>
//!     {mounted.get().then(|| view! {
//!       <div class=popover::OVERLAY on:click=move |_| open.set(false)></div>
//!       <div role="listbox" data-state=popover::state(open) class=popover::PANEL>
//!         <div class="p-1 max-h-96 overflow-y-auto">…</div>
//!       </div>
//!     })}
//!   </div>
//! }
//! ```

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use crate::cn::cn;

/// 点击后关闭弹层的透明遮罩：铺满视口、位于弹层之下（`z-40` < 弹层 `z-50`）。
/// 遮罩保持透明：像素风里「变暗」只给模态对话框用，下拉是轻量的，不压暗背景。
///
/// 关闭后遮罩还会在 DOM 里留一个退场动画的时间：`data-[state=closed]:pointer-events-none`
/// 让它在这段时间里不再吞点击（需要在遮罩上写 `data-state`，见各调用点）。
pub const OVERLAY: &str = "fixed inset-0 z-40 data-[state=closed]:pointer-events-none";

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
    "pxl-popover absolute left-0 top-full mt-2 z-50 max-h-96 min-w-[8rem] w-full overflow-hidden pxl-enter data-[state=closed]:pointer-events-none"
  }
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

/// 键盘高亮行的额外类名（像素风）。
///
/// `.pxl-item` 只有 `:hover` / `:focus` 的底色，而下拉的键盘导航里焦点始终留在触发器上
/// （用 `aria-activedescendant` 指当前行），这两种状态都不会命中 —— 没有这条，方向键移动是「看不见」的。
/// 现代风的 `.md-option[data-active="true"]` 自带蓝调浅底，不需要工具类。
const ITEM_ACTIVE_PIXEL: &str = "data-[active=true]:bg-accent data-[active=true]:text-accent-foreground data-[active=true]:border-ink";

/// 下拉选项行的完整类名：[`item_base`] + 键盘高亮（见 [`ITEM_ACTIVE_PIXEL`]）。
///
/// 凡是带 `data-active` 的选项行（[`crate::ui::SelectItem`]）都用它，而不是裸的 `item_base()`。
#[must_use]
pub fn item_class() -> String {
  cn(&[
    item_base(),
    if crate::kit::is_modern_at_mount() {
      ""
    } else {
      ITEM_ACTIVE_PIXEL
    },
  ])
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

/// `data-state` 属性值：`"open"` / `"closed"`，驱动进出场动画。
pub fn state(open: RwSignal<bool>) -> impl Fn() -> &'static str + Copy + Send + Sync {
  move || if open.get() { "open" } else { "closed" }
}

/// 退场期间的 `inert`：与 [`state`] 配对写在弹层的**面板与遮罩**上。
///
/// 关闭后元素还要在 DOM 里留一个动画的时间，但对读屏、键盘与自动化定位器它应当已经不存在了：
/// `inert` 同时把它移出无障碍树和焦点序。没有它，选完一项后约 250ms 内已经关闭的列表框
/// 仍是无障碍树里的 `listbox` + 一串 `option`，紧接着打开的第二个下拉会同时露出两份「全部」。
/// 返回 `Option<&str>` 是 Leptos 布尔属性的写法：`Some("")` 写上，`None` 不渲染。
pub fn inert(open: RwSignal<bool>) -> impl Fn() -> Option<&'static str> + Copy + Send + Sync {
  move || (!open.get()).then_some("")
}

/// 退场期间的 `aria-hidden`：与 [`inert`] 并用。
///
/// `inert` 是现代浏览器把元素移出无障碍树的最权威写法，但并非所有辅助技术 / 自动化定位器都认它
/// （Playwright 的角色定位器只认 `aria-hidden` / `display:none` / `visibility:hidden`；
/// 老一些的屏幕阅读器 + 浏览器组合同理）。两个一起写是 WAI-ARIA 对「已不存在的元素」的常规做法。
/// `aria-hidden` 只在退场期间出现：打开时不写这个属性（而不是写成 `"false"`），避免整棵树被意外隐藏。
pub fn aria_hidden(open: RwSignal<bool>) -> impl Fn() -> Option<&'static str> + Copy + Send + Sync {
  move || (!open.get()).then_some("true")
}

/// 监听 `Escape` 关闭弹层；组件清理时移除监听。
///
/// 每个弹层各挂一个 `window` 监听，但只有 Esc 栈的栈顶响应（见 [`crate::focus::register_escape`]）：
/// 对话框里的下拉 / 日期选择器按 Esc 只收自己，不会连对话框一起关掉；输入法组词中的 Esc 也忽略。
pub fn close_on_escape(open: RwSignal<bool>) {
  let slot = crate::focus::register_escape(open);
  let handle = window_event_listener(ev::keydown, move |e| {
    if crate::focus::is_escape_for_overlay(&e) && open.get_untracked() && slot.is_top() {
      // 吃掉这次 Esc：别的 `window` 监听（页面快捷键等）不该再把它当成自己的。
      e.prevent_default();
      open.set(false);
    }
  });
  let handle = SendWrapper::new(Some(handle));
  on_cleanup(move || {
    if let Some(h) = handle.take() {
      h.remove();
    }
  });
}

/// 焦点移出「弹层 + 触发器」所在的容器时关闭弹层（挂到面板的 `on:focusout`）。
///
/// 适用于面板里有可聚焦元素的弹层（日历 / 时间选择：Tab 会先进入面板再离开）。
/// 下拉列表的选项是 `tabindex="-1"`、焦点始终留在触发器上，Tab 直接越过面板，
/// 面板上的 `focusout` 根本不会触发 —— 那类弹层请用 [`close_on_focus_leave`]。
///
/// 没有这类规则时弹层会一直开着，而 `fixed inset-0 z-40` 的遮罩继续盖住整页 —— 只有 Esc
/// 或再点一下才能收掉（`close_on_escape` 只管 Esc）。
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
/// 普通 `get_untracked` 会 panic（同 `mount_on_open` 的定时器坑）。因此这里全部走
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

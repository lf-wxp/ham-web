//! 弹层（popover）基座：全站下拉 / 选择面板的**唯一**外观与交互来源。
//!
//! 导航栏语言切换用的 [`crate::ui::Select`] 是这套样式的基准，[`crate::ui::NativeSelect`]、
//! [`crate::ui::DatePicker`]、[`crate::ui::TimePicker`] 全部复用这里的类名与开合行为 ——
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

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use crate::cn::cn;

/// 点击后关闭弹层的透明遮罩：铺满视口、位于弹层之下（`z-40` < 弹层 `z-50`）。
/// 遮罩保持透明：像素风里「变暗」只给模态对话框用，下拉是轻量的，不压暗背景。
pub const OVERLAY: &str = "fixed inset-0 z-40";

/// 弹层容器类名（与语言切换下拉逐字一致）：`.pxl-popover` 窗口 + 向下「跳」出的两帧动画。
///
/// 动画只有位移与显隐（`pop-in` 3 帧），没有缩放：缩放会让点阵文字在中间帧变糊。
/// 退场靠 [`EXIT_MS`] 的延迟卸载，期间 `data-[state=closed]` 让它反向淡出。
pub const PANEL: &str = "pxl-popover absolute left-0 top-full mt-2 z-50 max-h-96 min-w-[8rem] w-full overflow-hidden pxl-enter";

/// 弹层内单个可选项的类名（与 [`crate::ui::SelectItem`] 逐字一致）：`.pxl-item`，
/// 悬停 / 聚焦时描边 + 底色，选中项左边是像素三角光标（`data-state=checked`）。
pub const ITEM: &str = "pxl-item text-sm";

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
pub fn trigger_class(size: super::control::ControlSize, extra: &str) -> String {
  super::control::control_class(
    size,
    &cn(&[
      "flex items-center justify-between gap-2 [&>span]:line-clamp-1",
      extra,
    ]),
  )
}

/// 弹层容器类名 + 调用方覆盖（改宽度 / 内边距等；后写胜出）。
pub fn panel_class(extra: &str) -> String {
  cn(&[PANEL, extra])
}

/// `data-state` 属性值：`"open"` / `"closed"`，驱动 tw-animate 的进出场动画。
pub fn state(open: RwSignal<bool>) -> impl Fn() -> &'static str + Copy + Send + Sync {
  move || if open.get() { "open" } else { "closed" }
}

/// 关闭后延迟卸载的毫秒数：让 `animate-out` 的退场动画播完（tw-animate 默认 150ms，
/// 这里留余量）。对话框（`ui::dialog`）的退场延迟也按同一语义取值。
pub const EXIT_MS: u64 = 200;

/// 开合状态 → 「弹层是否挂在 DOM 上」。
///
/// 关闭后**延迟 [`EXIT_MS`]** 再卸载，让 `animate-out` 的退场动画播完；
/// 期间若又打开，则取消卸载。
///
/// 两个细节都在绕开同一个坑：**定时器回调里绝不能再碰信号**。筛选器常写在会重渲染的
/// 闭包里（改一次筛选值就整块重建），组件销毁后信号即释放，此时回调再读它会直接 panic
/// （`reactive_graph` 对已释放值的访问是 panic 而非 `None`），整个 wasm 实例随之崩掉。
/// 因此「是否已销毁」与「关闭代次」用普通原子量记，卸载前只需判这两个数。
pub fn mount_on_open(open: RwSignal<bool>) -> RwSignal<bool> {
  let mounted = RwSignal::new(open.get_untracked());
  let disposed = Arc::new(AtomicBool::new(false));
  let generation = Arc::new(AtomicU64::new(0));

  let flag = disposed.clone();
  on_cleanup(move || flag.store(true, Ordering::Relaxed));

  Effect::new(move |_| {
    if open.get() {
      generation.fetch_add(1, Ordering::Relaxed);
      mounted.set(true);
    } else {
      let ticket = generation.fetch_add(1, Ordering::Relaxed) + 1;
      let generation = generation.clone();
      let disposed = disposed.clone();
      set_timeout(
        move || {
          if !disposed.load(Ordering::Relaxed) && generation.load(Ordering::Relaxed) == ticket {
            mounted.set(false);
          }
        },
        Duration::from_millis(EXIT_MS),
      );
    }
  });
  mounted
}

/// 监听 `Escape` 关闭弹层；组件清理时移除监听。
///
/// 每个弹层各挂一个 `window` 监听，未打开的弹层在回调里直接返回，开销可忽略。
pub fn close_on_escape(open: RwSignal<bool>) {
  let handle = window_event_listener(ev::keydown, move |e| {
    if e.key() == "Escape" && open.get_untracked() {
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
/// 触发器用 `role="combobox"`、选项是真实 `<button>`：键盘用户 Tab 进面板再 Tab 出去时，
/// 没有这条规则弹层会一直开着，而 `fixed inset-0 z-40` 的遮罩继续盖住整页 —— 只有 Esc
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
    let Some(el) = panel.get() else {
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

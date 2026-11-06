//! 模态浮层的焦点编排：像素风与现代风共用一份实现。
//!
//! 皮肤不同、规则相同 —— 打开时把焦点送进面板、Tab / Shift+Tab 在面板内循环、
//! 关掉后把焦点还给触发按钮。像素侧原先写在 `ui/dialog/shared.rs`，现代侧后来又写了
//! 一遍（`kit_modern/presence.rs` 只有 Esc 与滚动锁）：两份实现一旦漂移，就会出现
//! 「一套弹层能用键盘、另一套不能」，所以提到这里，两个 Kit 都调它。
//!
//! 面板用 [`MODAL_ROOT`] 标记。组件只写一个属性，剩下的由这里按「最上层那个面板」统一
//! 处理：不必每个面板各接一次 `NodeRef` 回调，也就没有「DOM 还没挂上」的时序问题。

use std::cell::{Cell, RefCell};

use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

/// 模态面板上的标记属性：`focus::` 认它，e2e 也用它定位「当前最上层的弹层」。
pub const MODAL_ROOT: &str = "data-modal-root";

/// 面板内算「可交互」的元素。`[tabindex="-1"]` 排除掉：它们只用于代码聚焦面板自身。
///
/// 每一项都要单独排除 `tabindex=-1`：只在最后一项排除的话，`Input clearable` 的清除按钮
/// （`tabindex="-1"` 的 `<button>`）会被当成「最后一个可聚焦元素」，输入框里按 Tab 时
/// 首尾判定落空，焦点直接漏出面板。
const FOCUSABLE: &str = "input:not([disabled]):not([tabindex='-1']), button:not([disabled]):not([tabindex='-1']), [href]:not([tabindex='-1']), select:not([disabled]):not([tabindex='-1']), textarea:not([disabled]):not([tabindex='-1']), summary:not([tabindex='-1']), [contenteditable='true']:not([tabindex='-1']), [tabindex]:not([tabindex='-1'])";

thread_local! {
  /// 自动生成的标题 id 序号（同一页可能同时挂多个弹层）。
  static TITLE_SEQ: Cell<u32> = const { Cell::new(0) };
  /// Tab 陷阱只装一次：每个弹层各装一份会让同一次 Tab 被处理两遍。
  static TRAP_INSTALLED: Cell<bool> = const { Cell::new(false) };
  /// 当前打开着的「按 Esc 可关」的弹层，按打开先后排列（栈顶 = 最上层）。
  static ESC_STACK: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
  static ESC_SEQ: Cell<u32> = const { Cell::new(0) };
}

/// 当前聚焦的元素（打开弹层前记下来，关闭时还回去）。
pub fn active_element() -> Option<web_sys::HtmlElement> {
  crate::util::document()
    .active_element()
    .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
}

/// 打开时的统一处理：未显式命名时以内容里首个标题作为对话框名称（`aria-labelledby`），
/// 再聚焦首个可交互元素。
pub fn on_open(root: &web_sys::Element) {
  if !root.has_attribute("aria-labelledby")
    && !root.has_attribute("aria-label")
    && let Ok(Some(title)) = root.query_selector("[data-slot$='-title'], h1, h2, h3")
  {
    if title.id().is_empty() {
      let n = TITLE_SEQ.with(|c| {
        c.set(c.get() + 1);
        c.get()
      });
      title.set_id(&format!("dialog-title-{n}"));
    }
    let _ = root.set_attribute("aria-labelledby", &title.id());
  }
  focus_first(root);
}

/// 面板内「可见且可交互」的元素，按文档顺序。
///
/// 必须过滤 `offset_parent()`：`query_selector` 会把 `display: none` 的折叠区块
/// 一并返回，把焦点送进不可见控件，用户看到的就是「按了没反应」。
fn focusable_items(root: &web_sys::Element) -> Vec<web_sys::HtmlElement> {
  let Ok(nodes) = root.query_selector_all(FOCUSABLE) else {
    return Vec::new();
  };
  (0..nodes.length())
    .filter_map(|i| nodes.item(i))
    .filter_map(|n| n.dyn_into::<web_sys::HtmlElement>().ok())
    .filter(is_rendered)
    .collect()
}

/// 元素真的占了版面且没被 `visibility: hidden` 藏起来。
///
/// 不能只看 `offset_parent()`：`position: fixed` 的元素它恒为 `null`，会把固定定位的可聚焦
/// 元素误判成不可见；反过来 `visibility: hidden` 的元素有版面、却聚焦不了。
fn is_rendered(el: &web_sys::HtmlElement) -> bool {
  let rect = el.get_bounding_client_rect();
  if rect.width() <= 0.0 && rect.height() <= 0.0 {
    return false;
  }
  crate::util::window()
    .get_computed_style(el)
    .ok()
    .flatten()
    .and_then(|style| style.get_property_value("visibility").ok())
    .is_none_or(|v| v != "hidden")
}

/// 聚焦面板里第一个可交互元素；一个都没有时聚焦面板自身（面板需带 `tabindex="-1"`）。
pub fn focus_first(root: &web_sys::Element) {
  // 焦点已经在这个面板里就别抢：这个调用是**延迟一帧**发出的（见 [`use_modal_focus`]），
  // 面板刚出现的这一帧里用户完全可能已经点进了第二个输入框 —— 抢回第一个输入框，
  // 正在敲的字就落进了别的字段（自动化里必现：`fill` 的「聚焦」与「输入」隔着一次往返）。
  let active = crate::util::document().active_element();
  if active
    .as_ref()
    .is_some_and(|el| root.contains(Some(el.unchecked_ref::<web_sys::Node>())))
  {
    return;
  }
  let target = focusable_items(root)
    .into_iter()
    .next()
    .or_else(|| root.clone().dyn_into::<web_sys::HtmlElement>().ok());
  if let Some(el) = target {
    let _ = el.focus();
  }
}

/// 模态内 Tab / Shift+Tab 循环聚焦，避免焦点跑到被遮罩的页面上。
pub fn trap_tab(e: &web_sys::KeyboardEvent, root: &web_sys::Element) {
  if e.key() != "Tab" {
    return;
  }
  let items = focusable_items(root);
  let (Some(first), Some(last)) = (items.first(), items.last()) else {
    e.prevent_default();
    return;
  };
  let active = crate::util::document().active_element();
  let at = |el: &web_sys::HtmlElement| {
    active
      .as_ref()
      .is_some_and(|a| a == el.unchecked_ref::<web_sys::Element>())
  };
  let at_root = active
    .as_ref()
    .is_some_and(|a| a == root.unchecked_ref::<web_sys::Element>());
  // 焦点不在面板里（所在节点被重渲染摘掉后落到 `body`、或点击了面板外的遮罩）：
  // 直接拉回面板，而不是放行让 Tab 走进被遮住的页面。
  let outside = active.as_ref().is_none_or(|a| !root.contains(Some(a)));
  if outside {
    e.prevent_default();
    let _ = if e.shift_key() {
      last.focus()
    } else {
      first.focus()
    };
  } else if e.shift_key() && (at(first) || at_root) {
    e.prevent_default();
    let _ = last.focus();
  } else if !e.shift_key() && at(last) {
    e.prevent_default();
    let _ = first.focus();
  }
}

/// DOM 里最上层的模态面板：后挂载的在上（Portal 按挂载顺序追加到 `<body>`）。
fn topmost_modal() -> Option<web_sys::Element> {
  let nodes = crate::util::document()
    .query_selector_all(&format!("[{MODAL_ROOT}]"))
    .ok()?;
  (0..nodes.length())
    .filter_map(|i| nodes.item(i))
    .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
    // 退场动画期间面板还留在 DOM 里（约 200ms）：它已经「关了」，不能再当最上层，
    // 否则快速 Esc 后焦点会被抢到下层面板的第一个控件上。`data-state` 现代风写在面板的
    // 父级（遮罩）上、像素风写在面板自身，`closest` 两种都覆盖。
    .rfind(|el| el.closest("[data-state='closed']").ok().flatten().is_none())
}

/// 当前是否有未退场的模态面板（给全局快捷键判断「现在是不是在弹层里」）。
#[must_use]
pub fn has_open_modal() -> bool {
  topmost_modal().is_some()
}

/// 弹层的焦点生命周期：记住触发者 → 打开后聚焦面板 → 关闭 / 卸载时归还。
///
/// 与 [`crate::overlay::use_scroll_lock`] 一起构成「模态副作用」的最小集，
/// 两套 Kit 的每个模态都该各调一次。
pub fn use_modal_focus(open: RwSignal<bool>) {
  let opener = StoredValue::new(SendWrapper::new(None::<web_sys::HtmlElement>));
  Effect::new(move |_| {
    if open.get() {
      // 只在「从关到开」记一次：连开两层弹层时，上层不该把下层面板里的按钮记成触发者。
      if opener.with_value(|o| o.is_none()) {
        opener.set_value(SendWrapper::new(active_element()));
      }
      // 面板是 `open.then(…)` 挂上去的，这一帧 DOM 里还没有：等下一帧再聚焦。
      request_animation_frame(move || {
        // 开了立刻又关（快速 Esc）：这一帧已经不该聚焦了，否则会把焦点抢到下层面板。
        if open.try_get_untracked() != Some(true) {
          return;
        }
        if let Some(root) = topmost_modal() {
          on_open(&root);
        }
      });
    } else {
      restore_opener(&opener);
    }
  });
  install_trap_once();
  // 卸载时也归还：弹层可能在打开状态被卸载（风格切换重建子树）。
  on_cleanup(move || restore_opener(&opener));
}

/// Esc 栈里的一格：只有栈顶（最上层的弹层）才响应 Esc。
#[derive(Clone, Copy)]
pub struct EscapeSlot(u32);

impl EscapeSlot {
  /// 自己是不是当前最上层的、打开着的弹层。
  #[must_use]
  pub fn is_top(self) -> bool {
    ESC_STACK.with(|stack| stack.borrow().last() == Some(&self.0))
  }
}

/// 把一个弹层登记进 Esc 栈：`open` 为真时入栈、为假时出栈，卸载时也出栈。
///
/// 为什么要栈：每个弹层各挂一个 `window` 监听，若都无条件响应 Esc，确认框叠在对话框上时
/// 一次 Esc 两个都关；对话框里的下拉 / 日期选择器按 Esc 收面板，会连整个对话框一起关掉，
/// 表单草稿跟着丢。有了栈，只有最后打开的那一层处理，与监听器的注册顺序无关。
pub fn register_escape(open: RwSignal<bool>) -> EscapeSlot {
  let id = ESC_SEQ.with(|c| {
    c.set(c.get().wrapping_add(1));
    c.get()
  });
  let leave = move || ESC_STACK.with(|stack| stack.borrow_mut().retain(|x| *x != id));
  Effect::new(move |_| {
    if open.get() {
      ESC_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        if !stack.contains(&id) {
          stack.push(id);
        }
      });
    } else {
      leave();
    }
  });
  on_cleanup(leave);
  EscapeSlot(id)
}

/// Esc 事件是否该由弹层处理：不是输入法组词中的那一下（组词时 Esc 是「取消候选」）。
#[must_use]
pub fn is_escape_for_overlay(e: &web_sys::KeyboardEvent) -> bool {
  e.key() == "Escape" && !e.is_composing() && e.key_code() != 229
}

/// 按 Esc 关闭弹层。
///
/// 挂在 `window` 上而不是某个控件上：焦点可能停在面板里的任意元素（输入框、列表项），
/// 只在输入框的 `on:keydown` 上处理 Esc，用户 Tab 出去之后就按不动了。
/// 两套 Kit 的每个弹层都调这一份（`check-structure` 的 C7.8 守着）；叠层时只有最上层响应，
/// 见 [`register_escape`]。
pub fn use_escape_close(open: RwSignal<bool>) {
  let slot = register_escape(open);
  let handle = window_event_listener(leptos::ev::keydown, move |e| {
    if is_escape_for_overlay(&e) && open.get_untracked() && slot.is_top() {
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

/// 把焦点还给打开弹层的那个元素（`None` 时什么都不做）。
pub fn restore(el: Option<web_sys::HtmlElement>) {
  let Some(el) = el else { return };
  if el.is_connected() {
    let _ = el.focus();
    return;
  }
  // 触发者已经不在文档里（切风格重建了顶栏、从另一个正在卸载的弹层里打开的）：
  // 焦点落到 `body` 会让键盘用户从页面最顶端重新开始，退到正文入口更接近原来的位置。
  if let Some(main) = crate::util::document()
    .get_element_by_id("main-content")
    .and_then(|m| m.dyn_into::<web_sys::HtmlElement>().ok())
  {
    let _ = main.focus();
  }
}

/// 把焦点还给打开弹层的那个元素，并清掉记录（重复调用是安全的）。
fn restore_opener(opener: &StoredValue<SendWrapper<Option<web_sys::HtmlElement>>>) {
  let previous = opener.with_value(|o| (**o).clone());
  opener.set_value(SendWrapper::new(None));
  restore(previous);
}

/// 装一次全局 Tab 陷阱。监听器活到文档结束（整个应用生命周期只有一个）。
fn install_trap_once() {
  if TRAP_INSTALLED.with(|c| c.replace(true)) {
    return;
  }
  let handle = window_event_listener(leptos::ev::keydown, |e| {
    if e.key() != "Tab" {
      return;
    }
    if let Some(root) = topmost_modal() {
      trap_tab(&e, &root);
    }
  });
  std::mem::forget(handle);
}

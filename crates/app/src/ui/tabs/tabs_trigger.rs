//! [`TabsTrigger`]：标签栏里的一项。

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::shared::TabsCtx;

/// 一组触发项里当前处于「可用」状态的那些（禁用项不参与方向键移动）。
fn enabled_triggers(ev: &web_sys::KeyboardEvent) -> Vec<web_sys::HtmlElement> {
  let Some(current) = ev
    .current_target()
    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
  else {
    return Vec::new();
  };
  let Some(list) = current.parent_element() else {
    return Vec::new();
  };
  let Ok(nodes) = list.query_selector_all("[data-slot=\"tabs-trigger\"]:not([disabled])") else {
    return Vec::new();
  };
  (0..nodes.length())
    .filter_map(|i| nodes.item(i))
    .filter_map(|n| n.dyn_into::<web_sys::HtmlElement>().ok())
    .collect()
}

/// 当前触发项在这组里的下标。
fn current_index(ev: &web_sys::KeyboardEvent, items: &[web_sys::HtmlElement]) -> Option<usize> {
  let node = ev
    .current_target()
    .and_then(|t| t.dyn_into::<web_sys::Node>().ok())?;
  items.iter().position(|it| it.is_same_node(Some(&node)))
}

/// 焦点落到目标项并**顺带激活**它（WAI-ARIA tabs 的自动激活）。
///
/// 走 `click` 而不是直接调 `on_change`：禁用 / 已激活两条守卫都只写在
/// 组件的 `click` 里，键盘与鼠标因此共用同一份逻辑。
fn activate(target: &web_sys::HtmlElement) {
  let _ = target.focus();
  target.click();
}

/// 方向键：在可用项之间环形移动。
fn step(ev: &web_sys::KeyboardEvent, step: isize) {
  let items = enabled_triggers(ev);
  let Some(idx) = current_index(ev, &items) else {
    return;
  };
  let len = items.len() as isize;
  if len == 0 {
    return;
  }
  let next = (idx as isize + step).rem_euclid(len) as usize;
  if let Some(target) = items.get(next) {
    activate(target);
  }
}

/// Home / End：跳到首项 / 末项。
fn edge(ev: &web_sys::KeyboardEvent, last: bool) {
  let items = enabled_triggers(ev);
  let target = if last { items.last() } else { items.first() };
  if let Some(target) = target {
    activate(target);
  }
}

/// 标签栏里的一项。
///
/// 键盘：`←` / `→`（`↑` / `↓` 同样接受）环形移动并激活，`Home` / `End` 跳首尾。
/// 组内做 roving tabindex：只有激活项在 Tab 序里，整组只占一个 Tab 位。
#[component]
pub fn TabsTrigger(
  /// 本项对应的值：与 [`Tabs`](super::Tabs) 的 `value` 同类型同取值。
  /// 同时用于拼 `id`，请用 ascii slug（如 `koch` / `send`）。
  #[prop(into)]
  value: String,
  #[prop(optional, into)] disabled: Signal<bool>,
  children: Children,
) -> impl IntoView {
  let ctx = expect_context::<TabsCtx>();
  let tab_id = ctx.tab_id(&value);
  let panel_id = ctx.panel_id(&value);
  let v = StoredValue::new(value);
  let active = move || ctx.is_active(&v);
  view! {
    <button
      type="button"
      role="tab"
      data-slot="tabs-trigger"
      id=tab_id
      aria-selected=move || active().to_string()
      aria-controls=panel_id
      data-state=move || if active() { "active" } else { "inactive" }
      // roving tabindex：非激活项不进 Tab 序，组内靠方向键。
      tabindex=move || if active() { "0" } else { "-1" }
      disabled=move || disabled.get()
      class="pxl-tab"
      on:click=move |_| {
        if !disabled.get_untracked() && !active() {
          ctx.on_change.run(v.get_value());
        }
      }
      on:keydown=move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
        "ArrowRight" | "ArrowDown" => {
          // 方向键在这里是「换标签」，不兼作页面滚动。
          ev.prevent_default();
          step(&ev, 1);
        }
        "ArrowLeft" | "ArrowUp" => {
          ev.prevent_default();
          step(&ev, -1);
        }
        "Home" => {
          ev.prevent_default();
          edge(&ev, false);
        }
        "End" => {
          ev.prevent_default();
          edge(&ev, true);
        }
        _ => {}
      }
    >
      {children()}
    </button>
  }
}

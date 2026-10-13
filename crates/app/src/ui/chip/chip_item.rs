//! [`Chip`]：组里的一项（互斥选择）。
//!
//! 文件名不叫 `chip.rs`：`chip/chip.rs` 会触发 Rust 的「模块与父模块同名」限制
//! （`dialog/dialog_root.rs`、`select/select_root.rs` 同理）。

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::shared::{CHIP_BASE, ChipCtx};

/// 方向键在同一组内移动选择：焦点先落到目标项，再交给它的 `click` ——
/// 于是 `aria-checked` 与 `on_change` 都只有一条实现（WAI-ARIA 的「选中随焦点」）。
///
/// 刻意没做 roving tabindex：所有 chip 仍在 Tab 序里（见 `docs/ui-components.md` §6）。
fn move_focus(ev: &web_sys::KeyboardEvent, step: isize) {
  let Some(current) = ev
    .current_target()
    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
  else {
    return;
  };
  let Some(current_node) = current.dyn_ref::<web_sys::Node>() else {
    return;
  };
  let Some(group) = current.parent_element() else {
    return;
  };
  let Ok(nodes) = group.query_selector_all("[data-slot=\"chip\"]") else {
    return;
  };
  let chips: Vec<web_sys::HtmlElement> = (0..nodes.length())
    .filter_map(|i| nodes.item(i))
    .filter_map(|n| n.dyn_into::<web_sys::HtmlElement>().ok())
    .collect();
  let Some(idx) = chips
    .iter()
    .position(|c| c.is_same_node(Some(current_node)))
  else {
    return;
  };
  let len = chips.len() as isize;
  if len == 0 {
    return;
  }
  let next = (idx as isize + step).rem_euclid(len) as usize;
  if let Some(target) = chips.get(next) {
    let _ = target.focus();
    // `click` 而非直接调 `on_change`：已选中 / 禁用两条守卫都在组件的 click 里。
    target.click();
  }
}

/// 组里的一项。必须放在 [`ChipGroup`](super::ChipGroup) 的 `children` 里就地构建
/// （它靠 context 拿到组的值与回调）。
#[component]
pub fn Chip(#[prop(into)] value: String, children: Children) -> impl IntoView {
  let ctx = expect_context::<ChipCtx>();
  let v = StoredValue::new(value);
  let checked = move || ctx.value.with(|cur| v.with_value(|v| cur == v));
  view! {
    <button
      type="button"
      role="radio"
      data-slot="chip"
      value=v.get_value()
      aria-checked=move || checked().to_string()
      data-state=move || if checked() { "checked" } else { "unchecked" }
      disabled=move || ctx.disabled.get()
      class=CHIP_BASE
      on:click=move |_| {
        if !ctx.disabled.get_untracked() && !checked() {
          ctx.on_change.run(v.get_value());
        }
      }
      on:keydown=move |ev: web_sys::KeyboardEvent| {
        let step = match ev.key().as_str() {
          "ArrowRight" | "ArrowDown" => 1,
          "ArrowLeft" | "ArrowUp" => -1,
          _ => return,
        };
        // 方向键在这里是「移动选择」，不兼作页面滚动。
        ev.prevent_default();
        move_focus(&ev, step);
      }
    >
      {children()}
    </button>
  }
}

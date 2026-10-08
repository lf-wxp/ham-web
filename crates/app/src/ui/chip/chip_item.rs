//! [`Chip`]：组里的一项（互斥选择）。
//!
//! 文件名不叫 `chip.rs`：`chip/chip.rs` 会触发 Rust 的「模块与父模块同名」限制
//! （`dialog/dialog_root.rs`、`select/select_root.rs` 同理）。

use leptos::prelude::*;

use crate::cn::cn;

use super::shared::{CHIP_BASE, CHIP_OFF, CHIP_ON, ChipCtx};

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
      class=move || cn(&[CHIP_BASE, if checked() { CHIP_ON } else { CHIP_OFF }])
      on:click=move |_| {
        if !ctx.disabled.get_untracked() && !checked() {
          ctx.on_change.run(v.get_value());
        }
      }
    >
      {children()}
    </button>
  }
}

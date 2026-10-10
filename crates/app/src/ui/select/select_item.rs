use leptos::prelude::*;

use super::super::popover;
use super::select_root::SelectCtx;

/// 下拉选项。
#[component]
pub fn SelectItem(#[prop(into)] value: String, children: Children) -> impl IntoView {
  let ctx = expect_context::<SelectCtx>();
  let v = StoredValue::new(value);
  let selected = move || {
    ctx
      .value
      .with(|cur| v.with_value(|v| cur.as_deref() == Some(v.as_str())))
  };
  view! {
    <div
      role="option"
      aria-selected=move || selected().to_string()
      data-state=move || if selected() { "checked" } else { "unchecked" }
      tabindex="-1"
      class=popover::ITEM
      on:click=move |e| {
        popover::swallow(&e);
        ctx.on_change.run(v.get_value());
        ctx.open.set(false);
      }
    >
      <span class="w-full">{children()}</span>
    </div>
  }
}

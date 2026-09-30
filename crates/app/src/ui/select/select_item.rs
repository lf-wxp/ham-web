use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

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
      class="relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
      on:click=move |_| {
        ctx.on_change.run(v.get_value());
        ctx.open.set(false);
      }
    >
      <span class="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
        {move || selected().then(|| view! { <Icon kind=IconKind::Check class="h-4 w-4" /> })}
      </span>
      <span class="w-full">{children()}</span>
    </div>
  }
}

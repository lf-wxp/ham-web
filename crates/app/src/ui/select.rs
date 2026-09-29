//! 下拉选择（对应 Radix Select，popper 定位）。

use leptos::ev;
use leptos::prelude::*;
use send_wrapper::SendWrapper;

use crate::icons::{Icon, IconKind};

#[derive(Clone, Copy)]
struct SelectCtx {
  value: Signal<Option<String>>,
  on_change: Callback<String>,
  open: RwSignal<bool>,
}

/// `trigger` 为选中项在触发器中的展示内容；`children` 为若干 [`SelectItem`]。
#[component]
pub fn Select(
  #[prop(into)] value: Signal<Option<String>>,
  on_change: Callback<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(into)] placeholder: String,
  #[prop(into)] trigger: ViewFn,
  children: ChildrenFn,
) -> impl IntoView {
  let open = RwSignal::new(false);
  provide_context(SelectCtx {
    value,
    on_change,
    open,
  });

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

  let state = move || if open.get() { "open" } else { "closed" };
  view! {
    <div class="relative">
      <button
        type="button"
        role="combobox"
        aria-expanded=move || open.get().to_string()
        data-state=state
        disabled=move || disabled.get()
        class="flex h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 [&>span]:line-clamp-1"
        on:click=move |_| open.update(|o| *o = !*o)
      >
        <span style="pointer-events: none;">
          {move || {
            if value.with(Option::is_some) {
              trigger.run()
            } else {
              view! { <span class="text-muted-foreground">{placeholder.clone()}</span> }.into_any()
            }
          }}
        </span>
        <Icon kind=IconKind::ChevronDown class="h-4 w-4 opacity-50" />
      </button>
      {move || {
        open
          .get()
          .then(|| {
            view! {
              <div class="fixed inset-0 z-40" on:click=move |_| open.set(false)></div>
              <div
                role="listbox"
                data-state="open"
                data-side="bottom"
                class="absolute left-0 top-full mt-1 z-50 max-h-96 min-w-[8rem] w-full overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2"
              >
                <div class="p-1 w-full overflow-y-auto max-h-96">{children()}</div>
              </div>
            }
          })
      }}
    </div>
  }
}

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

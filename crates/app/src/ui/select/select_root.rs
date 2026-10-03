use std::time::Duration;

use leptos::ev;
use leptos::prelude::*;
use send_wrapper::SendWrapper;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

use super::super::control::{ControlSize, TextValue, control_class, invalid_attr};

#[derive(Clone, Copy)]
pub(super) struct SelectCtx {
  pub(super) value: Signal<Option<String>>,
  pub(super) on_change: Callback<String>,
  pub(super) open: RwSignal<bool>,
}

/// `trigger` 为选中项在触发器中的展示内容；`children` 为若干 [`SelectItem`]。
#[component]
pub fn Select(
  #[prop(into)] value: Signal<Option<String>>,
  on_change: Callback<String>,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(into)] placeholder: TextValue,
  #[prop(into)] trigger: ViewFn,
  /// 触发按钮尺寸（默认与输入框对齐）。
  #[prop(optional)]
  size: ControlSize,
  /// 错误态。
  #[prop(optional, into)]
  invalid: Signal<bool>,
  /// 无可见标签时的无障碍名称。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
  #[prop(optional, into)] id: Option<String>,
  #[prop(optional, into)] class: String,
  children: ChildrenFn,
) -> impl IntoView {
  let open = RwSignal::new(false);
  // 关闭后延迟卸载，让列表的 `animate-out` 退场动画播完再移除 DOM
  // （tw-animate 默认 150ms，这里留 200ms 余量）。
  let mounted = RwSignal::new(open.get_untracked());
  Effect::new(move |_| {
    if open.get() {
      mounted.set(true);
    } else {
      set_timeout(
        move || {
          if !open.get_untracked() {
            mounted.set(false);
          }
        },
        Duration::from_millis(200),
      );
    }
  });
  let title = placeholder.clone();
  let trigger_class = control_class(
    size,
    &cn(&[
      "flex items-center justify-between [&>span]:line-clamp-1",
      &class,
    ]),
  );
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
        id=id
        aria-haspopup="listbox"
        aria-expanded=move || open.get().to_string()
        aria-label=move || aria_label.as_ref().map(TextValue::get)
        aria-invalid=invalid_attr(invalid)
        title=move || title.get()
        data-state=state
        disabled=move || disabled.get()
        class=trigger_class
        on:click=move |_| open.update(|o| *o = !*o)
      >
        <span style="pointer-events: none;">
          {move || {
            if value.with(Option::is_some) {
              trigger.run()
            } else {
              view! { <span class="text-muted-foreground">{placeholder.get()}</span> }.into_any()
            }
          }}
        </span>
        <Icon
          kind=IconKind::ChevronDown
          class=Signal::derive(move || {
            cn(&[
              "h-4 w-4 opacity-50 transition-transform duration-200",
              if open.get() { "rotate-180" } else { "" },
            ])
          })
        />
      </button>
      {move || {
        mounted
          .get()
          .then(|| {
            view! {
              <div class="fixed inset-0 z-40" on:click=move |_| open.set(false)></div>
              <div
                role="listbox"
                data-state=state
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

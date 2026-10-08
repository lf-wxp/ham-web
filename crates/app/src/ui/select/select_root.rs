use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};

use super::super::control::{ControlSize, TextValue, invalid_attr};
use super::super::popover;

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
  /// 弹层额外类名（改宽度等）。
  #[prop(optional, into)]
  panel_class: String,
  children: ChildrenFn,
) -> impl IntoView {
  // 空串一律当作「未设置」：`aria-label=""` 会被读屏当成空名称，`id=""` 也是无效值 ——
  // 都比属性缺失更糟。转发自 [`super::super::NativeSelect`] 的未设置值因此能安全透传。
  let aria_label = aria_label.filter(|v| !v.is_empty());
  let id = id.filter(|s| !s.is_empty());

  let open = RwSignal::new(false);
  // 关闭后延迟卸载（退场动画），开合行为见 `popover` 模块。
  let mounted = popover::mount_on_open(open);
  popover::close_on_escape(open);
  let title = placeholder.clone();
  let trigger_class = popover::trigger_class(size, &class);
  provide_context(SelectCtx {
    value,
    on_change,
    open,
  });

  let state = popover::state(open);
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
        <Icon kind=IconKind::ChevronDown class=Signal::derive(move || {
          cn(&[
            "h-4 w-4 opacity-50 transition-transform duration-200",
            if open.get() { "rotate-180" } else { "" },
          ])
        }) />
      </button>
      {move || {
        mounted
          .get()
          .then(|| {
            view! {
              <div class=popover::OVERLAY on:click=move |e| { popover::swallow(&e); open.set(false); }></div>
              <div
                role="listbox"
                data-state=state
                data-side="bottom"
                class=cn(&[popover::PANEL, &panel_class])
              >
                <div class="p-1 w-full overflow-y-auto max-h-96">{children()}</div>
              </div>
            }
          })
      }}
    </div>
  }
}

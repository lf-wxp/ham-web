//! 表单行：标签 + 控件 + 说明 + 错误提示的统一排版。
//!
//! 页面上反复出现「`<div class="space-y-1.5"><label>…</label><input/></div>`」，
//! 抽出后标签字号、间距、必填标记与错误文案只有一处定义。

use leptos::prelude::*;

use crate::cn::cn;

use super::control::TextValue;
use super::label_class;

/// 表单行。
#[component]
pub fn Field(
  /// 标签文案。
  #[prop(into)]
  label: TextValue,
  children: Children,
  /// 关联控件的 `id`（点击标签即可聚焦）。
  #[prop(optional, into)]
  r#for: Option<String>,
  /// 标签下方的补充说明。
  #[prop(optional, into)]
  hint: Option<TextValue>,
  /// 错误文案；非空即视为错误态。
  #[prop(optional, into)]
  error: Option<Signal<String>>,
  #[prop(optional)] required: bool,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  view! {
    <div class=cn(&["space-y-1.5", &class])>
      <label for=r#for class=label_class("")>
        {move || label.get()}
        {required
          .then(|| {
            view! { <span class="text-destructive" aria-hidden="true">"*"</span> }
          })}
      </label>
      {children()}
      {hint
        .map(|h| {
          view! { <p class="text-xs text-muted-foreground">{move || h.get()}</p> }
        })}
      {error
        .map(|e| {
          view! {
            <Show when=move || !e.get().is_empty()>
              <p class="text-xs text-destructive" role="alert">{move || e.get()}</p>
            </Show>
          }
        })}
    </div>
  }
}

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
  /// 仅「有 / 无错误」而没有具体文案时用这个：非空即渲染一行默认提示。
  ///
  /// 计算器一类的表单只做「能否解析」的判定（`NumberField` 的 `invalid`），
  /// 没有可读的错误句子；以前这种情况界面上只有一圈红边、没有解释。
  #[prop(optional, into)]
  invalid: Signal<bool>,
  #[prop(optional)] required: bool,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let modern = crate::kit::is_modern_at_mount();
  view! {
    <div class=if modern { cn(&["md-field", &class]) } else { cn(&["space-y-1.5", &class]) }>
      <label for=r#for class=if modern { "md-field-label".to_owned() } else { label_class("") }>
        {move || label.get()}
        {required
          .then(|| {
            view! { <span class=if modern { "md-field-required" } else { "text-hp-text" } aria-hidden="true">"*"</span> }
          })}
      </label>
      {children()}
      {hint
        .map(|h| {
          view! { <p class=if modern { "md-field-hint".to_owned() } else { "text-xs text-muted-foreground".to_owned() }>{move || h.get()}</p> }
        })}
      {match error {
        Some(e) => {
          view! {
            <Show when=move || !e.get().is_empty()>
              <p class=if modern { "md-field-error".to_owned() } else { "text-xs text-hp-text".to_owned() } role="alert">{move || e.get()}</p>
            </Show>
          }
            .into_any()
        }
        None => {
          view! {
            <Show when=move || invalid.get()>
              <p class=if modern { "md-field-error".to_owned() } else { "text-xs text-hp-text".to_owned() } role="alert">
                {move || crate::i18n::t("common.check-this-field")}
              </p>
            </Show>
          }
            .into_any()
        }
      }}
    </div>
  }
}

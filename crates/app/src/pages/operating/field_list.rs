use crate::data::kt_tracked;
use leptos::prelude::*;

/// 渲染一个「名称 → 说明」的字段列表。
#[component]
pub(super) fn FieldList(fields: &'static [(&'static str, &'static str)]) -> impl IntoView {
  view! {
    <dl class="divide-y">
      {fields
        .iter()
        .map(|&(k, v)| {
          view! {
            <div class="grid gap-1 px-4 py-2.5 sm:grid-cols-[8rem_1fr]">
              <dt class="text-sm">{move || kt_tracked(k)}</dt>
              <dd class="text-sm text-muted-foreground">{move || kt_tracked(v)}</dd>
            </div>
          }
        })
        .collect_view()}
    </dl>
  }
}

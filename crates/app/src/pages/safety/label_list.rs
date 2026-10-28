use crate::data::kt_tracked;
use leptos::prelude::*;

/// 渲染「名称 → 说明」列表。
#[component]
pub(super) fn LabelList(rows: &'static [(&'static str, &'static str)]) -> impl IntoView {
  view! {
    <dl class="divide-y">
      {rows
        .iter()
        .map(|&(k, v)| {
          view! {
            <div class="grid gap-1 px-4 py-3 sm:grid-cols-[12rem_1fr]">
              <dt>{move || kt_tracked(k)}</dt>
              <dd class="text-sm text-muted-foreground">{move || kt_tracked(v)}</dd>
            </div>
          }
        })
        .collect_view()}
    </dl>
  }
}

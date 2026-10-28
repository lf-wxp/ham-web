use crate::data::kt_tracked;
use leptos::prelude::*;

/// 渲染一个「编号 → 说明」的列表。
#[component]
pub(super) fn LabelList(rows: &'static [(&'static str, &'static str)]) -> impl IntoView {
  view! {
    <dl class="divide-y">
      {rows
        .iter()
        .map(|&(k, v)| {
          view! {
            <div class="flex items-baseline gap-3 px-4 py-2.5">
              <dt class="w-12 shrink-0 font-mono text-sm text-primary">{move || kt_tracked(k)}</dt>
              <dd class="text-sm text-muted-foreground">{move || kt_tracked(v)}</dd>
            </div>
          }
        })
        .collect_view()}
    </dl>
  }
}

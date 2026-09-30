use leptos::prelude::*;

/// 渲染一个要点列表。
#[component]
pub(super) fn TipList(tips: &'static [&'static str]) -> impl IntoView {
  view! {
    <ul class="space-y-2 p-4">
      {tips
        .iter()
        .map(|tip| {
          view! {
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{*tip}</span>
            </li>
          }
        })
        .collect_view()}
    </ul>
  }
}

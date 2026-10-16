use leptos::prelude::*;

use crate::i18n::tf;
use crate::ui::{Button, Size, Variant};

/// 一组候选（标题 + 呼号按钮 + 「还有 N 个」）。空组不渲染。
#[component]
pub(super) fn ScpGroup(
  heading: String,
  calls: Vec<String>,
  total: usize,
  on_pick: Callback<String>,
) -> impl IntoView {
  let more = total.saturating_sub(calls.len());
  (!calls.is_empty()).then(|| {
    view! {
      <div class="space-y-1">
        <div class="text-xs text-muted-foreground">{heading}</div>
        <div class="flex flex-wrap items-center gap-1.5">
          {calls
            .into_iter()
            .map(|c| {
              let pick = c.clone();
              // aria_label 要在切语言时跟随更新：包成派生信号，别在组件体内直接 tf()。
              let label = {
                let c = c.clone();
                Signal::derive(move || tf("contest.scp-pick", &[&c]))
              };
              view! {
                <Button
                  variant=Variant::Outline
                  size=Size::Xs
                  class="font-mono"
                  aria_label=label
                  on_click=Callback::new(move |_| on_pick.run(pick.clone()))
                >
                  {c}
                </Button>
              }
            })
            .collect_view()}
          {(more > 0).then(|| view! {
            <span class="text-xs text-muted-foreground">
              {move || tf("contest.scp-more", &[&more.to_string()])}
            </span>
          })}
        </div>
      </div>
    }
  })
}

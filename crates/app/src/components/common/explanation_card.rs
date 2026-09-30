use ham_web_core::QuestionItem;
use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};
use crate::speech;
use crate::ui::card_class;

/// 可折叠的「答案解析」卡片；题目切换时由调用方重建，自动恢复折叠状态。
#[component]
pub fn ExplanationCard(question: QuestionItem) -> impl IntoView {
  let open = RwSignal::new(false);
  let text = question
    .explanation
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(ToOwned::to_owned);
  text.map(|text| {
    let speak_text = text.clone();
    view! {
      <div data-slot="card" class=card_class("gap-0 py-0")>
        <div class="flex items-center">
          <button
            type="button"
            on:click=move |_| open.update(|v| *v = !*v)
            aria-expanded=move || open.get().to_string()
            class="flex flex-1 items-center justify-between gap-2 px-6 py-4 text-left cursor-pointer transition-colors hover:bg-accent/50 outline-none focus-visible:ring-2 focus-visible:ring-ring/50 rounded-xl"
          >
            <span class="text-base font-semibold leading-none">"答案解析"</span>
            <Icon
              kind=IconKind::ChevronDown
              class=Signal::derive(move || cn(&["h-4 w-4 shrink-0 transition-transform", if open.get() { "rotate-180" } else { "" }]))
            />
          </button>
          <button
            type="button"
            class="mr-4 flex size-8 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            title="朗读解析"
            aria-label="朗读解析"
            on:click=move |_| speech::speak_zh(&speak_text)
          >
            <Icon kind=IconKind::Volume2 class="h-4 w-4" />
          </button>
        </div>
        {move || {
          open
            .get()
            .then(|| {
              view! {
                <div class="px-6 pb-5">
                  <div class="whitespace-pre-line text-sm leading-6 text-muted-foreground border-t pt-4">
                    {text.clone()}
                  </div>
                </div>
              }
            })
        }}
      </div>
    }
  })
}

//! 首页「知识卡片」：到期卡片数与入口。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::{t, tf, tp};
use crate::pages::load_card_schedule;
use crate::ui::{Size, Variant, button_class};
use crate::util::now_ms;

#[component]
pub(super) fn CardsCard() -> impl IntoView {
  let schedule = load_card_schedule();
  let now = now_ms();
  let due = schedule.due_total(now);
  let learned = schedule.cards.len();
  view! {
    <SectionCard
      title=Signal::derive(move || t("shell.study-cards"))
      meta=ViewFn::from(move || (learned > 0).then(|| tf("home.learned", &[&learned.to_string()])))
    >
      <div class="flex flex-wrap items-center gap-3 p-4">
        <p class="mr-auto text-sm text-muted-foreground">
          {move || {
            if learned == 0 {
              t("home.q-codes-abbreviations-phonetic")
            } else if due > 0 {
              tp("home.cards-are-due-for", due as u32, &[&due.to_string()])
            } else {
              t("home.all-cards-reviewed-today")
            }
          }}
        </p>
        // 刻意的例外：链接的高亮要随「今日待复习数」切换 `variant`，而 `ButtonLink` 的
        // `variant` 是静态 prop（见 `docs/ui-components.md` 的「常见坑」第 4 条）。
        <a href="/cards" class=button_class(if due > 0 { Variant::Default } else { Variant::Outline }, Size::Sm, "")>
          {move || if due > 0 { tf("home.review", &[&due.to_string()]) } else if learned == 0 { t("home.start-learning") } else { t("home.learn-new") }}
        </a>
      </div>
    </SectionCard>
  }
}

use ham_web_core::categories::top_category;
use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::{bank_class, t, tf, tp};
use crate::pages::load_card_schedule;
use crate::study;
use crate::ui::{ButtonLink, Size, Variant};
use crate::util::now_ms;

/// 今日学习队列：错题、知识卡片与薄弱分类的统一定期复习入口。
#[component]
pub(super) fn ReviewCard() -> impl IntoView {
  let book = study::load_book();
  let stats = study::load_stats();
  let schedule = load_card_schedule();
  let now = now_ms();

  if book.records.is_empty() && schedule.cards.is_empty() {
    return ().into_any();
  }

  let mistakes_due = book.due_count(now);
  let cards_due = schedule.due_total(now);
  let total_due = mistakes_due + cards_due;

  let bank = stats.main_bank();
  let weakest = stats
    .categories_of(bank)
    .and_then(|c| c.weakest(5))
    .and_then(|(key, t)| {
      let top = top_category(key)?;
      Some((top.key, top.name, t.rate()? * 100.0))
    });
  let bank = bank.unwrap_or_default();
  let due_in_bank = book
    .records
    .iter()
    .filter(|r| r.is_due(now) && r.in_bank(bank))
    .count();

  view! {
    <SectionCard
      title=Signal::derive(move || t("exam.today-s-study-queue"))
      meta=ViewFn::from(move || tp("exam.items-due-today", total_due, &[&total_due.to_string()]))
    >
      <div class="space-y-2 p-4">
        {move || {
          if total_due == 0 {
            return view! {
              <p class="text-sm text-muted-foreground">
                {t("exam.all-reviews-done-for")}
              </p>
            }
            .into_any();
          }
          view! {
            <div class="space-y-2">
              {(mistakes_due > 0).then(|| {
                view! {
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm">{tp("exam.mistakes", mistakes_due, &[&mistakes_due.to_string()])}</span>
                    <ButtonLink
                      href="/mistakes?review=1"
                      variant=Variant::Default
                      size=Size::Sm
                    >
                      {t("exam.review-mistakes")}
                    </ButtonLink>
                    {(due_in_bank > 0 && due_in_bank < mistakes_due).then(|| view! {
                      <ButtonLink
                        href=format!("/mistakes?review=1&bank={bank}")
                        variant=Variant::Outline
                        size=Size::Sm
                      >
                        {tf("exam.only-class", &[&(bank_class(&bank.to_string())).to_string()])}
                      </ButtonLink>
                    })}
                  </div>
                }
                .into_any()
              })}
              {(cards_due > 0).then(|| {
                view! {
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm">{tp("exam.cards", cards_due, &[&cards_due.to_string()])}</span>
                    <ButtonLink
                      href="/cards"
                      variant=Variant::Default
                      size=Size::Sm
                    >
                      {t("exam.review-cards")}
                    </ButtonLink>
                  </div>
                }
                .into_any()
              })}
              {weakest.map(|(key, name, rate)| {
                let rate_s = format!("{rate:.0}");
                view! {
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm">
                      {tf("exam.weak-spot", &[&crate::data::core_text_tracked(name), rate_s.as_str()])}
                    </span>
                    <ButtonLink
                      href=format!("/practice?bank={bank}&topic={}", js_sys::encode_uri_component(key))
                      variant=Variant::Outline
                      size=Size::Sm
                      title=Signal::derive(move || t("home.lowest-accuracy-topic"))
                    >
                      {t("learning.focused-practice")}
                    </ButtonLink>
                    <ButtonLink
                      href=format!("/exam?bank={bank}&mode=weak")
                      variant=Variant::Outline
                      size=Size::Sm
                      title=Signal::derive(move || t("home.mock-exam-weighted-by"))
                    >
                      {t("exam.weak-area-exam")}
                    </ButtonLink>
                  </div>
                }
                .into_any()
              })}
            </div>
          }
          .into_any()
        }}
      </div>
    </SectionCard>
  }
  .into_any()
}

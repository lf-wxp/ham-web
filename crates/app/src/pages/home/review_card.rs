use ham_web_core::categories::top_category;
use leptos::prelude::*;

use crate::i18n::{bank_class, t, tf};
use crate::pages::load_card_schedule;
use crate::study;
use crate::ui::{
  CARD_HEADER, Size, Variant, button_class, card_class, card_content_class, card_title_class,
};
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
    <div data-slot="card" class=card_class("")>
      <div data-slot="card-header" class=CARD_HEADER>
        <div data-slot="card-title" class=card_title_class("flex items-center justify-between")>
          <span>{t("今日学习队列")}</span>
          <span class="text-sm font-normal text-muted-foreground">
            {move || tf("共 {} 项待复习", &[&total_due.to_string()])}
          </span>
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-2")>
        {move || {
          if total_due == 0 {
            return view! {
              <p class="text-sm text-muted-foreground">
                {t("今天的复习都完成了，可以去练新题或做一套模拟考试。")}
              </p>
            }
            .into_any();
          }
          view! {
            <div class="space-y-2">
              {(mistakes_due > 0).then(|| {
                view! {
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm">{tf("错题 {} 道", &[&mistakes_due.to_string()])}</span>
                    <a href="/mistakes?review=1" class=button_class(Variant::Default, Size::Sm, "")>
                      {t("复习错题")}
                    </a>
                    {(due_in_bank > 0 && due_in_bank < mistakes_due).then(|| view! {
                      <a
                        href=format!("/mistakes?review=1&bank={bank}")
                        class=button_class(Variant::Outline, Size::Sm, "")
                      >
                        {tf("只看 {} 类", &[&(bank_class(&bank.to_string())).to_string()])}
                      </a>
                    })}
                  </div>
                }
                .into_any()
              })}
              {(cards_due > 0).then(|| {
                view! {
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm">{tf("知识卡片 {} 张", &[&cards_due.to_string()])}</span>
                    <a href="/cards" class=button_class(Variant::Default, Size::Sm, "")>
                      {t("复习卡片")}
                    </a>
                  </div>
                }
                .into_any()
              })}
              {weakest.map(|(key, name, rate)| {
                let rate_s = format!("{rate:.0}");
                view! {
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm">
                      {tf("薄弱专项：{}（{}%）", &[name, rate_s.as_str()])}
                    </span>
                    <a
                      href=format!("/practice?bank={bank}&topic={}", js_sys::encode_uri_component(key))
                      class=button_class(Variant::Outline, Size::Sm, "")
                      title=t("正确率最低的分类")
                    >
                      {t("专项练习")}
                    </a>
                    <a
                      href=format!("/exam?bank={bank}&mode=weak")
                      class=button_class(Variant::Outline, Size::Sm, "")
                      title=t("按分类正确率与错题加权抽题的模拟卷")
                    >
                      {t("薄弱项组卷")}
                    </a>
                  </div>
                }
                .into_any()
              })}
            </div>
          }
          .into_any()
        }}
      </div>
    </div>
  }
  .into_any()
}

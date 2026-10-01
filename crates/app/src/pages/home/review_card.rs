use ham_web_core::categories::top_category;
use leptos::prelude::*;

use crate::study;
use crate::ui::{
  CARD_HEADER, Size, Variant, button_class, card_class, card_content_class, card_title_class,
};
use crate::util::now_ms;

/// 今日待复习：错题本按间隔复习到期的题目数，以及正确率最低的分类。
#[component]
pub(super) fn ReviewCard() -> impl IntoView {
  let book = study::load_book();
  if book.records.is_empty() {
    return ().into_any();
  }
  let due = book.due_count(now_ms());
  let total = book.records.len();
  let stats = study::load_stats();
  let bank = stats.main_bank();
  let weakest = stats
    .categories_of(bank)
    .and_then(|c| c.weakest(5))
    .and_then(|(key, t)| {
      let top = top_category(key)?;
      Some((top.key, top.name, t.rate()? * 100.0))
    });
  let bank = bank.unwrap_or_default();
  let due_in_bank = {
    let now = now_ms();
    book
      .records
      .iter()
      .filter(|r| r.is_due(now) && r.in_bank(bank))
      .count()
  };
  view! {
    <div data-slot="card" class=card_class("")>
      <div data-slot="card-header" class=CARD_HEADER>
        <div data-slot="card-title" class=card_title_class("flex items-center justify-between")>
          <span>"今日待复习"</span>
          <span class="text-sm font-normal text-muted-foreground">
            "错题本共 " <span class="font-semibold text-foreground">{total}</span> " 题"
          </span>
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-3")>
        <p class="text-sm text-muted-foreground">
          {if due > 0 {
            format!("有 {due} 道错题到了复习时间，趁记忆还新鲜复习一遍吧。")
          } else {
            "今天的错题都复习完了，到期的题会在这里提醒你。".to_owned()
          }}
        </p>
        <div class="flex flex-wrap gap-2">
          {(due > 0).then(|| view! {
            <a href="/mistakes?review=1" class=button_class(Variant::Default, Size::Sm, "")>
              {format!("复习 {due} 题")}
            </a>
          })}
          {(due_in_bank > 0 && due_in_bank < due).then(|| view! {
            <a href=format!("/mistakes?review=1&bank={bank}") class=button_class(Variant::Outline, Size::Sm, "")>
              {format!("只复习 {bank} 类 {due_in_bank} 题")}
            </a>
          })}
          <a href="/mistakes" class=button_class(Variant::Outline, Size::Sm, "")>"查看错题本"</a>
          {weakest.map(|(key, name, rate)| view! {
            <a
              href=format!("/practice?bank={bank}&topic={}", js_sys::encode_uri_component(key))
              class=button_class(Variant::Outline, Size::Sm, "")
              title="正确率最低的分类"
            >
              {format!("{bank} 类专项：{name}（{rate:.0}%）")}
            </a>
            <a
              href=format!("/exam?bank={bank}&mode=weak")
              class=button_class(Variant::Outline, Size::Sm, "")
              title="按分类正确率与错题加权抽题的模拟卷"
            >
              {format!("{bank} 类薄弱项组卷")}
            </a>
          })}
        </div>
      </div>
    </div>
  }
  .into_any()
}

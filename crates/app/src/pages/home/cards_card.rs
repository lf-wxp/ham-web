//! 首页「知识卡片」：到期卡片数与入口。

use leptos::prelude::*;

use crate::pages::load_card_schedule;
use crate::ui::{
  CARD_HEADER, Size, Variant, button_class, card_class, card_content_class, card_title_class,
};
use crate::util::now_ms;

#[component]
pub(super) fn CardsCard() -> impl IntoView {
  let schedule = load_card_schedule();
  let now = now_ms();
  let due = schedule.due_total(now);
  let learned = schedule.cards.len();
  view! {
    <div data-slot="card" class=card_class("")>
      <div data-slot="card-header" class=CARD_HEADER>
        <div data-slot="card-title" class=card_title_class("flex items-center justify-between")>
          <span>"知识卡片"</span>
          {(learned > 0).then(|| view! {
            <span class="text-sm font-normal text-muted-foreground">
              "已学 " <span class="font-semibold text-foreground">{learned}</span> " 张"
            </span>
          })}
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("flex flex-wrap items-center gap-3")>
        <p class="mr-auto text-sm text-muted-foreground">
          {if learned == 0 {
            "Q 简语、通联缩语、字母解释法、莫尔斯电码、术语，每天几分钟按遗忘规律复习。".to_owned()
          } else if due > 0 {
            format!("有 {due} 张卡片到了复习时间。")
          } else {
            "今天的卡片都复习完了，也可以继续学新卡。".to_owned()
          }}
        </p>
        <a href="/cards" class=button_class(if due > 0 { Variant::Default } else { Variant::Outline }, Size::Sm, "")>
          {if due > 0 { format!("复习 {due} 张") } else if learned == 0 { "开始学习".to_owned() } else { "学新卡".to_owned() }}
        </a>
      </div>
    </div>
  }
}

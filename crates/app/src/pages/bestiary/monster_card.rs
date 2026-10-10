//! 图鉴里的一只怪物 = 一道错题。

use ham_web_core::mistake_book::MistakeRecord;
use leptos::prelude::*;

use crate::i18n::{t, tp};
use crate::icons::PixelSprite;
use crate::rpg::monster_sprite;
use crate::ui::{ButtonLink, Progress, Size, Variant};
use crate::util::encode_uri_component;

const DAY_MS: i64 = 86_400_000;

/// 一只怪物。`top_key` / `top_name` 决定外形与专题标签（无分类码的题归到「其它」）。
#[component]
pub(super) fn MonsterCard(
  record: MistakeRecord,
  top_key: &'static str,
  top_name: &'static str,
  now: i64,
) -> impl IntoView {
  // 怪物生命 = 距离「掌握」还差几次连对。
  let target = record.target_streak();
  let left = target.saturating_sub(record.streak);
  let hp = i64::from(left) * 100 / i64::from(target.max(1));
  let due = record.is_due(now);
  let days = ((record.due_ms - now).max(0) + DAY_MS - 1) / DAY_MS;
  let code = record
    .question
    .j_code()
    .map_or_else(|| top_name.to_owned(), ToOwned::to_owned);
  let href = format!(
    "/battle?mode=revenge&key={}",
    encode_uri_component(&record.key)
  );
  let wrong = record.wrong_count;

  view! {
    <li class="pxl-window flex gap-3 p-3">
      <PixelSprite name=monster_sprite(top_key) scale=4 class="pxl-bob" />
      <div class="min-w-0 flex-1 space-y-1">
        <div class="pxl-label flex flex-wrap gap-x-2 text-xs text-muted-foreground">
          <span>{code}</span>
          <span class="truncate">{move || t(top_name)}</span>
        </div>
        <p class="line-clamp-2 text-sm">{record.question.question.clone()}</p>
        <div class="flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
          <span>{move || tp("learning.wrong-times", wrong, &[&wrong.to_string()])}</span>
          <span class=if due { "text-hp-text" } else { "" }>
            {move || {
              if due {
                t("rpg.monster-due")
              } else {
                tp("rpg.monster-wait", days, &[&days.to_string()])
              }
            }}
          </span>
        </div>
        <Progress value=hp label=Signal::derive(move || t("rpg.enemy-hp-label")) class="pxl-bar-hp h-3" />
        <div class="flex items-center justify-between gap-2">
          <span class="text-xs tabular-nums text-muted-foreground">
            {move || tp("rpg.monster-hits-left", u32::from(left), &[&left.to_string()])}
          </span>
          <ButtonLink href=href variant=Variant::Outline size=Size::Sm>
            {move || t("rpg.monster-challenge")}
          </ButtonLink>
        </div>
      </div>
    </li>
  }
}

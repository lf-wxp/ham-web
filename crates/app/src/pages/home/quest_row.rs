//! 基地任务栏里的一行任务：图标 + 标题 + 说明 + 奖励 / 完成标记。

use leptos::prelude::*;

use crate::i18n::t;
use crate::icons::{Icon, IconKind, PixelSprite};

/// 一行任务。整行是一个链接：点哪里都能出发。
#[component]
pub(super) fn QuestRow(
  #[prop(into)] href: String,
  sprite: &'static str,
  #[prop(into)] title: Signal<String>,
  #[prop(into)] hint: Signal<String>,
  /// 奖励说明（如 `+50 XP`）；任务已完成时不显示。
  #[prop(into)]
  reward: Signal<String>,
  done: bool,
) -> impl IntoView {
  view! {
    <li>
      <a
        href=href
        class="pxl-window motion-press flex h-full items-start gap-3 p-3 hover:bg-accent"
      >
        <PixelSprite name=sprite scale=3 class=if done { "opacity-50 grayscale" } else { "pxl-bob" } />
        // 奖励与标题同一行、贴着右边：跟说明文字挤在一行时，说明会被奖励截成两段。
        <span class="flex min-w-0 flex-1 flex-col gap-1">
          <span class="flex flex-wrap items-baseline justify-between gap-x-2">
            <span class="text-sm">{move || title.get()}</span>
            {if done {
              view! {
                <span class="pxl-label flex items-center gap-1 text-xs text-win-text">
                  <Icon kind=IconKind::Check class="size-6" />
                  {move || t("rpg.done-tag")}
                </span>
              }
                .into_any()
            } else {
              view! {
                <span class="pxl-label text-xs text-gold-text">{move || reward.get()}</span>
              }
                .into_any()
            }}
          </span>
          <span class="text-xs text-muted-foreground">{move || hint.get()}</span>
        </span>
      </a>
    </li>
  }
}

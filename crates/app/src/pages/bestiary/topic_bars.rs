//! 各专题的在世怪物数量条形图：一眼看出最该复仇的区域。

use leptos::prelude::*;

use crate::i18n::{t, tp};
use crate::ui::Progress;

/// `(专题名, 在世怪物数)`，已按数量从多到少排好。
#[component]
pub(super) fn TopicBars(rows: Vec<(&'static str, usize)>) -> impl IntoView {
  let max = rows.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1);
  view! {
    <section class="pxl-window space-y-2 p-4" aria-labelledby="topic-bars-title">
      <h2 id="topic-bars-title" class="pxl-title text-sm">{move || t("rpg.weak-topics")}</h2>
      <ul class="space-y-2">
        {rows
          .into_iter()
          .map(|(name, count)| {
            let value = i64::try_from(count * 100 / max).unwrap_or(100);
            view! {
              <li class="grid grid-cols-[minmax(0,9rem)_1fr_auto] items-center gap-3 text-sm">
                <span class="truncate">{move || t(name)}</span>
                <Progress value=value label=Signal::derive(move || t(name)) class="pxl-bar-hp h-3" />
                <span class="tabular-nums text-xs text-muted-foreground">
                  {move || tp("rpg.monsters-count", count, &[&count.to_string()])}
                </span>
              </li>
            }
          })
          .collect_view()}
      </ul>
    </section>
  }
}

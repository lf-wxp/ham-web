//! `history_bars`：从 `study_plan_card.rs` 拆出的视图构造函数（一个组件一个文件）。

use leptos::prelude::*;

use crate::i18n::{t, tp};
use crate::study;

/// 最近 14 天每日作答量柱状图。
pub(super) fn history_bars(today: &str) -> impl IntoView {
  let days = study::load_daily().recent(today, 14);
  let max = days
    .iter()
    .map(|(_, t)| t.answered)
    .max()
    .unwrap_or(0)
    .max(1);
  let total: u32 = days.iter().map(|(_, t)| t.answered).sum();
  view! {
    <div>
      <div class="mb-1.5 flex items-center justify-between text-xs text-muted-foreground">
        <span>{move || t("common.last-14-days")}</span>
        <span class="tabular-nums">{tp("common.questions-3", total, &[&total.to_string()])}</span>
      </div>
      <div class="flex h-16 items-end gap-1" role="img" aria-label=tp("common.questions-answered-in-the", total, &[&total.to_string()])>
        {days
          .into_iter()
          .map(|(day, t)| {
            let h = f64::from(t.answered) / f64::from(max) * 100.0;
            view! {
              <div
                class="flex-1 rounded-t bg-primary/70"
                style=format!("height: max({h:.1}%, 2px)")
                title=tp(
                  "common.questions-new",
                  t.answered,
                  &[&day[5..], &t.answered.to_string(), &t.new.to_string()],
                )
              ></div>
            }
          })
          .collect_view()}
      </div>
    </div>
  }
}

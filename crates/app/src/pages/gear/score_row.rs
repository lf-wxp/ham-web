//! 一行「加权得分」：分数 + 缺了哪几条轴。

use ham_web_core::gear_score::Score;
use leptos::prelude::*;

use crate::i18n::{t, tf};

use super::shared::axis_label;

#[component]
pub(super) fn ScoreRow(
  label: String,
  score: Score,
  /// 这台机器的得分用到了用户自己测的值（界面据此标一下，免得分数变了没人知道为什么）。
  #[prop(optional)]
  user_based: bool,
) -> impl IntoView {
  let missing: Vec<String> = score.missing.iter().copied().map(axis_label).collect();
  let insufficient = score.is_insufficient();
  view! {
    <div class="flex flex-col gap-1 border-t py-2 first:border-t-0">
      <div class="flex items-baseline justify-between gap-2">
        <span class="truncate text-sm">{label}</span>
        {if insufficient {
          view! {
            <span class="shrink-0 rounded-full border px-2 py-0.5 text-[10px] text-muted-foreground">
              {move || t("knowledge.score-insufficient")}
            </span>
          }
            .into_any()
        } else {
          let points = score.points.unwrap_or_default();
          view! {
            <span class="shrink-0 font-mono text-sm font-semibold text-primary">
              {format!("{points:.0}")}
            </span>
          }
            .into_any()
        }}
      </div>
      {user_based
        .then(|| {
          view! {
            <p class="text-[11px] text-primary">
              {move || t("knowledge.score-uses-your-values")}
            </p>
          }
        })}
      {insufficient
        .then(|| {
          view! {
            <p class="text-[11px] text-muted-foreground">
              {move || t("knowledge.score-insufficient-hint")}
            </p>
          }
        })}
      {(!missing.is_empty())
        .then(|| {
          let list = missing.join(" · ");
          view! {
            <p class="text-[11px] text-muted-foreground">
              {tf("knowledge.score-missing", &[&list])}
            </p>
          }
        })}
    </div>
  }
}

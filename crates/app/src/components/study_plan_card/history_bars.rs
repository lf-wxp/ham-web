//! `history_bars`：从 `study_plan_card.rs` 拆出的视图构造函数（一个组件一个文件）。

use leptos::prelude::*;

use crate::i18n::{t, tp};
use crate::study;
use crate::ui::Tooltip;

/// 最近 14 天每日作答量柱状图。
///
/// `+ use<>`（Rust 2024 的精确捕获）：`today` 只在函数体里用来取数据，返回的视图不借用它。
/// 不写这一句的话，`impl IntoView` 会把入参的生命周期带进返回类型，调用点
/// `editable.then(|| history_bars(&today))` 就要求 `&today` 是 `'static`（host 的 test 构建报 E0597）。
pub(super) fn history_bars(today: &str) -> impl IntoView + use<> {
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
        <span class="tabular-nums">{tp("common.questions-total", total, &[&total.to_string()])}</span>
      </div>
      <div class="flex h-16 items-end gap-1" role="img" aria-label=tp("common.questions-answered-in-the", total, &[&total.to_string()])>
        {days
          .into_iter()
          .map(|(day, t)| {
            let h = f64::from(t.answered) / f64::from(max) * 100.0;
            let tip = tp(
              "common.questions-new",
              t.answered,
              &[&day[5..], &t.answered.to_string(), &t.new.to_string()],
            );
            view! {
              // 柱的百分比高度相对 `h-16` 的 flex 容器；包一层后 span 用 `self-stretch`
              // 撑到容器高度，包含块才有确定高度（父级已带整图 role="img" 名称，
              // 逐柱明细不再加 tabindex，避免 14 个停靠点）。
              <Tooltip class="flex flex-1 items-end self-stretch" label=tip>
                <div
                  class="w-full bg-primary/70"
                  style=format!("height: max({h:.1}%, 2px)")
                ></div>
              </Tooltip>
            }
          })
          .collect_view()}
      </div>
    </div>
  }
}

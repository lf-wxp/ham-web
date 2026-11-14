//! 首页主显示屏里的「值机记录」：近 28 天每天一格，深浅 = 当天作答量。
//!
//! 数据与分档同现代风首页的「信号瀑布」（`study-daily` + `study_plan::intensity_levels`），
//! 只是窗口拉长到四周、画在屏幕层上：一眼看出这一个月哪几天开过台。

use ham_web_core::study_plan::intensity_levels;
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};
use crate::study;
use crate::util::local_today;

/// 窗口天数：四周，正好排满一行 28 格（320px 的屏幕里每格仍有 6px）。
const DAYS: usize = 28;

#[component]
pub(super) fn LogStrip() -> impl IntoView {
  let today = local_today();
  let days = study::load_daily().recent(&today, DAYS);
  let total: usize = days.iter().map(|(_, tally)| tally.answered as usize).sum();
  // 按窗口内的最大值归一（不为了给几格上色去异步加载题库算「今日目标」）：表达的是「哪天亮、哪天暗」。
  let levels = intensity_levels(days.iter().map(|(_, tally)| tally.answered), None);

  view! {
    <div class="space-y-2 border-t-2 border-dashed border-(--display-rim) pt-3">
      <div class="flex flex-wrap items-center justify-between gap-2 text-xs">
        <span class="pxl-label pxl-d-dim">{move || t("home.log-strip-title")}</span>
        <span class="pxl-d-dim tabular-nums">
          {move || tp("home.log-strip-total", total, &[&total.to_string()])}
        </span>
      </div>
      // 格子里不写字：明细走 `title`，读屏靠整条的 `aria-label`（与现代风的瀑布同一口径）。
      <div class="pxl-logstrip" role="img" aria-label=move || t("home.log-strip-summary")>
        {days
          .into_iter()
          .zip(levels)
          .map(|((day, tally), level)| {
            let stamp = day.get(5..).unwrap_or(&day).to_owned();
            let title = tf("home.waterfall-cell", &[&stamp, &tally.answered.to_string()]);
            // 亮度档 → 屏幕读数色混入屏幕底的百分比（样式在 `radio.css` 的 `.pxl-logstrip`）；0 档不写。
            let mix = [0, 32, 50, 68, 84, 100].get(usize::from(level)).copied().unwrap_or(0);
            view! {
              <span
                data-level=level.to_string()
                data-today=(day == today).then_some("")
                style=(mix > 0).then(|| format!("--mix: {mix}%"))
                title=title
              ></span>
            }
          })
          .collect_view()}
      </div>
      {(total == 0).then(|| view! { <p class="pxl-d-dim text-xs">{move || t("home.waterfall-empty")}</p> })}
    </div>
  }
}

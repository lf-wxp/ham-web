use leptos::prelude::*;

use super::{MorseStats, export_all_morse_stats};
use crate::i18n::{t, tf};

/// 解码练习的统计面板：对错计数、正确率、连对与近 7 天正确率。
#[component]
pub(super) fn MorseStatsPanel(
  stats: RwSignal<MorseStats>,
  on_reset: Callback<()>,
) -> impl IntoView {
  let rate = move || {
    let s = stats.get();
    let total = s.correct + s.wrong;
    if total == 0 {
      0.0
    } else {
      s.correct as f64 / total as f64 * 100.0
    }
  };

  view! {
          <div class="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
            <span>
              {move || t("正确 ")} <span class="font-semibold tabular-nums text-foreground">{move || stats.get().correct}</span>
            </span>
            <span>
              {move || t("错误 ")} <span class="font-semibold tabular-nums text-foreground">{move || stats.get().wrong}</span>
            </span>
            <span>
              {move || t("正确率 ")} <span class="font-semibold tabular-nums text-foreground">{move || format!("{:.0}%", rate())}</span>
            </span>
            <span>
              {move || t("连对 ")} <span class="font-semibold tabular-nums text-foreground">{move || stats.get().streak}</span>
            </span>
            <span>
              {move || t("今日 ")} <span class="font-semibold tabular-nums text-foreground">{move || stats.get().today_correct}</span>
              " / "
              <span class="font-semibold tabular-nums text-foreground">{move || stats.get().today_wrong}</span>
            </span>
            {move || {
              (stats.get().top_wpm > 0.0).then(|| {
                view! {
                  <span>
                    {move || t("最高 ")} <span class="font-semibold tabular-nums text-foreground">{format!("{:.0}", stats.get().top_wpm)}</span> " WPM"
                  </span>
                }
              })
            }}
            {move || {
              (stats.get().best_streak > 0).then(|| {
                view! {
                  <span>
                    {move || t("最长 ")} <span class="font-semibold tabular-nums text-foreground">{stats.get().best_streak}</span>
                  </span>
                }
              })
            }}
            {move || {
              let weak: Vec<String> = stats
                .get()
                .mistakes
                .iter()
                .filter(|(_, n)| **n >= 2)
                .map(|(c, _)| c.clone())
                .collect();
              (!weak.is_empty()).then(|| {
                view! {
                  <span>
                    {move || t("易错：")} <span class="font-mono font-semibold text-foreground">{weak.join(" ")}</span>
                  </span>
                }
              })
            }}
            <button
              type="button"
              on:click=move |_| on_reset.run(())
              class="rounded-md px-2 py-0.5 transition-all duration-200 hover:bg-accent hover:text-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
            >
              {move || t("重置")}
            </button>
            <button
              type="button"
              on:click=move |_| export_all_morse_stats()
              class="rounded-md px-2 py-0.5 transition-all duration-200 hover:bg-accent hover:text-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
            >
              {move || t("导出")}
            </button>
          </div>

          {move || {
            let s = stats.get();
            let mut days: Vec<(String, usize, usize)> = s
              .daily
              .iter()
              .map(|(d, &(c, w))| (d.clone(), c, w))
              .collect();
            if !s.today.is_empty() {
              days.push((s.today.clone(), s.today_correct, s.today_wrong));
            }
            days.sort();
            let start = days.len().saturating_sub(7);
            let days = days[start..].to_vec();
            (!days.is_empty()).then(|| {
              view! {
                <div>
                  <div class="mb-1.5 text-xs font-medium text-muted-foreground">{move || t("近 7 天正确率")}</div>
                  <div class="flex items-end gap-1.5">
                    {days
                      .into_iter()
                      .map(|(d, c, w)| {
                        let total = c + w;
                        let pct = if total == 0 { 0.0 } else { c as f64 / total as f64 * 100.0 };
                        let day = d.chars().skip(5).collect::<String>();
                        view! {
                          <div class="flex flex-col items-center gap-1" title=tf("{}：对 {} 错 {}", &[&(d).to_string(), &(c).to_string(), &(w).to_string()])>
                            <div class="flex h-12 w-6 items-end overflow-hidden rounded bg-muted">
                              <div class="w-full bg-primary" style=format!("height: {pct:.0}%")></div>
                            </div>
                            <span class="text-[10px] tabular-nums text-muted-foreground">{day}</span>
                          </div>
                        }
                      })
                      .collect_view()}
                  </div>
                </div>
              }
            })
          }}

  }
}

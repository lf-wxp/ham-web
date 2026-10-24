use leptos::prelude::*;

use super::{MorseStats, export_all_morse_stats};
use crate::i18n::{t, tf};
use crate::ui::Tooltip;

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
              {move || t("common.correct-label")} <span class="tabular-nums text-foreground">{move || stats.get().correct}</span>
            </span>
            <span>
              {move || t("morse.wrong")} <span class="tabular-nums text-foreground">{move || stats.get().wrong}</span>
            </span>
            <span>
              {move || t("morse.accuracy")} <span class="tabular-nums text-foreground">{move || format!("{:.0}%", rate())}</span>
            </span>
            <span>
              {move || t("morse.streak")} <span class="tabular-nums text-foreground">{move || stats.get().streak}</span>
            </span>
            <span>
              {move || t("morse.today")} <span class="tabular-nums text-foreground">{move || stats.get().today_correct}</span>
              " / "
              <span class="tabular-nums text-foreground">{move || stats.get().today_wrong}</span>
            </span>
            {move || {
              (stats.get().top_wpm > 0.0).then(|| {
                view! {
                  <span>
                    {move || t("morse.best")} <span class="tabular-nums text-foreground">{format!("{:.0}", stats.get().top_wpm)}</span> " WPM"
                  </span>
                }
              })
            }}
            {move || {
              (stats.get().best_streak > 0).then(|| {
                view! {
                  <span>
                    {move || t("morse.best-streak")} <span class="tabular-nums text-foreground">{stats.get().best_streak}</span>
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
                    {move || t("morse.frequent-misses-label")} <span class="font-mono text-foreground">{weak.join(" ")}</span>
                  </span>
                }
              })
            }}
            <button
              type="button"
              on:click=move |_| on_reset.run(())
              class="px-2 py-0.5 transition duration-(--motion-normal) hover:bg-accent hover:text-foreground"
            >
              {move || t("morse.reset")}
            </button>
            <button
              type="button"
              on:click=move |_| export_all_morse_stats()
              class="px-2 py-0.5 transition duration-(--motion-normal) hover:bg-accent hover:text-foreground"
            >
              {move || t("learning.export")}
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
                  <div class="mb-1.5 text-xs text-muted-foreground">{move || t("morse.accuracy-over-the-last")}</div>
                  <div class="flex items-end gap-1.5">
                    {days
                      .into_iter()
                      .map(|(d, c, w)| {
                        let total = c + w;
                        let pct = if total == 0 { 0.0 } else { c as f64 / total as f64 * 100.0 };
                        let day = d.chars().skip(5).collect::<String>();
                        let tip = tf("common.right-wrong", &[&(d).to_string(), &(c).to_string(), &(w).to_string()]);
                        view! {
                          <Tooltip label=tip.clone()>
                            <div
                              class="flex flex-col items-center gap-1 outline-none focus-visible:outline-2 focus-visible:outline-dashed focus-visible:outline-ring"
                              tabindex="0"
                              role="img"
                              aria-label=tip
                            >
                              <div class="flex h-12 w-6 items-end overflow-hidden bg-muted">
                                <div class="w-full bg-primary" style=format!("height: {pct:.0}%")></div>
                              </div>
                              <span class="text-xs tabular-nums text-muted-foreground">{day}</span>
                            </div>
                          </Tooltip>
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

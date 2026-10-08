use std::collections::HashMap;

use ham_web_core::study_plan::{day_number, format_day};
use leptos::prelude::*;

use crate::i18n::{t, tp};

/// 按每日作答量返回色阶类名（完整字面量，供 Tailwind 扫描）。
fn level(count: u32) -> &'static str {
  if count == 0 {
    "bg-muted/50"
  } else if count < 5 {
    "bg-primary/20"
  } else if count < 15 {
    "bg-primary/40"
  } else if count < 30 {
    "bg-primary/70"
  } else {
    "bg-primary"
  }
}

const WEEKDAYS: [&str; 7] = ["日", "一", "二", "三", "四", "五", "六"];

/// GitHub 风格的每日学习打卡热力图：最近 13 周，按周 × 7 天排列。
/// `days` 为「日期 → 作答量」，`today` 为今天（`YYYY-MM-DD`）。
#[component]
pub fn StudyHeatmap(days: HashMap<String, u32>, today: String) -> impl IntoView {
  const WEEKS: usize = 13;
  let today_n = day_number(&today).unwrap_or(0);
  // 1970-01-01 是周四，因此周日 weekday=0 时 day_number ≡ -4 (mod 7)。
  let weekday = ((today_n + 4) % 7 + 7) % 7;
  let sunday = today_n - weekday;
  let first = sunday - (WEEKS as i64 - 1) * 7;

  let mut grid: Vec<Vec<(String, u32)>> = Vec::with_capacity(WEEKS);
  for w in 0..WEEKS {
    let mut row = Vec::with_capacity(7);
    for d in 0..7i64 {
      let n = first + w as i64 * 7 + d;
      let day = format_day(n);
      let c = days.get(&day).copied().unwrap_or(0);
      row.push((day, c));
    }
    grid.push(row);
  }

  view! {
    <div class="space-y-1.5">
      <div class="flex items-center gap-1">
        <span class="w-8 shrink-0"></span>
        <div class="flex flex-1 gap-px">
          {WEEKDAYS
            .iter()
            .map(|wd| {
              view! {
                <div class="flex-1 text-center text-[9px] leading-none text-muted-foreground">
                  {move || t(wd)}
                </div>
              }
            })
            .collect_view()}
        </div>
      </div>
      {grid
        .into_iter()
        .map(|row| {
          let month = row[0].0.get(5..7).unwrap_or("").to_owned();
          view! {
            <div class="flex items-center gap-1">
              <span class="w-8 shrink-0 text-right text-[10px] tabular-nums text-muted-foreground">
                {month}
              </span>
              <div class="flex flex-1 gap-px">
                {row
                  .into_iter()
                  .map(|(day, c)| {
                    view! {
                      <div
                        class=format!("h-4 flex-1 rounded-sm {}", level(c))
                        title=tp("learning.answered", c, &[&day, &c.to_string()])
                      ></div>
                    }
                  })
                  .collect_view()}
              </div>
            </div>
          }
        })
        .collect_view()}
      <div class="flex items-center justify-end gap-1.5 text-[10px] text-muted-foreground">
        <span>{t("learning.less")}</span>
        <span class="h-3 w-3 rounded-sm bg-muted/50"></span>
        <span class="h-3 w-3 rounded-sm bg-primary/20"></span>
        <span class="h-3 w-3 rounded-sm bg-primary/40"></span>
        <span class="h-3 w-3 rounded-sm bg-primary/70"></span>
        <span class="h-3 w-3 rounded-sm bg-primary"></span>
        <span>{t("learning.more")}</span>
      </div>
    </div>
  }
}

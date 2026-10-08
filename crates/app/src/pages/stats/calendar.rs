//! 日历 QSO 热力图（GitHub 贡献图风格）：横轴为周、纵轴为周日～周六，
//! 颜色越深表示当天通联越多。

use std::collections::BTreeMap;

use crate::i18n::{t, tp};
use leptos::prelude::*;
use wasm_bindgen::JsValue;

/// 一天的毫秒数（用于回推日期；夏令时切换日的 1 小时偏差不影响按天着色）。
const DAY_MS: f64 = 86_400_000.0;

/// 毫秒时间戳 → 本地日期 key（`YYYY-MM-DD`）。
fn date_key_from_ms(ms: f64) -> String {
  let d = js_sys::Date::new(&JsValue::from_f64(ms));
  let y = d.get_full_year();
  let m = d.get_month() + 1;
  let day = d.get_date();
  format!("{y:04}-{m:02}-{day:02}")
}

/// 根据计数值返回绿色色阶（完整字面量，供 Tailwind 扫描）。
fn cell_class(count: u32, max: u32) -> &'static str {
  if count == 0 {
    return "bg-muted/60";
  }
  let ratio = count as f64 / max as f64;
  if ratio >= 0.8 {
    "bg-emerald-600"
  } else if ratio >= 0.6 {
    "bg-emerald-500"
  } else if ratio >= 0.4 {
    "bg-emerald-400"
  } else if ratio >= 0.2 {
    "bg-emerald-300"
  } else {
    "bg-emerald-200"
  }
}

/// 最近 26 周（半年）的日历热力图，`counts` 为 `日期 -> QSO 数`。
#[component]
pub(super) fn CalendarHeatmap(counts: BTreeMap<String, u32>) -> impl IntoView {
  const WEEKS: i32 = 26;

  let today = js_sys::Date::new_0();
  let today_key = date_key_from_ms(today.get_time());
  let today_dow = today.get_day() as i32; // 0=周日

  // 今天本地 0 点，再回推到「今天所在周的周日」并往前 WEEKS-1 周。
  let today_midnight = js_sys::Date::new_with_year_month_day(
    today.get_full_year(),
    today.get_month() as i32,
    today.get_date() as i32,
  );
  let start_ms =
    today_midnight.get_time() - today_dow as f64 * DAY_MS - (WEEKS - 1) as f64 * 7.0 * DAY_MS;

  let max = counts.values().copied().max().unwrap_or(0).max(1);

  // 预生成网格：外层为周（列），内层为周日～周六（行）。
  let mut grid: Vec<Vec<(String, u32, bool)>> = Vec::with_capacity(WEEKS as usize);
  for w in 0..WEEKS {
    let mut week = Vec::with_capacity(7);
    for d in 0..7 {
      let ms = start_ms + (w * 7 + d) as f64 * DAY_MS;
      let key = date_key_from_ms(ms);
      let count = counts.get(&key).copied().unwrap_or(0);
      let future = key.as_str() > today_key.as_str();
      week.push((key, count, future));
    }
    grid.push(week);
  }

  view! {
    <div class="space-y-2">
      <div class="flex items-center justify-end gap-1 text-[10px] text-muted-foreground">
        <span>{move || t("learning.less")}</span>
        <span class="h-3 w-3 rounded-sm bg-muted/60"></span>
        <span class="h-3 w-3 rounded-sm bg-emerald-200"></span>
        <span class="h-3 w-3 rounded-sm bg-emerald-300"></span>
        <span class="h-3 w-3 rounded-sm bg-emerald-400"></span>
        <span class="h-3 w-3 rounded-sm bg-emerald-500"></span>
        <span class="h-3 w-3 rounded-sm bg-emerald-600"></span>
        <span>{move || t("learning.more")}</span>
      </div>
      <div class="overflow-x-auto">
        <div class="flex gap-[3px]">
          {grid
            .into_iter()
            .map(|week| {
              view! {
                <div class="flex flex-col gap-[3px]">
                  {week
                    .into_iter()
                    .map(|(key, count, future)| {
                      let cls = if future {
                        "h-3 w-3 rounded-sm bg-transparent"
                      } else {
                        cell_class(count, max)
                      };
                      view! {
                        <div class=format!("h-3 w-3 rounded-sm {cls}") title=tp("common.records", count, &[&(key).to_string(), &(count).to_string()])></div>
                      }
                    })
                    .collect_view()}
                </div>
              }
            })
            .collect_view()}
        </div>
      </div>
    </div>
  }
}

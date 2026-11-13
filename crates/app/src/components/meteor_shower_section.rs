//! 流星雨时刻表：IMO 2026 年历表 5 的数值，按峰值倒计时排列。
//!
//! 数据与「列哪几行、怎么排」都在 `crates/core/src/meteor_showers.rs`（出处与许可见该模块文档，
//! 筛选与排序有单测）。本卡片只做**呈现**：峰值日期按当前 UTC 日期算下一次，活动期打标，
//! ZHR 给数值或「可变」。峰值日只对 2026 年精确，卡片底部写明这一点，不假装是长期历表。
//!
//! `today` 由调用方在挂载时取一次（与 `/events` 页的其它块同口径）：页面开着跨过 UTC 午夜不会自己刷新，
//! 刷新页面即可。

use ham_web_core::meteor_showers::{TimetableEntry, WINDOW_DAYS, timetable};
use leptos::prelude::*;

use crate::i18n::{t, tf};

/// 比较两个名字是否「写法相同」：去掉连字符与空格、忽略大小写。
fn squash(name: &str) -> String {
  name
    .chars()
    .filter(|c| !matches!(c, '-' | ' '))
    .flat_map(char::to_lowercase)
    .collect()
}

/// 峰值那一列的文案：今天、已过、或还有几天。
fn peak_text(e: &TimetableEntry) -> String {
  let (m, d) = e.shower.peak;
  let date = tf("knowledge.month-day", &[&m.to_string(), &d.to_string()]);
  if e.days == 0 {
    tf("knowledge.peak-today", &[&date])
  } else if e.peak_passed {
    // 峰值已过、活动期还没结束：下一次是明年，写「还有 362 天」会被读成一年后才峰值。
    tf("knowledge.peak-passed", &[&date])
  } else {
    tf("knowledge.peak-in-days", &[&date, &e.days.to_string()])
  }
}

/// 流星雨卡片：`today` 为当前 UTC 日期。
#[component]
pub fn MeteorShowerSection(today: (i32, u32, u32)) -> impl IntoView {
  // 只挑「正在活动」或「30 天内到峰值」的，避免一年 37 行全铺开。
  let rows = timetable(today, WINDOW_DAYS);

  view! {
    <section class=crate::kit::card_class()>
      <div class="flex flex-wrap items-baseline justify-between gap-2 px-4 pt-4">
        <h2 class=crate::kit::card_title_class()>
          {move || t("knowledge.meteor-showers-title")}
        </h2>
        <span class="text-xs text-muted-foreground">{move || t("knowledge.imo-source")}</span>
      </div>
      {rows.is_empty().then(|| {
        view! {
          <p class="px-4 py-3 text-sm text-muted-foreground">
            {move || tf("knowledge.no-meteor-showers-in-window", &[&WINDOW_DAYS.to_string()])}
          </p>
        }
      })}
      <div class="divide-y">
        {rows
          .iter()
          .map(|e| {
            let s = e.shower;
            let entry = *e;
            view! {
              <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1 px-4 py-3">
                <span class="text-sm">{move || crate::data::core_text_tracked(s.name)}</span>
                // IMO 的英文名当国际通用名一直留着；译名与它写法相同（忽略连字符 / 空格 / 大小写）时不重复显示。
                {move || {
                  let shown = crate::data::core_text_tracked(s.name);
                  (squash(&shown) != squash(s.name_en))
                    .then(|| view! { <span class="text-xs text-muted-foreground">{s.name_en}</span> })
                }}
                {entry
                  .active
                  .then(|| {
                    view! {
                      <span class=crate::kit::tile_class("px-2 py-0.5 text-xs")>
                        {move || t("knowledge.active-now")}
                      </span>
                    }
                  })}
                <span class="ml-auto text-xs tabular-nums text-muted-foreground">
                  {move || peak_text(&entry)}
                </span>
                <span class="w-20 text-right text-xs tabular-nums text-muted-foreground">
                  {move || match s.zhr {
                    Some(z) => tf("knowledge.zhr-value", &[&format!("{z:.0}")]),
                    None => t("knowledge.zhr-variable"),
                  }}
                </span>
              </div>
            }
          })
          .collect_view()}
      </div>
      <p class="px-4 py-3 text-xs text-muted-foreground">
        {move || t("knowledge.peak-dates-are-accurate-only")}
      </p>
    </section>
  }
}

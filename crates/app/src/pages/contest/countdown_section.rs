use std::time::Duration;

use crate::i18n::{t, tf};
use leptos::prelude::*;

/// 未来赛事（2026 赛季，UTC 起始日期）。
const UPCOMING: &[(&str, u32, u32, &str)] = &[
  ("CQ WPX RTTY", 2, 14, "RTTY 模式，交换序号"),
  ("ARRL DX CW", 2, 21, "仅与 DXCC 实体通联"),
  ("CQ WPX SSB", 3, 28, "按前缀数量计分"),
  ("CQ WPX CW", 5, 30, "CW 模式，按前缀计分"),
  ("ARRL Field Day", 6, 27, "野外应急设台 24 小时"),
  ("IARU HF", 7, 11, "交换 ITU 分区"),
  ("CQ WW RTTY", 9, 26, "RTTY 全球大赛"),
  ("CQ WW SSB", 10, 24, "全球最大 SSB 竞赛"),
  ("CQ WW CW", 11, 28, "全球最大 CW 竞赛"),
  ("ARRL 十米", 12, 12, "仅限 10 米波段"),
];

/// 未来赛事倒计时（UTC），每分钟刷新。
#[component]
pub(super) fn CountdownSection() -> impl IntoView {
  let now = RwSignal::new(js_sys::Date::new_0().get_time());
  if let Ok(handle) = set_interval_with_handle(
    move || now.set(js_sys::Date::new_0().get_time()),
    Duration::from_secs(60),
  ) {
    on_cleanup(move || handle.clear());
  }

  let year = js_sys::Date::new_0().get_utc_full_year() as f64;

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("contest.upcoming-contest-countdown-utc")}</h2>
      <div class="divide-y">
        {UPCOMING
          .iter()
          .map(|&(name, m, d, desc)| {
            let month_start = js_sys::Date::utc(year, (m - 1) as f64);
            let target = month_start + (d as f64 - 1.0) * 86_400_000.0;
            view! {
              <div class="grid gap-1 px-4 py-3 sm:grid-cols-[10rem_1fr_9rem]">
                <div class="font-medium">{move || t(name)}</div>
                <div class="text-sm text-muted-foreground">{tf("contest.entry", &[&m.to_string(), &d.to_string(), &(t(desc)).to_string()])}</div>
                <div class="text-right text-sm font-medium tabular-nums text-primary">
                  {move || {
                    let left = target - now.get();
                    if left <= 0.0 {
                      t("contest.started")
                    } else {
                      let days = (left / 86_400_000.0).floor();
                      let hours = ((left % 86_400_000.0) / 3_600_000.0).floor();
                      tf(
                        "contest.d-h",
                        &[&format!("{days:.0}"), &format!("{hours:.0}")],
                      )
                    }
                  }}
                </div>
              </div>
            }
          })
          .collect_view()}
      </div>
      <p class="px-4 py-3 text-xs text-muted-foreground">
        {move || t("contest.dates-are-estimated-from")}
      </p>
    </section>
  }
}

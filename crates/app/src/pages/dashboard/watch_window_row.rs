//! 「现在守哪一段」里的一行窗口：波段、功率上限、模式分段、需协调标记与（有预测时的）可靠度条。

use ham_web_core::watch_windows::Window;
use leptos::prelude::*;

use crate::i18n::t;

/// 功率上限的文字：`≤ 25 W`、`≤ 1000 W`（两个数字语言中立，不进词典）。
fn power_text(w: f64) -> String {
  format!("≤ {} W", w as u32)
}

/// 一行窗口。
#[component]
pub(super) fn WatchWindowRow(w: Window) -> impl IntoView {
  let band = w.band;
  let percent = w.forecast.map(|f| (f.reliability * 100.0).round() as u32);
  let usable = w.forecast.is_some_and(|f| f.usable);
  view! {
    <li
      class="flex flex-wrap items-center gap-x-3 gap-y-1 bg-inset px-3 py-2 text-xs"
      data-window=band
      data-reliability=percent.map(|p| p.to_string()).unwrap_or_default()
    >
      <span class="w-12 shrink-0 font-mono text-sm">{band}</span>
      <span class="font-mono tabular-nums text-muted-foreground">{power_text(w.power_w)}</span>
      <span class="flex flex-wrap gap-1 text-muted-foreground">
        {w.modes.cw.then(|| view! { <span class="border px-1.5 py-0.5">{t("radio.watch-mode-cw")}</span> })}
        {w.modes.narrow.then(|| view! { <span class="border px-1.5 py-0.5">{t("radio.watch-mode-data")}</span> })}
        {w.modes.phone.then(|| view! { <span class="border px-1.5 py-0.5">{t("radio.watch-mode-phone")}</span> })}
        {w.needs_coordination.then(|| view! { <span class="border px-1.5 py-0.5">{t("radio.watch-coordination")}</span> })}
      </span>
      {percent.map(|p| {
        view! {
          <span class="ml-auto flex items-center gap-2" data-usable=usable.to_string()>
            <span class="h-1.5 w-20 bg-muted" aria-hidden="true">
              <span class="block h-full bg-primary" style=format!("width: {p}%")></span>
            </span>
            <span class="w-9 text-right font-mono tabular-nums">{format!("{p}%")}</span>
          </span>
        }
      })}
    </li>
  }
}

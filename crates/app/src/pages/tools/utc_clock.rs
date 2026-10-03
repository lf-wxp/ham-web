//! UTC / Zulu 时间：实时 UTC 时钟与常用时区对照。

use std::time::Duration;

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::i18n::t;

/// 常用时区（名称，UTC 偏移小时，说明）。
const TIMEZONES: &[(&str, f64, &str)] = &[
  ("北京", 8.0, "中国标准时间 CST"),
  ("东京", 9.0, "日本标准时间 JST"),
  ("伦敦", 0.0, "格林尼治标准时间 GMT"),
  ("纽约", -5.0, "美国东部时间 EST（夏令时 -4）"),
  ("洛杉矶", -8.0, "美国太平洋时间 PST（夏令时 -7）"),
  ("悉尼", 10.0, "澳大利亚东部时间 AEST"),
];

/// 由 UTC 毫秒时间戳换算某时区的 HH:MM:SS。
fn zoned(utc_ms: f64, offset_hours: f64) -> String {
  let shifted = utc_ms + offset_hours * 3_600_000.0;
  let d = js_sys::Date::new(&JsValue::from_f64(shifted));
  format!(
    "{:02}:{:02}:{:02}",
    d.get_utc_hours(),
    d.get_utc_minutes(),
    d.get_utc_seconds()
  )
}

#[component]
pub(super) fn UtcClock() -> impl IntoView {
  let now_ms = RwSignal::new(js_sys::Date::new_0().get_time());
  if let Ok(handle) = set_interval_with_handle(
    move || now_ms.set(js_sys::Date::new_0().get_time()),
    Duration::from_secs(1),
  ) {
    on_cleanup(move || handle.clear());
  }

  let utc = Memo::new(move |_| zoned(now_ms.get(), 0.0));

  view! {
    <div class="grid gap-3">
      <div class="flex items-center gap-3 rounded-lg border bg-muted/30 px-4 py-3">
        <div class="font-mono text-3xl font-semibold tabular-nums">{move || utc.get()}</div>
        <div class="text-sm text-muted-foreground">"UTC / Zulu"</div>
      </div>

      <div class="overflow-x-auto rounded-lg border">
        <table class="w-full min-w-[480px] border-collapse text-sm">
          <thead class="bg-muted/60 text-xs">
            <tr>
              <th class="border px-3 py-2 text-left">{move || t("时区")}</th>
              <th class="border px-3 py-2 text-left">{move || t("偏移")}</th>
              <th class="border px-3 py-2 text-left">{move || t("当前时间")}</th>
            </tr>
          </thead>
          <tbody>
            {TIMEZONES
              .iter()
              .map(|&(name, offset, note)| {
                view! {
                  <tr class="border-t">
                    <td class="border px-3 py-2">
                      <div class="font-medium">{name}</div>
                      <div class="text-xs text-muted-foreground">{note}</div>
                    </td>
                    <td class="border px-3 py-2 font-mono tabular-nums text-muted-foreground">
                      {format!("UTC{offset:+.0}")}
                    </td>
                    <td class="border px-3 py-2 font-mono tabular-nums">{move || zoned(now_ms.get(), offset)}</td>
                  </tr>
                }
              })
              .collect_view()}
          </tbody>
        </table>
      </div>

      <p class="text-xs text-muted-foreground">
        {move || t("通联日志、竞赛与卫星过境统一用 UTC 记录；跨日期变更线时注意日期 ±1 天。")}
      </p>
    </div>
  }
}

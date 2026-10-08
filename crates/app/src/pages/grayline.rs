//! 灰线地图：实时晨昏圈（日出/日落分界），用于判断低频 DX 的灰线窗口。
//!
//! 底图、缩放/平移/反子午线环绕等交互统一由 [`super::map::MapView`] 提供，
//! 本页仅注入 [`super::map::GraylineOverlay`]（晨昏圈 + 太阳直射点）并叠加时间偏移控制。

use std::time::Duration;

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::components::common::{Legend, PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::pages::map::{GraylineOverlay, MapView};
use crate::ui::Slider;
use crate::util::set_title;

#[component]
pub fn GraylinePage() -> impl IntoView {
  set_title("shell.grayline-map");

  let now = RwSignal::new(js_sys::Date::new_0().get_time());
  // 时间偏移（小时）：0 = 当前，正数预测未来、负数回溯过去。
  let time_offset = RwSignal::new(0.0f64);
  if let Ok(handle) = set_interval_with_handle(
    move || now.set(js_sys::Date::new_0().get_time()),
    Duration::from_secs(60),
  ) {
    on_cleanup(move || handle.clear());
  }

  // 当前展示时刻（含时间偏移），驱动灰线叠加。
  let display_ms = Signal::derive(move || now.get() + time_offset.get() * 3_600_000.0);

  let clock = Memo::new(move |_| {
    let date = js_sys::Date::new(&JsValue::from_f64(display_ms.get()));
    format!(
      "{:02}:{:02}:{:02} UTC",
      date.get_utc_hours(),
      date.get_utc_minutes(),
      date.get_utc_seconds()
    )
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=Signal::derive(move || t("shell.grayline-map"))
        subtitle=Signal::derive(move || t("radio.live-terminator-lf-dx"))
      />
      <PageContainer>
        <section class="rounded-xl border bg-card p-4">
          <MapView aria_label=Signal::derive(move || t("radio.grayline-map-scroll-to"))>
            <GraylineOverlay now_ms=display_ms />
          </MapView>

          <div class="mt-3 flex flex-col gap-2">
            <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
              <span class="font-mono text-sm font-semibold tabular-nums">{move || clock.get()}</span>
              {move || {
                let off = time_offset.get();
                if off.abs() < 1e-9 {
                  view! { <span class="text-muted-foreground">{move || t("radio.current-time")}</span> }.into_any()
                } else {
                  view! {
                    <span class="rounded bg-amber-500/15 px-1.5 py-0.5 font-medium text-amber-600">
                      {move || {
                        tf(
                          "radio.offset-h",
                          &[
                            if off > 0.0 { "+" } else { "-" },
                            &format!("{:.1}", off.abs()),
                          ],
                        )
                      }}
                    </span>
                  }
                  .into_any()
                }
              }}
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs text-muted-foreground">"-12h"</span>
              <Slider
                value=time_offset
                on_change=Callback::new(move |v: f64| time_offset.set(v))
                min=-12.0
                max=12.0
                step=0.5
                class="flex-1"
                aria_label=Signal::derive(move || t("radio.time-offset-hours"))
                aria_valuetext=Signal::derive(move || format!("{:+.1} h", time_offset.get()))
              />
              <span class="text-xs text-muted-foreground">"+12h"</span>
            </div>
            <div class="flex flex-wrap gap-1">
              {[
                ("-6h", -6.0),
                ("-1h", -1.0),
                ("现在", 0.0),
                ("+1h", 1.0),
                ("+6h", 6.0),
              ]
              .into_iter()
              .map(|(label, h)| {
                view! {
                  <button
                    type="button"
                    on:click=move |_| time_offset.set(h)
                    class="rounded-md border px-2 py-0.5 text-xs transition-colors hover:bg-accent"
                  >
                    {move || t(label)}
                  </button>
                }
              })
              .collect_view()}
            </div>
          </div>

          <div class="mt-2 space-y-1.5">
            <Legend
              items=vec![
                ("h-3 w-3 rounded-sm bg-amber-500/40", "晨昏圈"),
                ("h-2.5 w-2.5 rounded-full bg-amber-500", "太阳直射点"),
              ]
            />
            <div class="text-xs text-muted-foreground">{move || t("radio.scroll-pinch-to-zoom-2")}</div>
          </div>
          <p class="mt-3 text-xs text-muted-foreground">
            {move || t("radio.the-yellow-band-is")}
          </p>
        </section>
      </PageContainer>
    </div>
  }
}

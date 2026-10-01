//! 灰线地图：实时晨昏圈（日出/日落分界），用于判断低频 DX 的灰线窗口。
//!
//! 底图、缩放/平移/反子午线环绕等交互统一由 [`super::map::MapView`] 提供，
//! 本页仅注入 [`super::map::GraylineOverlay`]（晨昏圈 + 太阳直射点）并叠加时间偏移控制。

use std::time::Duration;

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::pages::map::{GraylineOverlay, MapView};
use crate::util::set_title;

#[component]
pub fn GraylinePage() -> impl IntoView {
  set_title("灰线地图");

  let now = RwSignal::new(js_sys::Date::new_0().get_time());
  // 时间偏移（小时）：0 = 当前，正数预测未来、负数回溯过去。
  let time_offset = RwSignal::new(0.0f64);
  set_interval(
    move || now.set(js_sys::Date::new_0().get_time()),
    Duration::from_secs(60),
  );

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
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"灰线地图"</h1>
            <div class="text-xs text-muted-foreground">"实时晨昏圈 · 低频 DX 灰线窗口"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-4 px-4 py-5">
        <section class="rounded-xl border bg-card p-4">
          <MapView aria_label="灰线地图（滚轮缩放、拖拽平移、双指缩放、双击复位、反子午线环绕）">
            <GraylineOverlay now_ms=display_ms />
          </MapView>

          <div class="mt-3 flex flex-col gap-2">
            <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
              <span class="font-mono text-sm font-semibold tabular-nums">{move || clock.get()}</span>
              {move || {
                let off = time_offset.get();
                if off.abs() < 1e-9 {
                  view! { <span class="text-muted-foreground">{"当前时间".to_string()}</span> }
                } else {
                  view! {
                    <span class="rounded bg-amber-500/15 px-1.5 py-0.5 font-medium text-amber-600">
                      {format!("偏移 {}{:.1}h", if off > 0.0 { "+" } else { "-" }, off.abs())}
                    </span>
                  }
                }
              }}
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs text-muted-foreground">"-12h"</span>
              <input
                type="range"
                min="-12"
                max="12"
                step="0.5"
                prop:value=move || time_offset.get().to_string()
                on:input=move |e| {
                  if let Ok(v) = event_target_value(&e).parse::<f64>() {
                    time_offset.set(v);
                  }
                }
                class="h-1.5 flex-1 accent-primary"
                aria-label="时间偏移（小时）"
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
                    {label}
                  </button>
                }
              })
              .collect_view()}
            </div>
          </div>

          <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
            <span class="flex items-center gap-1.5">
              <span class="inline-block h-3 w-3 rounded-sm bg-amber-500/40"></span>
              <span>"晨昏圈"</span>
              <span class="ml-1 inline-block h-2.5 w-2.5 rounded-full bg-amber-500"></span>
              <span>"太阳直射点"</span>
            </span>
            <span class="text-muted-foreground">"滚轮/双指缩放 · 拖拽平移 · 双击复位 · 悬停经纬度"</span>
          </div>
          <p class="mt-3 text-xs text-muted-foreground">
            "黄色带为晨昏圈（日出/日落分界），随 UTC 时间实时移动；实心点为太阳直射点、空心为反日点。两端处于灰线的路径，常是 160m / 80m 低频 DX 的黄金窗口。"
          </p>
        </section>
      </div>
    </div>
  }
}

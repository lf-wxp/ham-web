//! 收到的 APRS 台站位置：计数、世界地图与台站列表。
//!
//! 位置的识别与按站汇总全在核心 `ham_web_core::aprs_position`（有单测）；这里只负责画。

use ham_web_core::aprs_position::{Station, StationBook};
use leptos::prelude::*;

use crate::components::common::EmptyState;
use crate::i18n::{t, tf};
use crate::pages::map::{MapView, project};
use crate::ui::card_title_class;

/// 台站在 `<For>` 里的键：站名 + 报数 + 位置。位置变了就是新的一行 / 一个新点，
/// 不会出现「键没变所以点还画在旧位置」。
fn station_key(s: &Station) -> String {
  format!(
    "{}|{}|{}|{}",
    s.name,
    s.reports,
    s.fix.lat.to_bits(),
    s.fix.lon.to_bits()
  )
}

#[component]
pub fn PositionMap(book: RwSignal<StationBook>) -> impl IntoView {
  let stations = move || book.with(|b| b.stations().to_vec());
  // 外层闭包只跟「空 / 非空」：直接读 `book` 的话每收一帧都会把地图整个重建，用户的缩放与平移全丢。
  let empty = Memo::new(move |_| book.with(StationBook::is_empty));

  view! {
    <section class="border bg-card p-4" data-testid="aprs-position-map">
      <h2 class=card_title_class("")>{move || t("tools.kiss-map")}</h2>
      {move || {
        if empty.get() {
          view! {
            <div class="mt-3">
              <EmptyState plain=true title=Signal::derive(move || t("tools.kiss-map-empty")) />
            </div>
          }
            .into_any()
        } else {
          view! {
            <div class="mt-3 space-y-3">
              <p class="text-xs text-muted-foreground" role="status" data-testid="aprs-tally">
                {move || {
                  let c = book.with(StationBook::tally);
                  tf(
                    "tools.kiss-map-tally",
                    &[
                      &c.positioned.to_string(),
                      &c.compressed.to_string(),
                      &c.mic_e.to_string(),
                      &c.malformed.to_string(),
                    ],
                  )
                }}
              </p>
              <MapView aria_label=Signal::derive(move || t("tools.kiss-map-aria"))>
                <For each=stations key=station_key let:s>
                  {{
                    let (x, y) = project(s.fix.lon, s.fix.lat);
                    view! {
                      <circle
                        data-station=s.name.clone()
                        cx=x.to_string()
                        cy=y.to_string()
                        r="3"
                        class="fill-primary/80 stroke-background"
                        stroke-width="1"
                        vector-effect="non-scaling-stroke"
                      >
                        <title>{format!("{} · {}", s.name, s.fix.label())}</title>
                      </circle>
                    }
                  }}
                </For>
              </MapView>
              <ul class="space-y-1.5" aria-label=move || t("tools.kiss-map")>
                // 最近更新的排在最上面。
                <For each=move || stations().into_iter().rev() key=station_key let:s>
                  <li
                    class="flex flex-wrap items-baseline gap-x-3 gap-y-1 bg-inset px-3 py-2 text-xs"
                    data-station-row=s.name.clone()
                  >
                    <span class="font-mono">{s.name.clone()}</span>
                    <span class="min-w-0 break-words font-mono">{s.fix.label()}</span>
                    <span class="ml-auto text-muted-foreground">
                      {tf("tools.kiss-map-reports", &[&s.reports.to_string()])}
                    </span>
                  </li>
                </For>
              </ul>
              <p class="text-xs text-muted-foreground">{move || t("tools.kiss-map-note")}</p>
            </div>
          }
            .into_any()
        }
      }}
    </section>
  }
}

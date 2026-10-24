//! 接收台分布图：把 PSK Reporter 的接收台按网格中心落图。
//!
//! 与「接收方向分布」的扇区表互补：扇区表回答「哪个方向最多」，地图回答「具体在哪」。
//! 只有接收台网格，没有强度：点的位置来自 `receiver_locator`，不是实测定位。

use leptos::prelude::*;

use crate::i18n::{t, tp};
use crate::pages::map::{MapView, project};

#[component]
pub(super) fn CoverageMap(
  /// 接收台中心点 `(纬度, 经度)`。
  #[prop(into)]
  points: Signal<Vec<(f64, f64)>>,
) -> impl IntoView {
  // 只要长度：`with` 借用，不把整份点集克隆一遍。
  let count = Signal::derive(move || points.with(Vec::len));
  view! {
    <section class="pxl-card border bg-card" aria-labelledby="coverage-map-heading">
      <h2 id="coverage-map-heading" class="pxl-section-title">
        {move || t("coverage.map-title")}
      </h2>
      <div class="space-y-2 p-4">
        <p class="text-xs text-muted-foreground">{move || t("coverage.map-desc")}</p>
        <p class="text-xs text-muted-foreground">
          {move || {
            let n = count.get();
            tp("coverage.map-count", n, &[&n.to_string()])
          }}
        </p>
        <MapView aria_label=Signal::derive(|| t("coverage.map-title"))>
          {move || {
            points
              .get()
              .into_iter()
              .map(|(lat, lon)| {
                let (x, y) = project(lon, lat);
                view! {
                  <circle
                    cx=x
                    cy=y
                    r="3"
                    class="fill-win/70"
                    data-spot="receiver"
                  />
                }
              })
              .collect_view()
          }}
        </MapView>
      </div>
    </section>
  }
}

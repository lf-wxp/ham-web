//! DX spot 行里的方位一行：「方位 312° NW · 9120 km」加上这个方位的依据。
//!
//! 方位怎么算、什么时候不给，全在核心 `ham_web_core::spot_bearing`；这里只负责如实写出依据
//! （实体中心是粗定位，备注里的网格也可能是报告者的位置）。

use ham_web_core::spot_bearing::{Basis, Heading, compass8};
use leptos::prelude::*;

use crate::i18n::{t, tf};

#[component]
pub fn SpotHeading(heading: Heading) -> impl IntoView {
  let compass = compass8(heading.bearing_deg);
  let bearing = format!("{:.0}", heading.bearing_deg.rem_euclid(360.0));
  let distance = format!("{:.0}", heading.distance_km);
  let basis = heading.basis;
  view! {
    <p class="text-xs text-muted-foreground" data-slot="spot-heading">
      <span class="font-mono tabular-nums">
        {move || tf("radio.spot-heading", &[&bearing, compass, &distance])}
      </span>
      " · "
      {move || match &basis {
        Basis::Entity => t("radio.spot-heading-entity"),
        Basis::Grid(g) => tf("radio.spot-heading-grid", &[g]),
      }}
    </p>
  }
}

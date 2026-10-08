use ham_web_core::grid::{distance_bearing, lat_lon_from_grid};
use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, Input};
use crate::util::unique_id;

/// 两点距离 / 方位角：两个 Maidenhead 网格 → 大圆距离与方位角。
#[component]
pub(super) fn DistanceBearing() -> impl IntoView {
  let g1 = RwSignal::new(String::new());
  let g2 = RwSignal::new(String::new());

  let g1_id = unique_id("distance-bearing-start");
  let g2_id = unique_id("distance-bearing-end");

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.start-grid-e-g")) r#for=g1_id.clone()>
        <Input
          id=g1_id
          value=g1
          on_change=Callback::new(move |v: String| g1.set(v.to_uppercase()))
          placeholder="OM89EW"
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.end-grid-e-g")) r#for=g2_id.clone()>
        <Input
          id=g2_id
          value=g2
          on_change=Callback::new(move |v: String| g2.set(v.to_uppercase()))
          placeholder="JN18EU"
        />
      </Field>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let a = lat_lon_from_grid(g1.get().trim());
          let b = lat_lon_from_grid(g2.get().trim());
          match (a, b) {
            (Some((la1, lo1)), Some((la2, lo2))) => {
              let (d, br) = distance_bearing(la1, lo1, la2, lo2);
              tf(
                "tools.great-circle-distance-km",
                &[
                  &fmt_num(d),
                  &format!("{br:.0}"),
                  &format!("{la1:.2}"),
                  &format!("{lo1:.2}"),
                  &format!("{la2:.2}"),
                  &format!("{lo2:.2}"),
                ],
              )
            }
            _ => t("tools.enter-two-grid-codes"),
          }
        }}
      </div>
    </div>
  }
}

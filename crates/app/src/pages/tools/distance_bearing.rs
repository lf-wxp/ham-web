use ham_web_core::grid::{distance_bearing, lat_lon_from_grid};
use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// 两点距离 / 方位角：两个 Maidenhead 网格 → 大圆距离与方位角。
#[component]
pub(super) fn DistanceBearing() -> impl IntoView {
  let g1 = RwSignal::new(String::new());
  let g2 = RwSignal::new(String::new());

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("起点网格（如 OM89EW）")}</span>
        <input
          type="text"
          placeholder="OM89EW"
          prop:value=move || g1.get()
          on:input=move |e| g1.set(event_target_value(&e).to_uppercase())
          class=INPUT
        />
      </label>
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("终点网格（如 JN18EU）")}</span>
        <input
          type="text"
          placeholder="JN18EU"
          prop:value=move || g2.get()
          on:input=move |e| g2.set(event_target_value(&e).to_uppercase())
          class=INPUT
        />
      </label>
      <div class="sm:col-span-2 rounded-lg bg-muted/40 px-3 py-2 text-sm tabular-nums text-muted-foreground">
        {move || {
          let a = lat_lon_from_grid(g1.get().trim());
          let b = lat_lon_from_grid(g2.get().trim());
          match (a, b) {
            (Some((la1, lo1)), Some((la2, lo2))) => {
              let (d, br) = distance_bearing(la1, lo1, la2, lo2);
              tf(
                "大圆距离 {} km　方位角 {}°　（{}, {} → {}, {}）",
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
            _ => t("请输入两个至少 4 位的网格码。"),
          }
        }}
      </div>
    </div>
  }
}

use leptos::prelude::*;

use super::{BandCond, condition_color};
use crate::i18n::t;

/// 各波段传播条件表。
#[component]
pub(super) fn ConditionsTable(conditions: Vec<BandCond>) -> impl IntoView {
  // 预计算为 owned 行（band, 白天, 夜间），避免 view 闭包借用参数。
  let mut bands: Vec<String> = Vec::new();
  for c in &conditions {
    if !bands.contains(&c.band) {
      bands.push(c.band.clone());
    }
  }
  let rows: Vec<(String, String, String)> = bands
    .iter()
    .map(|band| {
      let find = |time: &str| {
        conditions
          .iter()
          .find(|c| &c.band == band && c.time == time)
          .map(|c| c.condition.clone())
          .unwrap_or_else(|| "—".to_owned())
      };
      (band.clone(), find("day"), find("night"))
    })
    .collect();

  view! {
    <div class="overflow-x-auto">
      <table class="w-full min-w-[320px] border-collapse text-sm">
        <thead class="bg-muted/60 text-xs">
          <tr>
            <th class="border px-3 py-2 text-left">{move || t("波段")}</th>
            <th class="border px-3 py-2 text-left">{move || t("白天")}</th>
            <th class="border px-3 py-2 text-left">{move || t("夜间")}</th>
          </tr>
        </thead>
        <tbody>
          {rows
            .into_iter()
            .map(|(band, day, night)| {
              let day_color = condition_color(&day);
              let night_color = condition_color(&night);
              view! {
                <tr class="border-t">
                  <td class="border px-3 py-2 font-mono">{band}</td>
                  <td class=format!("border px-3 py-2 font-medium {day_color}")>{day}</td>
                  <td class=format!("border px-3 py-2 font-medium {night_color}")>{night}</td>
                </tr>
              }
            })
            .collect_view()}
        </tbody>
      </table>
    </div>
  }
}

use leptos::prelude::*;

use super::{BandCond, condition_color};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};

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
    <Table class="min-w-[320px] text-sm">
      <TableHead>
        <TableRow>
          <TableHeader class="border px-3 py-2">{move || t("radio.band")}</TableHeader>
          <TableHeader class="border px-3 py-2">{move || t("radio.day")}</TableHeader>
          <TableHeader class="border px-3 py-2">{move || t("radio.night")}</TableHeader>
        </TableRow>
      </TableHead>
      <TableBody>
        {rows
          .into_iter()
          .map(|(band, day, night)| {
            let day_color = condition_color(&day);
            let night_color = condition_color(&night);
            view! {
              <TableRow>
                <TableCell class="border px-3 py-2 font-mono">{move || kt_tracked(&band)}</TableCell>
                <TableCell class=format!("border px-3 py-2 {day_color}")>{move || kt_tracked(&day)}</TableCell>
                <TableCell class=format!("border px-3 py-2 {night_color}")>{move || kt_tracked(&night)}</TableCell>
              </TableRow>
            }
          })
          .collect_view()}
      </TableBody>
    </Table>
  }
}

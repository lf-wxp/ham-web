//! `table_rows`：从 `mod.rs` 拆出的视图构造函数（一个组件一个文件）。

use ham_web_core::bands::{self, BANDS};
use leptos::prelude::*;

use crate::i18n::t;

use super::usage_badge::UsageBadge;
use super::{CELL, notes_view, range_view};

pub(super) fn table_rows(jump: Callback<&'static str>) -> impl IntoView {
  let mw_rows = bands::microwave_rows().to_string();
  let first_mw = BANDS.iter().position(|b| b.microwave);
  BANDS
    .iter()
    .enumerate()
    .flat_map(move |(bi, band)| {
      let rows = band.rows();
      let span = rows.to_string();
      let mw_rows = mw_rows.clone();
      (0..rows).map(move |ri| {
        let band_cells = (ri == 0).then(|| {
          view! {
            <td class=CELL rowspan=span.clone()>
              <span class="font-mono">{band.number}</span>
            </td>
            <td class=CELL rowspan=span.clone() colspan=if band.microwave { "1" } else { "2" }>
              {band.name}
            </td>
            {(band.microwave && first_mw == Some(bi))
              .then(|| {
                view! {
                  <td class=CELL rowspan=mw_rows.clone()>
                    <span class="[writing-mode:vertical-rl] tracking-[0.4em]">{move || t("knowledge.microwave")}</span>
                  </td>
                }
              })}
            <td class=CELL rowspan=span.clone()>{band.wavelength}</td>
            <td class=CELL rowspan=span.clone()>{band.freq_name}</td>
            <td class=CELL rowspan=span.clone()>
              <span class="font-mono">{band.freq_abbr}</span>
            </td>
            <td class=CELL rowspan=span.clone()>
              <span class="whitespace-nowrap">{band.freq_range}</span>
            </td>
          }
        });
        let alloc_cells = match band.allocations.get(ri) {
          None => view! {
            <td class=format!("{CELL} text-muted-foreground")>"/"</td>
            <td class=format!("{CELL} text-muted-foreground")>"/"</td>
            <td class=format!("{CELL} text-muted-foreground")>"/"</td>
          }
          .into_any(),
          Some(a) => view! {
            <td class=CELL>{range_view(a)}</td>
            <td class=CELL>
              <UsageBadge usage=a.usage />
            </td>
            {band
              .remark_span(ri)
              .map(|n| {
                view! {
                  <td class=format!("{CELL} text-xs") rowspan=n.to_string()>
                    {notes_view(band.remark_for(ri), jump)}
                  </td>
                }
              })}
          }
          .into_any(),
        };
        view! { <tr class="transition-colors hover:bg-muted/40">{band_cells} {alloc_cells}</tr> }
      })
    })
    .collect_view()
}

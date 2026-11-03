//! `table_rows`：从 `mod.rs` 拆出的视图构造函数（一个组件一个文件）。

use ham_web_core::bands::{self, BANDS};
use leptos::prelude::*;

use crate::data::kt_tracked;
use crate::i18n::t;

use super::usage_badge::UsageBadge;
use super::{notes_view, range_view};

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
            <td class=crate::kit::table_cell_class("") rowspan=span.clone()>
              <span class="font-mono">{band.number}</span>
            </td>
            <td class=crate::kit::table_cell_class("") rowspan=span.clone() colspan=if band.microwave { "1" } else { "2" }>
              {move || kt_tracked(band.name)}
            </td>
            {(band.microwave && first_mw == Some(bi))
              .then(|| {
                view! {
                  <td class=crate::kit::table_cell_class("") rowspan=mw_rows.clone()>
                    <span class="[writing-mode:vertical-rl] tracking-[0.4em]">{move || t("knowledge.microwave")}</span>
                  </td>
                }
              })}
            <td class=crate::kit::table_cell_class("") rowspan=span.clone()>{move || kt_tracked(band.wavelength)}</td>
            <td class=crate::kit::table_cell_class("") rowspan=span.clone()>{move || kt_tracked(band.freq_name)}</td>
            <td class=crate::kit::table_cell_class("") rowspan=span.clone()>
              <span class="font-mono">{move || kt_tracked(band.freq_abbr)}</span>
            </td>
            <td class=crate::kit::table_cell_class("") rowspan=span.clone()>
              <span class="whitespace-nowrap">{move || kt_tracked(band.freq_range)}</span>
            </td>
          }
        });
        let alloc_cells = match band.allocations.get(ri) {
          None => view! {
            <td class=crate::kit::table_cell_class("text-muted-foreground")>"/"</td>
            <td class=crate::kit::table_cell_class("text-muted-foreground")>"/"</td>
            <td class=crate::kit::table_cell_class("text-muted-foreground")>"/"</td>
          }
          .into_any(),
          Some(a) => view! {
            <td class=crate::kit::table_cell_class("")>{range_view(a)}</td>
            <td class=crate::kit::table_cell_class("")>
              <UsageBadge usage=a.usage />
            </td>
            {band
              .remark_span(ri)
              .map(|n| {
                view! {
                  <td class=crate::kit::table_cell_class("text-xs") rowspan=n.to_string()>
                    {notes_view(band.remark_for(ri), jump)}
                  </td>
                }
              })}
          }
          .into_any(),
        };
        view! { <tr class="transition-colors hover:bg-inset">{band_cells} {alloc_cells}</tr> }
      })
    })
    .collect_view()
}

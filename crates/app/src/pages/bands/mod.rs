//! 频谱波段划分表：以 HTML 表格展示各波段的波长/频率范围，以及业余业务、卫星业余业务频段划分与脚注。
//! 桌面端为与原表一致的合并单元格表格，移动端为按波段分组的卡片。

mod band_card;
mod bands_page;
mod usage_badge;

pub use bands_page::BandsPage;

use ham_web_core::bands::{self, Allocation, BANDS, Note, Usage};
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

use usage_badge::UsageBadge;

const CELL: &str = "border px-2 py-1.5 text-center align-middle";
const SAT_ICON: &str = "h-3.5 w-3.5 shrink-0 text-amber-600 dark:text-amber-400";

fn usage_class(u: Usage) -> &'static str {
  match u {
    Usage::Exclusive => {
      "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-300"
    }
    Usage::SolePrimary => "bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300",
    Usage::Primary => "bg-sky-100 text-sky-800 dark:bg-sky-900/40 dark:text-sky-300",
    Usage::Secondary => "bg-muted text-muted-foreground",
  }
}

fn footnote_id(code: &str) -> String {
  format!("fn-{}", code.replace('.', "-"))
}

fn range_view(a: &'static Allocation) -> impl IntoView {
  view! {
    <span class="inline-flex items-center gap-1 whitespace-nowrap">
      {a.satellite
        .then(|| {
          view! {
            <Icon kind=IconKind::Satellite class=SAT_ICON />
            <span class="sr-only">"卫星业余业务"</span>
          }
        })}
      <span class="tabular-nums">{a.range}</span>
    </span>
  }
}

fn notes_view(notes: &'static [Note], jump: Callback<&'static str>) -> impl IntoView {
  notes
    .iter()
    .map(|note| match *note {
      Note::Text(t) => view! { <span>{t}</span> }.into_any(),
      Note::Ref(code) => view! {
        <button
          type="button"
          class="font-mono font-medium text-blue-600 underline underline-offset-2 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
          title=format!("查看脚注 {code}")
          on:click=move |_| jump.run(code)
        >
          {code}
        </button>
      }
      .into_any(),
    })
    .collect_view()
}

fn table_rows(jump: Callback<&'static str>) -> impl IntoView {
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
                    <span class="[writing-mode:vertical-rl] tracking-[0.4em]">"微波"</span>
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

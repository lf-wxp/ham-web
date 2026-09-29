//! 频谱波段划分表：以 HTML 表格展示各波段的波长/频率范围，以及业余业务、卫星业余业务频段划分与脚注。
//! 桌面端为与原表一致的合并单元格表格，移动端为按波段分组的卡片。

use std::time::Duration;

use ham_exam_core::bands::{self, Allocation, BANDS, Band, FOOTNOTES, Note, Usage};
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::util::{document, set_title};

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

#[component]
fn UsageBadge(usage: Usage) -> impl IntoView {
  view! {
    <span class=format!(
      "inline-flex whitespace-nowrap rounded-md px-2 py-0.5 text-xs font-medium {}",
      usage_class(usage),
    )>{usage.label()}</span>
  }
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

#[component]
fn BandCard(band: &'static Band, jump: Callback<&'static str>) -> impl IntoView {
  let allocations = if band.allocations.is_empty() {
    view! {
      <div class="mt-3 border-t border-dashed pt-3 text-xs text-muted-foreground">"无业余业务频段划分"</div>
    }
    .into_any()
  } else {
    let items = band
      .allocations
      .iter()
      .enumerate()
      .map(|(i, a)| {
        let notes = band.remark_for(i);
        view! {
          <li class="flex flex-wrap items-center gap-x-2 gap-y-1 py-2 text-sm">
            {range_view(a)}
            <span class="ml-auto">
              <UsageBadge usage=a.usage />
            </span>
            {(!notes.is_empty())
              .then(|| view! { <div class="w-full text-xs text-muted-foreground">{notes_view(notes, jump)}</div> })}
          </li>
        }
      })
      .collect_view();
    view! { <ul class="mt-3 divide-y border-t border-dashed">{items}</ul> }.into_any()
  };
  view! {
    <article class="rounded-xl border bg-card p-4">
      <div class="flex flex-wrap items-center gap-2">
        <span class="rounded-md bg-muted px-2 py-0.5 font-mono text-xs font-semibold text-foreground">
          "带号 " {band.number}
        </span>
        <h3 class="font-semibold">{band.name}</h3>
        {band
          .microwave
          .then(|| view! { <span class="rounded-full border px-2 py-0.5 text-xs text-muted-foreground">"微波"</span> })}
        <span class="ml-auto font-mono text-sm font-semibold">{band.freq_abbr}</span>
      </div>
      <dl class="mt-2 grid grid-cols-2 gap-x-3 gap-y-1 text-xs">
        <div>
          <dt class="text-muted-foreground">"波长范围"</dt>
          <dd class="font-medium tabular-nums">{band.wavelength}</dd>
        </div>
        <div>
          <dt class="text-muted-foreground">"频段"</dt>
          <dd class="font-medium tabular-nums">{band.freq_name} " · " {band.freq_range}</dd>
        </div>
      </dl>
      {allocations}
    </article>
  }
}

#[component]
fn Stat(label: &'static str, value: usize) -> impl IntoView {
  view! {
    <div class="rounded-xl border bg-card p-3">
      <div class="text-2xl font-semibold tabular-nums">{value}</div>
      <div class="text-xs text-muted-foreground">{label}</div>
    </div>
  }
}

#[component]
pub fn BandsPage() -> impl IntoView {
  set_title("频谱波段划分表");
  let active = RwSignal::new(None::<&'static str>);

  let jump = Callback::new(move |code: &'static str| {
    if let Some(el) = document().get_element_by_id(&footnote_id(code)) {
      el.scroll_into_view();
    }
    active.set(Some(code));
    set_timeout(
      move || {
        if active.try_get_untracked().flatten() == Some(code) {
          active.try_set(None);
        }
      },
      Duration::from_millis(1600),
    );
  });

  let footnotes = FOOTNOTES
    .iter()
    .map(|f| {
      let code = f.code;
      view! {
        <div
          id=footnote_id(code)
          class=move || {
            format!(
              "grid scroll-mt-24 grid-cols-[4.5rem_1fr] gap-3 px-4 py-3 text-sm transition-colors duration-500 {}",
              if active.get() == Some(code) { "bg-accent" } else { "" },
            )
          }
        >
          <dt class="font-mono font-semibold">{code}</dt>
          <dd class="leading-6 text-muted-foreground">{f.text}</dd>
        </div>
      }
    })
    .collect_view();

  let legend = Usage::ALL
    .into_iter()
    .map(|u| view! { <UsageBadge usage=u /> })
    .collect_view();

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-6xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"频谱波段划分表"</div>
            <div class="text-xs text-muted-foreground">"均含上限，不含下限 · C = λf = 3×10⁸ m/s"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-6xl space-y-4 px-4 py-5">
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label="波段" value=BANDS.len() />
          <Stat label="业余业务频段" value=bands::allocation_count() />
          <Stat label="可供卫星业余业务" value=bands::satellite_count() />
          <Stat label="脚注" value=FOOTNOTES.len() />
        </div>

        <div class="flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-muted-foreground">
          <span class="inline-flex items-center gap-1">
            <Icon kind=IconKind::Satellite class=SAT_ICON />
            "表示该频段也供卫星业余业务使用"
          </span>
          <span class="inline-flex flex-wrap items-center gap-1.5">"使用状态：" {legend}</span>
        </div>

        <div class="hidden overflow-x-auto rounded-xl border bg-card md:block">
          <table class="w-full min-w-[960px] border-collapse border-hidden text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>"带号"</th>
                <th class=CELL colspan="2">"波段名称"</th>
                <th class=CELL>"波长范围"</th>
                <th class=CELL colspan="2">"频段名称"</th>
                <th class=CELL>"频段范围"</th>
                <th class=CELL>"业余业务/卫星业余业务频段"</th>
                <th class=CELL>"使用状态"</th>
                <th class=CELL>"脚注/备注"</th>
              </tr>
            </thead>
            <tbody>{table_rows(jump)}</tbody>
          </table>
        </div>

        <div class="space-y-3 md:hidden">
          {BANDS.iter().map(|band| view! { <BandCard band=band jump=jump /> }).collect_view()}
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"脚注"</h2>
          <dl class="divide-y">{footnotes}</dl>
        </section>
      </div>
    </div>
  }
}

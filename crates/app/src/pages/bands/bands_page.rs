use std::time::Duration;

use ham_web_core::bands::{self, BANDS, FOOTNOTES, Usage};
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::ui::Stat;
use crate::util::document;
use crate::util::set_title;

use super::band_card::BandCard;
use super::usage_badge::UsageBadge;
use super::{CELL, SAT_ICON, footnote_id, table_rows};

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
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"频谱波段划分表"</h1>
            <div class="text-xs text-muted-foreground">"均含上限，不含下限 · C = λf = 3×10⁸ m/s"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-4 px-4 py-5">
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

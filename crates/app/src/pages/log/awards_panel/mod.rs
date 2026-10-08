mod bar_row;
use bar_row::bar_row;

use crate::i18n::{t, tf};
use ham_web_core::award_progress::{
  AwardProgress, CONTINENTS, DXCC_TARGET, IOTA_TARGET, Progress, WAS_TARGET, WAZ_TARGET,
  WPX_TARGET, award_gaps, vucc_target,
};
use ham_web_core::logbook::LogEntry;
use leptos::prelude::*;

use crate::ui::{RadioGroup, RadioGroupItem};

fn count<T: Ord>(p: &Progress<T>, confirmed: bool) -> usize {
  if confirmed {
    p.confirmed.len()
  } else {
    p.worked.len()
  }
}

/// 日志联动的奖状进度：DXCC / WAZ / WAC / VUCC。
#[component]
pub(super) fn AwardsPanel(entries: Vec<LogEntry>) -> impl IntoView {
  let progress = StoredValue::new(AwardProgress::from_entries(&entries));
  let confirmed = RwSignal::new(false);

  view! {
    <section id="awards" class="scroll-mt-24 rounded-xl border bg-card">
      <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
        <h3 class="mr-auto text-sm font-semibold">{move || t("log.award-progress")}</h3>
        <RadioGroup
          value=Signal::derive(move || {
            if confirmed.get() { "confirmed" } else { "worked" }.to_owned()
          })
          on_change=Callback::new(move |v: String| confirmed.set(v == "confirmed"))
          aria_label=Signal::derive(move || t("log.award-progress"))
          class="flex items-center gap-4"
        >
          <label class="flex cursor-pointer items-center gap-2 text-xs">
            <RadioGroupItem value="worked" />
            {move || t("log.worked")}
          </label>
          <label class="flex cursor-pointer items-center gap-2 text-xs">
            <RadioGroupItem value="confirmed" />
            {move || t("log.confirmed-qsl")}
          </label>
        </RadioGroup>
      </div>
      {move || {
        let c = confirmed.get();
        progress.with_value(|p| {
          let mut bands: Vec<(String, usize)> = p
            .dxcc_by_band
            .iter()
            .map(|(b, x)| (b.clone(), count(x, c)))
            .filter(|(_, n)| *n > 0)
            .collect();
          bands.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
          let modes: Vec<(&'static str, usize)> = ["CW", "Phone", "Digital"]
            .into_iter()
            .map(|m| (m, p.dxcc_by_mode.get(m).map_or(0, |x| count(x, c))))
            .collect();
          let vucc: Vec<(String, usize, usize)> = p
            .vucc
            .iter()
            .filter_map(|(b, x)| Some((b.clone(), count(x, c), vucc_target(b)?)))
            .filter(|(_, n, _)| *n > 0)
            .collect();
          let zones = if c { &p.waz.confirmed } else { &p.waz.worked };
          let conts = if c { &p.wac.confirmed } else { &p.wac.worked };
          let gaps = award_gaps(p, c);
          view! {
            <div class="space-y-5 p-4">
              {(!gaps.is_empty()).then(|| view! {
                <div class="rounded-lg border bg-muted/30 p-3">
                  <div class="mb-1.5 text-xs font-semibold">{move || t("log.sprint-suggestions")}</div>
                  <div class="flex flex-wrap gap-1.5">
                    {gaps
                      .iter()
                      .take(3)
                      .map(|g| {
                        view! {
                          <span class="rounded-full border bg-background px-2.5 py-1 text-xs">
                            {tf("log.needs-more", &[g.label, &g.remaining().to_string()])}
                          </span>
                        }
                      })
                      .collect_view()}
                  </div>
                  <p class="mt-1.5 text-[11px] text-muted-foreground">{move || t("log.closer-to-the-goal")}</p>
                </div>
              })}
              <div class="space-y-3">
                {bar_row("DXCC".to_owned(), count(&p.dxcc, c), DXCC_TARGET)}
                <div class="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                  {modes
                    .into_iter()
                    .map(|(m, n)| view! {
                      <span>{m} " " <span class="font-semibold tabular-nums text-foreground">{n}</span></span>
                    })
                    .collect_view()}
                </div>
                {(!bands.is_empty()).then(|| view! {
                  <div class="flex flex-wrap gap-1.5">
                    {bands
                      .into_iter()
                      .map(|(b, n)| view! {
                        <span class="rounded-full border bg-muted/40 px-2.5 py-0.5 text-xs">
                          {b} " " <span class="font-semibold tabular-nums">{n}</span>
                        </span>
                      })
                      .collect_view()}
                  </div>
                })}
              </div>

              <div class="space-y-2">
                {bar_row(t("log.waz-cq-zones"), zones.len(), WAZ_TARGET)}
                <div class="grid grid-cols-10 gap-1">
                  {(1..=40u8)
                    .map(|z| {
                      let on = zones.contains(&z);
                      view! {
                        <span
                          class=if on {
                            "rounded bg-primary py-0.5 text-center text-[10px] font-medium tabular-nums text-primary-foreground"
                          } else {
                            "rounded bg-muted py-0.5 text-center text-[10px] tabular-nums text-muted-foreground"
                          }
                          title=tf("log.cq-zone", &[&z.to_string()])
                        >
                          {z}
                        </span>
                      }
                    })
                    .collect_view()}
                </div>
              </div>

              <div class="space-y-2">
                {bar_row(t("log.wac-six-continents"), conts.len(), CONTINENTS.len())}
                <div class="flex flex-wrap gap-1.5">
                  {CONTINENTS
                    .iter()
                    .map(|(code, name)| {
                      let on = conts.contains(code);
                      view! {
                        <span class=if on {
                          "rounded-full bg-primary px-2.5 py-0.5 text-xs text-primary-foreground"
                        } else {
                          "rounded-full border px-2.5 py-0.5 text-xs text-muted-foreground"
                        }>
                          {format!("{name} {code}")}
                        </span>
                      }
                    })
                    .collect_view()}
                </div>
              </div>

              {(!vucc.is_empty()).then(|| view! {
                <div class="space-y-3">
                  {vucc
                    .into_iter()
                    .map(|(b, n, target)| {
                      let label = if b == "SAT" { t("log.vucc-satellite") } else { format!("VUCC {b}") };
                      bar_row(label, n, target)
                    })
                    .collect_view()}
                </div>
              })}

              <div class="space-y-3">
                <div class="flex items-center justify-between text-xs">
                  <span class="font-medium">{"DXCC Challenge"}</span>
                  <span class="tabular-nums text-muted-foreground">{p.dxcc_challenge}</span>
                </div>
                {bar_row("WPX".to_owned(), count(&p.wpx, c), WPX_TARGET)}
                {bar_row(t("log.was-us-states-2"), count(&p.was, c), WAS_TARGET)}
                {bar_row(t("log.iota-island-groups-2"), count(&p.iota, c), IOTA_TARGET)}
              </div>

              <p class="text-xs text-muted-foreground">
                {move || t("log.records-without-a-cq")}
              </p>
            </div>
          }
        })
      }}
    </section>
  }
}

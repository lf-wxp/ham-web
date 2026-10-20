use ham_web_core::cabrillo_import::SkipReason;
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};

use super::cabrillo_import::{ImportReport, contest_label};

const fn reason_key(r: SkipReason) -> &'static str {
  match r {
    SkipReason::Layout => "contest.cabrillo-reason-layout",
    SkipReason::Frequency => "contest.cabrillo-reason-frequency",
    SkipReason::Mode => "contest.cabrillo-reason-mode",
    SkipReason::Date => "contest.cabrillo-reason-date",
    SkipReason::Time => "contest.cabrillo-reason-time",
    SkipReason::Callsign => "contest.cabrillo-reason-callsign",
    SkipReason::Report => "contest.cabrillo-reason-report",
  }
}

/// 最近一次 Cabrillo 导入的结果：持久显示（Toast 只有几秒，被跳过的行得留着给人看）。
#[component]
pub(super) fn ImportReportPanel(report: RwSignal<Option<ImportReport>>) -> impl IntoView {
  view! {
    {move || report.get().map(|r| {
      let label = contest_label(&r);
      let hidden = r.skipped_total.saturating_sub(r.samples.len());
      view! {
        <section class="pxl-card border bg-card" aria-labelledby="cabrillo-import-heading">
          <h2 id="cabrillo-import-heading" class="pxl-section-title">
            {move || t("contest.cabrillo-import-heading")}
          </h2>
          <div class="space-y-3 p-4 text-sm">
            <p class="font-medium">
              {tp("contest.cabrillo-imported", r.added, &[&r.added.to_string(), &label])}
            </p>
            {(r.dupes > 0).then(|| view! {
              <p>{tp("contest.cabrillo-dupes", r.dupes, &[&r.dupes.to_string()])}</p>
            })}
            {(r.x_qso > 0).then(|| view! {
              <p>{tp("contest.cabrillo-x-qso", r.x_qso, &[&r.x_qso.to_string()])}</p>
            })}
            {r.switched_start.clone().map(|start| view! {
              <p>{tf("contest.cabrillo-switched", &[&label, &start])}</p>
            })}
            {r.assumed_layout.then(|| view! {
              <p class="border border-amber-500/40 bg-amber-500/10 px-3 py-2">
                {tf("contest.cabrillo-assumed", &[&label])}
              </p>
            })}
            {(r.skipped_total > 0).then(|| view! {
              <div class="space-y-1">
                <p class="font-medium text-red-600 dark:text-red-400">
                  {tp("contest.cabrillo-skipped", r.skipped_total, &[&r.skipped_total.to_string()])}
                </p>
                <ul class="max-h-60 divide-y overflow-y-auto border font-mono text-xs">
                  {r.samples.iter().map(|s| {
                    let head = tf(
                      "contest.cabrillo-skipped-line",
                      &[&s.line.to_string(), &t(reason_key(s.reason))],
                    );
                    view! {
                      <li class="space-y-0.5 px-3 py-1.5">
                        <div class="font-sans">{head}</div>
                        <div class="break-all text-muted-foreground">{s.text.clone()}</div>
                      </li>
                    }
                  }).collect_view()}
                </ul>
                {(hidden > 0).then(|| view! {
                  <p class="text-xs text-muted-foreground">
                    {tf("contest.cabrillo-skipped-more", &[&hidden.to_string()])}
                  </p>
                })}
              </div>
            })}
            <p class="text-xs text-muted-foreground">{move || t("contest.cabrillo-limits")}</p>
          </div>
        </section>
      }
    })}
  }
}

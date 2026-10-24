use crate::i18n::{t, tf, tp};
use ham_web_core::exam_history::{ExamRecord, Readiness, Verdict, assess, of_bank};
use ham_web_core::{Bank, ExamRule};
use leptos::prelude::*;

fn verdict_class(v: Verdict) -> &'static str {
  match v {
    Verdict::Ready => "border-win/40 bg-win/15 text-win-text",
    Verdict::Almost => "border-gold/40 bg-gold/15 text-gold-text",
    Verdict::NotYet => "border-destructive/40 bg-destructive/15 text-hp-text",
    Verdict::NeedMore => "text-muted-foreground",
  }
}

fn verdict_hint(r: &Readiness, pass: usize, total: usize) -> String {
  match r.verdict {
    Verdict::NeedMore => tp("common.taken-times-a-few", r.count, &[&r.count.to_string()]),
    Verdict::Ready => tp(
      "common.the-last-attempts-were",
      r.count,
      &[&r.count.to_string(), &pass.to_string(), &total.to_string()],
    ),
    Verdict::Almost => tp(
      "common.of-the-last-attempts",
      r.count,
      &[&r.count.to_string(), &r.passed.to_string()],
    ),
    Verdict::NotYet => tp(
      "common.only-of-the-last",
      r.count,
      &[&r.count.to_string(), &r.passed.to_string()],
    ),
  }
}

/// 学习进度页：某题库的成绩折线与「可以去考了」判断。
#[component]
pub fn ExamTrend(history: Vec<ExamRecord>, bank: Bank) -> impl IntoView {
  let Some(readiness) = assess(&history, bank) else {
    return view! {
      <p class="text-sm text-muted-foreground">
        {tf("common.no-class-mock-exam", &[&bank.to_string()])}
        <a href=format!("/exam?bank={bank}") class="ml-1 text-primary underline underline-offset-4">
          {move || t("common.take-one-link")}
        </a>
      </p>
    }
    .into_any();
  };
  let rule = ExamRule::of(bank);
  let records: Vec<ExamRecord> = {
    let all = of_bank(&history, bank);
    all[all.len().saturating_sub(20)..]
      .iter()
      .map(|r| (*r).clone())
      .collect()
  };
  let pass_pct = rule.pass as f64 / rule.total as f64 * 100.0;
  let (w, h) = (600.0_f64, 120.0_f64);
  let lo = records
    .iter()
    .map(ExamRecord::percent)
    .fold(pass_pct, f64::min);
  let lo = ((lo - 10.0) / 10.0).floor().max(0.0) * 10.0;
  let x_of = |i: usize| {
    if records.len() <= 1 {
      w / 2.0
    } else {
      i as f64 / (records.len() - 1) as f64 * w
    }
  };
  let y_of = |pct: f64| h - (pct - lo) / (100.0 - lo) * h;
  let points = records
    .iter()
    .enumerate()
    .map(|(i, r)| format!("{:.1},{:.1}", x_of(i), y_of(r.percent())))
    .collect::<Vec<_>>()
    .join(" ");
  let pass_y = y_of(pass_pct);
  let trend = readiness.trend;
  let trend_class = if trend >= 0.0 {
    "tabular-nums text-win-text"
  } else {
    "tabular-nums text-hp-text"
  };
  view! {
    <div class="space-y-3">
      <div class="flex flex-wrap items-center gap-2 text-sm">
        <span class=format!("border px-2.5 py-0.5 text-xs {}", verdict_class(readiness.verdict))>
          {readiness.verdict.label()}
        </span>
        <span class="text-muted-foreground">{verdict_hint(&readiness, rule.pass, rule.total)}</span>
      </div>
      <div class="flex flex-wrap gap-x-5 gap-y-1 text-xs text-muted-foreground">
        <span>{move || t("common.last")} {readiness.count} {move || t("common.avg")} <span class="text-foreground tabular-nums">{format!("{:.0}%", readiness.average)}</span></span>
        <span>{move || t("common.passed")} <span class="text-foreground tabular-nums">{format!("{} / {}", readiness.passed, readiness.count)}</span></span>
        {(readiness.count >= 2).then(|| view! {
          <span>
            {move || t("common.change")}
            <span class=trend_class>
              {tp("common.percentage-points", trend, &[&(format!("{trend:+.0}")).to_string()])}
            </span>
          </span>
        })}
      </div>
      <svg viewBox=format!("-34 -8 {} {}", w + 42.0, h + 16.0) class="h-auto w-full" role="img" aria-label=move || t("common.exam-accuracy-trend")>
        {[lo, 100.0]
          .into_iter()
          .map(|p| view! {
            <line x1="0" x2=w y1=y_of(p) y2=y_of(p) class="stroke-border" stroke-width="1" vector-effect="non-scaling-stroke" />
            <text x="-6" y=y_of(p) dy="4" text-anchor="end" font-size="11" class="fill-muted-foreground">{format!("{p:.0}%")}</text>
          })
          .collect_view()}
        <text x="-6" y=pass_y dy="4" text-anchor="end" font-size="11" class="fill-gold">{format!("{pass_pct:.0}%")}</text>
        <line x1="0" x2=w y1=pass_y y2=pass_y class="stroke-gold/70" stroke-width="1" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" />
        <polyline points=points fill="none" class="stroke-primary" stroke-width="2" vector-effect="non-scaling-stroke" />
        {records
          .iter()
          .enumerate()
          .map(|(i, r)| {
            let passed = r.correct >= rule.pass;
            view! {
              <circle
                cx=x_of(i)
                cy=y_of(r.percent())
                r="3"
                class=if passed { "fill-win/70" } else { "fill-destructive/70" }
              >
                <title>{format!("{}/{}（{:.0}%）", r.correct, r.total, r.percent())}</title>
              </circle>
            }
          })
          .collect_view()}
      </svg>
      <p class="text-xs text-muted-foreground">
        {tp(
          "common.the-dashed-line-is",
          ham_web_core::exam_history::RECENT,
          &[&(format!("{pass_pct:.0}")).to_string(), &ham_web_core::exam_history::RECENT.to_string()],
        )}
      </p>
    </div>
  }
  .into_any()
}

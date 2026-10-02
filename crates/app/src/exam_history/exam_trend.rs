use crate::i18n::{t, tf};
use ham_web_core::exam_history::{ExamRecord, Readiness, Verdict, assess, of_bank};
use ham_web_core::{Bank, ExamRule};
use leptos::prelude::*;

fn verdict_class(v: Verdict) -> &'static str {
  match v {
    Verdict::Ready => {
      "border-emerald-500/40 bg-emerald-500/15 text-emerald-700 dark:text-emerald-300"
    }
    Verdict::Almost => "border-amber-500/40 bg-amber-500/15 text-amber-700 dark:text-amber-300",
    Verdict::NotYet => "border-red-500/40 bg-red-500/15 text-red-700 dark:text-red-300",
    Verdict::NeedMore => "text-muted-foreground",
  }
}

fn verdict_hint(r: &Readiness, pass: usize, total: usize) -> String {
  match r.verdict {
    Verdict::NeedMore => tf(
      "已考 {} 次，再考几次就能给出判断。",
      &[&r.count.to_string()],
    ),
    Verdict::Ready => tf(
      "最近 {} 次都稳定在合格线（{} / {}）以上，可以报名了。",
      &[&r.count.to_string(), &pass.to_string(), &total.to_string()],
    ),
    Verdict::Almost => tf(
      "最近 {} 次及格 {} 次，再巩固一下薄弱分类。",
      &[&r.count.to_string(), &r.passed.to_string()],
    ),
    Verdict::NotYet => tf(
      "最近 {} 次只及格 {} 次，建议先刷错题与专项练习。",
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
        {tf("还没有 {} 类模拟考试记录。", &[&bank.to_string()])}
        <a href=format!("/exam?bank={bank}") class="ml-1 text-primary underline underline-offset-4">
          {move || t("去考一次 →")}
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
    "font-semibold tabular-nums text-emerald-600"
  } else {
    "font-semibold tabular-nums text-red-600"
  };
  view! {
    <div class="space-y-3">
      <div class="flex flex-wrap items-center gap-2 text-sm">
        <span class=format!("rounded-full border px-2.5 py-0.5 text-xs font-semibold {}", verdict_class(readiness.verdict))>
          {readiness.verdict.label()}
        </span>
        <span class="text-muted-foreground">{verdict_hint(&readiness, rule.pass, rule.total)}</span>
      </div>
      <div class="flex flex-wrap gap-x-5 gap-y-1 text-xs text-muted-foreground">
        <span>{move || t("近 ")} {readiness.count} {move || t(" 次平均 ")} <span class="font-semibold text-foreground tabular-nums">{format!("{:.0}%", readiness.average)}</span></span>
        <span>{move || t("及格 ")} <span class="font-semibold text-foreground tabular-nums">{format!("{} / {}", readiness.passed, readiness.count)}</span></span>
        {(readiness.count >= 2).then(|| view! {
          <span>
            {move || t("变化 ")}
            <span class=trend_class>
              {tf("{} 个百分点", &[&(format!("{trend:+.0}")).to_string()])}
            </span>
          </span>
        })}
      </div>
      <svg viewBox=format!("-34 -8 {} {}", w + 42.0, h + 16.0) class="h-auto w-full" role="img" aria-label=move || t("考试正确率趋势")>
        {[lo, 100.0]
          .into_iter()
          .map(|p| view! {
            <line x1="0" x2=w y1=y_of(p) y2=y_of(p) class="stroke-border" stroke-width="1" vector-effect="non-scaling-stroke" />
            <text x="-6" y=y_of(p) dy="4" text-anchor="end" font-size="11" class="fill-muted-foreground">{format!("{p:.0}%")}</text>
          })
          .collect_view()}
        <text x="-6" y=pass_y dy="4" text-anchor="end" font-size="11" class="fill-amber-600">{format!("{pass_pct:.0}%")}</text>
        <line x1="0" x2=w y1=pass_y y2=pass_y class="stroke-amber-500" stroke-width="1" stroke-dasharray="4 3" vector-effect="non-scaling-stroke" />
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
                class=if passed { "fill-emerald-500" } else { "fill-red-500" }
              >
                <title>{format!("{}/{}（{:.0}%）", r.correct, r.total, r.percent())}</title>
              </circle>
            }
          })
          .collect_view()}
      </svg>
      <p class="text-xs text-muted-foreground">
        {tf("虚线为合格线 {}%；绿点及格、红点不及格。最近 {} 次全部比合格线多答对 2 题以上即判定「可以去考了」。", &[&(format!("{pass_pct:.0}")).to_string(), &ham_web_core::exam_history::RECENT.to_string()])}
      </p>
    </div>
  }
  .into_any()
}

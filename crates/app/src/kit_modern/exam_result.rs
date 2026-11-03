//! 现代风成绩弹层：大号分数 + 通过判定 + 分类变化 + 复盘与分享动作。
//!
//! 与像素版的差别：不画 Boss 判词（RPG 隐喻）、不内嵌历史折线图（像素画布），
//! 分数与分类变化是这块面板的主角。历史趋势仍在 `/exam-history` 页面可查。

use ham_web_core::weak_exam::CategoryDelta;
use ham_web_core::{Bank, ExamScore};
use leptos::prelude::*;

use crate::i18n::{t, tf, tp};
use crate::share_score;

use super::dialog::ModernDialog;

#[component]
pub fn ModernExamResult(
  open: RwSignal<bool>,
  #[prop(into)] score: Signal<ExamScore>,
  #[prop(into)] pass_line: Signal<usize>,
  #[prop(into)] deltas: Signal<Vec<CategoryDelta>>,
  #[prop(into)] weak: Signal<bool>,
  #[prop(into)] wrong_count: Signal<usize>,
  #[prop(into)] wrong_href: Signal<String>,
  #[prop(into)] bank: Signal<Bank>,
) -> impl IntoView {
  let card = move || {
    let (s, b, pl) = (score.get(), bank.get(), pass_line.get());
    share_score::render_score_card(b, s, pl).ok()
  };
  let download = move |_| {
    if let Some(data_url) = card() {
      share_score::download(
        &data_url,
        &format!(
          "ham-exam-{}-{}.png",
          bank.get().as_str(),
          crate::util::local_today()
        ),
      );
    }
  };
  let copy = move |_| {
    if let Some(data_url) = card() {
      leptos::task::spawn_local(async move {
        match share_score::copy_image(&data_url).await {
          Ok(()) => crate::ui::push_success(t("learning.copied-to-clipboard")),
          Err(e) => crate::ui::push_error(tf("learning.copy-failed", &[&e])),
        }
      });
    }
  };

  // 把正确率换算成业余无线电的「信号报告」：R（可读度 1–5）× S（强度 1–9）。
  // 换算规则在 core（`ExamScore::signal_report`，有边界单测）。
  let rst = move || score.get().signal_report();

  view! {
    <ModernDialog open=open title=Signal::derive(move || t("exam.score-title"))>
      <div class="md-score">
        <div class="md-rst">
          <span class="md-rst-value">"R" {move || rst().0.to_string()}</span>
          <span class="md-rst-value">"S" {move || rst().1.to_string()}</span>
          <span class="md-rst-label">{move || t("exam.signal-report")}</span>
        </div>
        <div class="flex items-baseline gap-2">
          <span class="md-score-value">{move || score.get().correct.to_string()}</span>
          <span class="md-muted md-mono">{move || format!("/ {}", score.get().total)}</span>
        </div>
        <span class="md-muted md-sm">
          {move || t("exam.accuracy")} " " {move || score.get().percent()} "%"
        </span>
        {move || {
          let pl = pass_line.get();
          let passed = score.get().is_passed(pl);
          view! {
            <span class=if passed { "md-score-pass" } else { "md-score-fail" }>
              {if passed { t("exam.passed") } else { t("exam.not-passed") }}
              " "
              {tp("exam.pass-line-questions", pl, &[&pl.to_string()])}
            </span>
          }
        }}
        {move || weak.get().then(|| view! { <span class="md-weak-tag">{move || t("exam.a-weak-area-exam")}</span> })}
      </div>

      {move || {
        let d = deltas.get();
        (!d.is_empty()).then(|| {
          view! {
            <ul class="md-delta-list">
              {d
                .into_iter()
                .map(|item| {
                  let change = item.change();
                  let label = match change {
                    Some(c) if c > 0.5 => format!("+{c:.0}pp"),
                    Some(c) if c < -0.5 => format!("{c:.0}pp"),
                    _ => "—".to_owned(),
                  };
                  let tone = match change {
                    Some(c) if c > 0.5 => "md-delta-up",
                    Some(c) if c < -0.5 => "md-delta-down",
                    _ => "md-delta-flat",
                  };
                  let rate = item.exam.rate().unwrap_or(0.0);
                  let width = (rate * 100.0).clamp(0.0, 100.0);
                  view! {
                    <li class="md-delta-row">
                      <span class="md-delta-name">
                        {move || crate::data::core_text_tracked(item.name)}
                      </span>
                      <span class="md-delta-bar">
                        <span class="md-delta-fill" style=format!("width: {width:.0}%")></span>
                      </span>
                      <span class=format!("md-mono {tone}")>{label}</span>
                    </li>
                  }
                })
                .collect_view()}
            </ul>
          }
        })
      }}

      <p class="md-muted md-sm">{move || t("exam.after-submitting-you-can")}</p>

      <div class="md-dialog-actions">
        {move || {
          (wrong_count.get() > 0)
            .then(|| {
              view! {
                <a class="md-btn md-btn-outline" href=move || wrong_href.get()>
                  {tp("exam.retry-mistakes", wrong_count.get(), &[&wrong_count.get().to_string()])}
                </a>
              }
            })
        }}
        <a class="md-btn md-btn-outline" href="/exam-review">
          {move || t("exam.review-question-by-question")}
        </a>
        <button type="button" class="md-btn md-btn-ghost" on:click=move |_| download(())>
          {move || t("learning.download-card")}
        </button>
        <button type="button" class="md-btn md-btn-ghost" on:click=move |_| copy(())>
          {move || t("learning.copy-image")}
        </button>
        <button type="button" class="md-btn md-btn-accent" on:click=move |_| open.set(false)>
          {move || t("exam.keep-browsing")}
        </button>
      </div>
    </ModernDialog>
  }
}

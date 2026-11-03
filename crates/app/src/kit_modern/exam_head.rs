//! 现代风考试页顶部：进度条 + 倒计时 + 薄弱项切换 + 设置，下面一行考试元信息。

use ham_web_core::{Bank, ExamRule};
use leptos::prelude::*;

use crate::i18n::{bank_class, t, tf};

use super::exam_clock::ModernExamClock;
use super::{MIcon, ModernIcon};

#[component]
pub fn ModernExamHead(
  #[prop(into)] percent: Signal<i64>,
  #[prop(into)] weak: Signal<bool>,
  #[prop(into)] version: Signal<Option<String>>,
  #[prop(into)] bank: Signal<Bank>,
  #[prop(into)] rule: Signal<ExamRule>,
  remaining: RwSignal<i64>,
  on_open_settings: Callback<()>,
) -> impl IntoView {
  let href = move || {
    let base = crate::pages::bank_href("/exam", version.get().as_deref(), bank.get());
    if weak.get() {
      base
    } else {
      format!("{base}&mode=weak")
    }
  };

  view! {
    <div class="md-practice-head">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex items-center gap-3">
          <span class="md-h2">{move || t("shell.mock-exam")}</span>
          {move || {
            (rule.get().minutes > 0)
              .then(|| view! { <ModernExamClock remaining=remaining rule=rule /> })
          }}
        </div>
        <div class="flex items-center gap-2">
          <a class="md-chip" href=Signal::derive(href)>
            {move || if weak.get() { t("exam.standard-exam") } else { t("exam.weak-area-exam") }}
          </a>
          <button
            type="button"
            class="md-icon-btn"
            aria-label=move || t("exam.settings")
            on:click=move |_| on_open_settings.run(())
          >
            <ModernIcon kind=MIcon::Settings />
          </button>
        </div>
      </div>

      <div class="md-progress" role="presentation">
        <div
          class="md-progress-fill"
          style=move || format!("width: {}%", percent.get().clamp(0, 100))
        ></div>
      </div>

      <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1 md-sm md-muted">
        <span>{move || t("exam.class-prefix")} {move || bank_class(bank.get().as_str())}</span>
        <span>
          {move || {
            let r = rule.get();
            tf(
              "exam.questions-single-multiple",
              &[&r.total.to_string(), &r.singles.to_string(), &r.multiples.to_string()],
            )
          }}
        </span>
        {move || {
          (rule.get().minutes > 0)
            .then(|| view! { <span>{move || tf("exam.time-limit-min", &[&rule.get().minutes.to_string()])}</span> })
        }}
        {move || weak.get().then(|| view! { <span class="md-weak-tag">{move || t("exam.weak-area-exam")}</span> })}
      </div>
    </div>
  }
}

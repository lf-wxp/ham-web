//! 现代风考试倒计时：等宽数字 + 档位色，不加动画、不加 aria-live（与像素版同一口径）。

use ham_web_core::ExamRule;
use ham_web_core::exam_clock::{ClockTone, clock_tone};
use ham_web_core::text::format_ms;
use leptos::prelude::*;

#[component]
pub fn ModernExamClock(
  #[prop(into)] remaining: Signal<i64>,
  #[prop(into)] rule: Signal<ExamRule>,
) -> impl IntoView {
  view! {
    {move || {
      let ms = remaining.get();
      let tone = match clock_tone(ms, rule.get().duration_ms()) {
        ClockTone::Calm => "md-clock",
        ClockTone::Warn => "md-clock md-clock-warn",
        ClockTone::Danger => "md-clock md-clock-danger",
      };
      view! {
        <span class=tone data-slot="exam-clock">
          {format_ms(ms)}
        </span>
      }
    }}
  }
}

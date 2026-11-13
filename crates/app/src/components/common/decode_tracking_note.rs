//! 解码页的「已修正时钟误差 / 频漂」一行。
//!
//! 只在解码器确实跟踪并修正过时才出现（取整后仍非零，口径见 `ham_web_core::decode_tracking`）；
//! 干净录音不渲染任何东西，免得每次都多写一句「0 ppm」。

use ham_web_core::decode_tracking;
use leptos::prelude::*;

use crate::i18n::tf;

/// 「已修正录音时钟误差 N ppm · 已修正频漂 X Hz/s」，两项各自按需出现。
#[component]
pub fn DecodeTrackingNote(clock_ppm: f64, drift_hz_per_s: f64) -> impl IntoView {
  decode_tracking::summarize(clock_ppm, drift_hz_per_s).map(|tracking| {
    view! {
      <p class="text-xs text-muted-foreground" data-testid="decode-tracking">
        {tracking
          .clock_ppm
          .map(|v| move || tf("tools.decode-tracking-clock", &[&format!("{v:+.0}")]))}
        {tracking
          .clock_ppm
          .zip(tracking.drift_hz_per_s)
          .map(|_| " · ")}
        {tracking
          .drift_hz_per_s
          .map(|v| move || tf("tools.decode-tracking-drift", &[&format!("{v:+.2}")]))}
      </p>
    }
  })
}

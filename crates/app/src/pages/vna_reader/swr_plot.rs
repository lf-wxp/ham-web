//! 驻波比 – 频率曲线：纵轴 1–5（超出的截在 5），画 SWR = 2 参考线。

use ham_web_core::touchstone::Sweep;
use leptos::prelude::*;

use super::{NOTE, fmt_hz};
use crate::i18n::t;

const W: f64 = 320.0;
const H: f64 = 180.0;
const PAD: f64 = 28.0;
/// 纵轴上限：天线调试关心的是 1–3 的区间，5 以上一律视为「不能用」。
const SWR_MAX: f64 = 5.0;

#[component]
pub(super) fn SwrPlot(sweep: Memo<Option<Sweep>>) -> impl IntoView {
  move || {
    sweep.get().map(|s| {
      let (f0, f1) = (
        s.points.first().map_or(0.0, |p| p.f_hz),
        s.points.last().map_or(1.0, |p| p.f_hz),
      );
      let span = (f1 - f0).max(1.0);
      let x = move |f: f64| PAD + (f - f0) / span * (W - 2.0 * PAD);
      let y = |swr: f64| H - PAD - (swr.clamp(1.0, SWR_MAX) - 1.0) / (SWR_MAX - 1.0) * (H - 2.0 * PAD);
      let mut d = String::new();
      for (i, p) in s.points.iter().enumerate() {
        let swr = p.swr();
        if !swr.is_finite() {
          continue;
        }
        d.push_str(&format!(
          "{}{:.2},{:.2} ",
          if i == 0 { "M" } else { "L" },
          x(p.f_hz),
          y(swr)
        ));
      }
      let y2 = y(2.0);
      view! {
        <div class="space-y-1">
          <svg
            viewBox=format!("0 0 {W} {H}")
            class="w-full"
            role="img"
            aria-label=move || t("tools.vna-swr-curve")
            data-slot="vna-swr"
          >
            <rect x=PAD y=PAD width=W - 2.0 * PAD height=H - 2.0 * PAD fill="none" class="stroke-muted-foreground/50" stroke-width="0.5" />
            <line x1=PAD y1=y2 x2=W - PAD y2=y2 class="stroke-amber-500" stroke-width="0.8" stroke-dasharray="4 3" />
            <text x=PAD - 4.0 y=y2 + 3.0 text-anchor="end" class="fill-muted-foreground" font-size="9">"2"</text>
            <text x=PAD - 4.0 y=H - PAD + 3.0 text-anchor="end" class="fill-muted-foreground" font-size="9">"1"</text>
            <text x=PAD - 4.0 y=PAD + 3.0 text-anchor="end" class="fill-muted-foreground" font-size="9">"5"</text>
            <path d=d fill="none" class="stroke-primary" stroke-width="1.5" />
          </svg>
          <div class=format!("flex justify-between {NOTE}")>
            <span>{fmt_hz(f0)}</span>
            <span>{fmt_hz(f1)}</span>
          </div>
        </div>
      }
    })
  }
}

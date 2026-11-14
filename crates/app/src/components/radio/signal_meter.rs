use ham_web_core::rig_panel::{S_SEGMENTS, S9, s_reading};
use leptos::prelude::*;

use crate::i18n::t;

/// 刻度标签：只标奇数格（S1 / S3 … S9、+20、+40），其余列空着给标签溢出用。
const SCALE: [&str; S_SEGMENTS as usize] = [
  "1", "", "3", "", "5", "", "7", "", "9", "", "+20", "", "+40",
];

/// S 表：13 格电平条 + 刻度。S9 = 合格线，以上四格是 +10…+40 dB 的余量。
///
/// `units` 是点亮的格数（`ham_web_core::rig_panel::s_units` 的结果），`None` 表示还没有数据 ——
/// 整条只剩幽灵格，读屏念「无数据」，而不是假装成 S0。读数本身（`S7`）由调用方用 `Readout` 放在旁边。
/// `peak` 是峰值保持：比当前高的那一格画成琥珀（「最好到过这里」），必须是真实记录过的值。
/// 只放在屏幕（`pxl-display`）里。
#[component]
pub fn SignalMeter(
  #[prop(into)] units: Signal<Option<u8>>,
  #[prop(into)] label: Signal<String>,
  #[prop(optional, into)] peak: Signal<Option<u8>>,
) -> impl IntoView {
  view! {
    <div
      role="meter"
      aria-label=move || label.get()
      aria-valuemin="0"
      aria-valuemax=S_SEGMENTS.to_string()
      aria-valuenow=move || units.get().unwrap_or(0).to_string()
      aria-valuetext=move || units.get().map_or_else(|| t("common.no-signal"), s_reading)
      class="space-y-1"
    >
      <div class="pxl-meter-bar" aria-hidden="true">
        {(0..S_SEGMENTS)
          .map(|i| {
            view! {
              <span
                class="pxl-meter-seg"
                data-lit=move || units.get().is_some_and(|u| i < u).then_some("")
                data-plus=(i >= S9).then_some("")
                data-peak=move || {
                  let held = peak.get().is_some_and(|p| p > units.get().unwrap_or(0) && i + 1 == p);
                  held.then_some("")
                }
              ></span>
            }
          })
          .collect_view()}
      </div>
      <div class="pxl-meter-scale" aria-hidden="true">
        {SCALE
          .iter()
          .enumerate()
          .map(|(i, mark)| view! { <span data-plus=(i >= usize::from(S9)).then_some("")>{*mark}</span> })
          .collect_view()}
      </div>
    </div>
  }
}

//! 练习页题卡上方的两块读数屏（canvas「练习 · 收信」，`docs/ui/pixel.md` §3.4）：
//! 题号读数 + LOCK / QRM 两颗灯；「连对信号」S 表（峰值保持 = 本次练习的最高连对）。只在像素风渲染。

use ham_web_core::rig_panel::{S_SEGMENTS, s_reading};
use leptos::prelude::*;

use crate::components::radio::SignalMeter;
use crate::i18n::t;
use crate::ui::{Led, LedState, LedTone, Readout, ReadoutTone};

#[component]
pub(super) fn PracticeRig(
  /// 当前题的序号（0 起）与总题数。
  #[prop(into)]
  index: Signal<usize>,
  #[prop(into)] total: Signal<usize>,
  /// 当前题的判分：`Some(true)` 锁定、`Some(false)` 干扰、`None` 还没判分。
  #[prop(into)]
  verdict: Signal<Option<bool>>,
  #[prop(into)] streak: Signal<u32>,
  #[prop(into)] best: Signal<u32>,
) -> impl IntoView {
  // 连对 N 题 = S 表亮 N 格（满 13 格封顶）；峰值保持是本次练习到过的最高连对。
  let units = move || {
    u8::try_from(streak.get())
      .unwrap_or(u8::MAX)
      .min(S_SEGMENTS)
  };
  let peak = move || u8::try_from(best.get()).unwrap_or(u8::MAX).min(S_SEGMENTS);
  let locked = move || verdict.get() == Some(true);
  let jammed = move || verdict.get() == Some(false);
  view! {
    // 题号与判分灯都已在题卡里有文字（「第 N / M 题」、答案行），这一块是装饰（`aria-hidden`）。
    <div class="pxl-display flex items-center gap-2 px-2 py-1.5" aria-hidden="true">
      <span class="pxl-label pxl-d-dim text-xs">{move || t("learning.rig-q")}</span>
      <Readout text=Signal::derive(move || format!("{:03}", index.get() + 1)) />
      <span class="pxl-label pxl-d-dim text-xs">{move || format!("/ {}", total.get())}</span>
      <span class="ml-auto flex items-center gap-1.5">
        <Led tone=LedTone::Rx state=Signal::derive(move || LedState::lit(locked())) small=true />
        <span class=move || if locked() { "pxl-label pxl-d-ok text-xs" } else { "pxl-label pxl-d-dim text-xs" }>
          {move || t("learning.rig-lock")}
        </span>
        <Led tone=LedTone::Tx state=Signal::derive(move || LedState::lit(jammed())) small=true />
        <span class=move || if jammed() { "pxl-label pxl-d-alarm text-xs" } else { "pxl-label pxl-d-dim text-xs" }>
          {move || t("learning.rig-qrm")}
        </span>
      </span>
    </div>
    <div class="pxl-display space-y-2 p-3">
      <div class="flex items-center justify-between gap-2">
        <span class="pxl-label pxl-d-dim text-xs">{move || t("learning.streak-signal")}</span>
        <Readout
          text=Signal::derive(move || s_reading(units()))
          label=Signal::derive(move || s_reading(units()))
          tone=Signal::derive(move || ReadoutTone::for_s_units(Some(units())))
        />
      </div>
      <SignalMeter
        units=Signal::derive(move || Some(units()))
        peak=Signal::derive(move || Some(peak()))
        label=Signal::derive(move || t("learning.streak-signal"))
      />
    </div>
  }
}

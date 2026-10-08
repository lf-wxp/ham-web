//! 香农容量沙盒：带宽、信噪比与可达速率之间的换算。

use ham_web_core::waveform_lab::{
  SHANNON_LIMIT_EBN0_DB, capacity_bps, ebn0_db, spectral_efficiency,
};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::Slider;

use super::plot::Axes;
use super::{NOTE, PLOT, RESULT, fmt_num};

const W: f64 = 640.0;
const H: f64 = 220.0;
const PAD: f64 = 30.0;
/// 曲线横轴范围（dB）。
const SNR_MIN: f64 = -5.0;
const SNR_MAX: f64 = 40.0;
/// 曲线纵轴上限（bit/s/Hz）。
const SE_MAX: f64 = 13.0;
const STEPS: usize = 120;

/// 速率文案：自动选 bit/s、kbit/s 还是 Mbit/s。
fn fmt_rate(bps: f64) -> String {
  if bps >= 1e6 {
    format!("{:.3} Mbit/s", bps / 1e6)
  } else if bps >= 1e3 {
    format!("{:.2} kbit/s", bps / 1e3)
  } else {
    format!("{bps:.1} bit/s")
  }
}

#[component]
pub(super) fn ShannonSection() -> impl IntoView {
  let bw_khz = RwSignal::new(3.0);
  let snr = RwSignal::new(20.0);

  let se = move || spectral_efficiency(snr.get());
  let cap = move || capacity_bps(bw_khz.get() * 1e3, snr.get());

  let axes = Axes {
    x0: SNR_MIN,
    x1: SNR_MAX,
    y0: 0.0,
    y1: SE_MAX,
    w: W,
    h: H,
    pad: PAD,
  };

  let curve_path = move || {
    let pts: Vec<(f64, f64)> = (0..=STEPS)
      .map(|i| {
        let snr_db = SNR_MIN + (SNR_MAX - SNR_MIN) * i as f64 / STEPS as f64;
        (snr_db, spectral_efficiency(snr_db).min(SE_MAX))
      })
      .collect();
    axes.path(&pts)
  };
  let marker = move || (axes.px(snr.get()), axes.py(se().min(SE_MAX)));

  view! {
    <section id="shannon" class="scroll-mt-24 rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.shannon-capacity-sandbox")}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">
        {move || t("tools.shannon-s-formula-c")}
      </p>

      <div class="grid gap-4 p-4 sm:grid-cols-2">
        <div class="flex flex-col gap-2">
          <span class=NOTE>{move || t("tools.bandwidth-b-khz")}</span>
          <Slider
            value=Signal::derive(move || bw_khz.get())
            on_change=Callback::new(move |v: f64| bw_khz.set(v))
            min=0.1
            max=500.0
            step=0.1
            aria_label=Signal::derive(move || t("tools.bandwidth"))
            aria_valuetext=Signal::derive(move || format!("{:.1} kHz", bw_khz.get()))
          />
        </div>
        <div class="flex flex-col gap-2">
          <span class=NOTE>{move || t("tools.signal-to-noise-ratio-3")}</span>
          <Slider
            value=Signal::derive(move || snr.get())
            on_change=Callback::new(move |v: f64| snr.set(v))
            min=-10.0
            max=40.0
            step=1.0
            aria_label=Signal::derive(move || t("tools.signal-to-noise-ratio"))
            aria_valuetext=Signal::derive(move || format!("{:.0} dB", snr.get()))
          />
        </div>
      </div>

      <div class="px-4 pb-2">
        <div class=NOTE>{move || t("tools.spectral-efficiency-limit-bit")}</div>
        <svg viewBox=format!("0 0 {W} {H}") class=PLOT role="img" aria-label=move || t("tools.shannon-capacity-curve")>
          {axes
            .hlines(&[0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0])
            .into_iter()
            .map(|y| {
              view! {
                <line
                  x1=PAD
                  y1=y
                  x2=W - PAD
                  y2=y
                  class="stroke-muted-foreground/20"
                  stroke-width="1"
                />
              }
            })
            .collect_view()}
          {axes
            .vlines(&[0.0, 10.0, 20.0, 30.0, 40.0])
            .into_iter()
            .map(|x| {
              view! {
                <line
                  x1=x
                  y1=PAD
                  x2=x
                  y2=H - PAD
                  class="stroke-muted-foreground/20"
                  stroke-width="1"
                />
              }
            })
            .collect_view()}
          <path
            d=curve_path
            class="fill-none stroke-primary"
            stroke-width="1.8"
            vector-effect="non-scaling-stroke"
          />
          <line
            x1=move || marker().0
            y1=move || marker().1
            x2=move || marker().0
            y2=H - PAD
            class="stroke-amber-500/70"
            stroke-dasharray="4 3"
            stroke-width="1"
          />
          <line
            x1=PAD
            y1=move || marker().1
            x2=move || marker().0
            y2=move || marker().1
            class="stroke-amber-500/70"
            stroke-dasharray="4 3"
            stroke-width="1"
          />
          <circle
            cx=move || marker().0
            cy=move || marker().1
            r="4.5"
            class="fill-amber-500 stroke-background"
            stroke-width="1.5"
          />
          <text x=PAD y=H - 8.0 class="fill-muted-foreground text-[10px]">
            {move || t("tools.snr-db")}
          </text>
        </svg>
      </div>

      <div class="space-y-2 px-4 pb-4">
        <div class=RESULT>
          {move || {
            tf(
              "tools.capacity-ceiling-spectral-efficiency",
              &[
                &fmt_rate(cap()),
                &fmt_num(se()),
                &fmt_num(ebn0_db(snr.get(), se())),
              ],
            )
          }}
        </div>
        <p class=NOTE>
          {move || {
            tf(
              "tools.the-shannon-limit-is",
              &[&format!("{SHANNON_LIMIT_EBN0_DB:.2}")],
            )
          }}
        </p>
      </div>
    </section>
  }
}

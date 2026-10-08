//! 调制波形、频谱与功率分配。

use ham_web_core::waveform_lab::{
  Modulation, WaveSpec, am_power, carson_bandwidth_hz, spectrum, waveform,
};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Chip, ChipGroup, Slider};

use super::plot::Axes;
use super::{NOTE, PLOT, RESULT, fmt_num};

/// 采样点数（须为 2 的幂，频谱走 FFT）。
const SAMPLES: usize = 1024;
/// 渲染的调制周期数：2 个周期 → 频谱 bin 宽恰为 0.5 fm，边带落在整数 bin 上。
const PERIODS: f64 = 2.0;
const W: f64 = 640.0;
const TIME_H: f64 = 170.0;
const SPEC_H: f64 = 200.0;
const PAD: f64 = 22.0;
/// 调制指数滑块的上下限（AM 的调制度与 FM 的 β 共用一条滑块）。
const INDEX_MIN: f64 = 0.1;
const INDEX_MAX: f64 = 6.0;

/// 调制方式按钮。
const KINDS: [Modulation; 5] = [
  Modulation::Am,
  Modulation::Dsb,
  Modulation::Usb,
  Modulation::Lsb,
  Modulation::Fm,
];

/// 调制方式 → 译文取值函数。
///
/// 返回**函数指针**而不是 key / 中文原文：`check-i18n` 判死条目时只认「`t("key")` 的直接
/// 实参」或「中文原文出现在源码里」，两者经变量转手都会被判成死条目。这里的
/// `t("tools.am")` 是货真价实的字面量实参，转成 `fn` 后在视图闭包里照常响应式求值。
/// chip 的选项值：用语义名而不是下标，重排 [`KINDS`] 不会换掉用户的选择。
fn kind_key(kind: Modulation) -> &'static str {
  match kind {
    Modulation::Am => "am",
    Modulation::Dsb => "dsb",
    Modulation::Usb => "usb",
    Modulation::Lsb => "lsb",
    Modulation::Fm => "fm",
  }
}

fn kind_text(kind: Modulation) -> fn() -> String {
  match kind {
    Modulation::Am => || t("tools.am"),
    Modulation::Dsb => || t("tools.dsb"),
    Modulation::Usb => || t("tools.usb"),
    Modulation::Lsb => || t("tools.lsb"),
    Modulation::Fm => || t("tools.fm"),
  }
}

#[component]
pub(super) fn WaveSection() -> impl IntoView {
  let kind = RwSignal::new(Modulation::Am);
  let ratio = RwSignal::new(20.0);
  let index = RwSignal::new(1.0);

  let is_fm = move || kind.get() == Modulation::Fm;

  let spec = Memo::new(move |_| WaveSpec {
    kind: kind.get(),
    carrier_ratio: ratio.get(),
    index: index.get(),
    periods: PERIODS,
    samples: SAMPLES,
  });

  // 必须用 tracked 的 `.get()`：`get_untracked()` 不会建立依赖，`Memo` 只算一次就永不失效，
  // 切调制方式 / 拖滑块时波形与频谱都不会重画（同一页的 symbol / filter 面板都是 tracked）。
  let wave = Memo::new(move |_| waveform(&spec.get()));
  let bins = Memo::new(move |_| spectrum(&wave.get(), PERIODS));

  // 频谱只画到「载波 ± 边带」一带：画满 0–256 fm 的话载波会缩成一个点。
  let display_max = move || ratio.get() + 2.0 * index.get().max(1.0) + 4.0;

  let time_axes = Axes {
    x0: 0.0,
    x1: PERIODS,
    y0: -1.25,
    y1: 1.25,
    w: W,
    h: TIME_H,
    pad: PAD,
  };
  let time_path = move || {
    let w = wave.get();
    let pts: Vec<(f64, f64)> = w
      .iter()
      .enumerate()
      .step_by(2)
      .map(|(i, &v)| (i as f64 / SAMPLES as f64 * PERIODS, v))
      .collect();
    time_axes.path(&pts)
  };

  let spec_path = move || {
    let sp = bins.get();
    let max = display_max();
    let pts: Vec<(f64, f64)> = sp
      .freqs_fm
      .iter()
      .zip(sp.mag_db.iter())
      .filter(|(f, _)| **f <= max)
      .map(|(&f, &m)| (f, m))
      .collect();
    let axes = Axes {
      x0: 0.0,
      x1: max,
      y0: -80.0,
      y1: 5.0,
      w: W,
      h: SPEC_H,
      pad: PAD,
    };
    axes.path(&pts)
  };
  let spec_cx = move || {
    let axes = Axes {
      x0: 0.0,
      x1: display_max(),
      y0: -80.0,
      y1: 5.0,
      w: W,
      h: SPEC_H,
      pad: PAD,
    };
    axes.px(ratio.get())
  };

  view! {
    <section id="wave" class="scroll-mt-24 rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.modulated-waveform-and-spectrum")}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">
        {move || t("tools.the-horizontal-axis-is")}
      </p>

      <ChipGroup
        value=Signal::derive(move || kind_key(kind.get()).to_owned())
        on_change=Callback::new(move |v: String| {
          if let Some(k) = KINDS.into_iter().find(|k| kind_key(*k) == v) {
            kind.set(k);
          }
        })
        aria_label=Signal::derive(move || t("tools.modulation"))
        class="px-4 pt-3"
      >
        {KINDS
          .iter()
          .map(|&k| view! { <Chip value=kind_key(k).to_owned()>{move || kind_text(k)()}</Chip> })
          .collect_view()}
      </ChipGroup>

      <div class="grid gap-4 p-4 sm:grid-cols-2">
        <div class="flex flex-col gap-2">
          <span class=NOTE>{move || if is_fm() { t("tools.modulation-index-peak-deviation") } else { t("tools.modulation-index-m-0") }}</span>
          <Slider
            value=Signal::derive(move || index.get())
            on_change=Callback::new(move |v: f64| index.set(v))
            min=INDEX_MIN
            max=INDEX_MAX
            step=0.1
            aria_label=Signal::derive(move || t("tools.modulation-index"))
            aria_valuetext=Signal::derive(move || format!("β / m = {:.1}", index.get()))
          />
        </div>
        <div class="flex flex-col gap-2">
          <span class=NOTE>{move || t("tools.carrier-ratio-fc-fm")}</span>
          <Slider
            value=Signal::derive(move || ratio.get())
            on_change=Callback::new(move |v: f64| ratio.set((v / 2.0).round() * 2.0))
            min=10.0
            max=60.0
            step=2.0
            aria_label=Signal::derive(move || t("tools.carrier-ratio"))
            aria_valuetext=Signal::derive(move || format!("fc / fm = {:.0}", ratio.get()))
          />
        </div>
      </div>

      <div class="px-4 pb-2">
        <div class=NOTE>{move || t("tools.time-domain")}</div>
        <svg
          viewBox=format!("0 0 {W} {TIME_H}")
          class=PLOT
          role="img"
          aria-label=move || t("tools.modulated-waveform")
        >
          {time_axes
            .hlines(&[0.0])
            .into_iter()
            .map(|y| {
              view! {
                <line
                  x1=PAD
                  y1=y
                  x2=W - PAD
                  y2=y
                  class="stroke-muted-foreground/30"
                  stroke-width="1"
                />
              }
            })
            .collect_view()}
          {time_axes
            .vlines(&(0..=2).map(|i| i as f64).collect::<Vec<_>>())
            .into_iter()
            .map(|x| {
              view! {
                <line
                  x1=x
                  y1=PAD
                  x2=x
                  y2=TIME_H - PAD
                  class="stroke-muted-foreground/20"
                  stroke-width="1"
                />
              }
            })
            .collect_view()}
          <path
            d=time_path
            class="fill-none stroke-primary"
            stroke-width="1.5"
            vector-effect="non-scaling-stroke"
          />
        </svg>
      </div>

      <div class="px-4 pb-4">
        <div class=NOTE>{move || t("tools.amplitude-spectrum-single-sided")}</div>
        <svg
          viewBox=format!("0 0 {W} {SPEC_H}")
          class=PLOT
          role="img"
          aria-label=move || t("tools.amplitude-spectrum")
        >
          {Axes {
            x0: 0.0,
            x1: 1.0,
            y0: -80.0,
            y1: 5.0,
            w: W,
            h: SPEC_H,
            pad: PAD,
          }
            .hlines(&[0.0, -20.0, -40.0, -60.0])
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
          <line
            x1=spec_cx
            y1=PAD
            x2=spec_cx
            y2=SPEC_H - PAD
            class="stroke-amber-500/60"
            stroke-dasharray="3 3"
            stroke-width="1"
          />
          <path
            d=spec_path
            class="fill-none stroke-primary"
            stroke-width="1.5"
            vector-effect="non-scaling-stroke"
          />
        </svg>
        <p class=NOTE>
          {move || tf("tools.the-amber-dashed-line-2", &[&format!("{:.0}", ratio.get())])}
        </p>
      </div>

      <div class="border-t px-4 py-3">
        {move || {
          if is_fm() {
            let beta = index.get();
            view! {
              <div class="space-y-2">
                <div class=RESULT>
                  {move || {
                    tf(
                      "tools.peak-deviation-f-fm",
                      &[
                        &fmt_num(beta),
                        &fmt_num(beta),
                        &fmt_num(carson_bandwidth_hz(beta, 1.0)),
                      ],
                    )
                  }}
                </div>
                <p class=NOTE>
                  {move || t("tools.fm-sideband-amplitudes-follow")}
                </p>
              </div>
            }
            .into_any()
          } else {
            let p = am_power(index.get());
            let pct = |v: f64| format!("{:.1}%", v / p.total * 100.0);
            view! {
              <div class="space-y-2">
                <div class="flex h-4 overflow-hidden rounded" role="img"
                  aria-label=move || t("tools.power-split-carrier-and")>
                  <div class="bg-primary" style=format!("width: {}", pct(p.carrier))></div>
                  <div class="bg-emerald-500" style=format!("width: {}", pct(p.sideband_each))></div>
                  <div class="bg-sky-500" style=format!("width: {}", pct(p.sideband_each))></div>
                </div>
                <div class=RESULT>
                  {move || {
                    tf(
                      "tools.total-power-carrier-normalised",
                      &[
                        &fmt_num(p.total),
                        &fmt_num(p.sideband_each),
                        &format!("{:.1}%", p.efficiency * 100.0),
                      ],
                    )
                  }}
                </div>
                <p class=NOTE>
                  {move || t("tools.at-100-modulation-m")}
                </p>
              </div>
            }
            .into_any()
          }
        }}
      </div>
    </section>
  }
}

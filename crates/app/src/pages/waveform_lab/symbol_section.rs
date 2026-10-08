//! 星座图与眼图：噪声下的判决点云、成形滤波后的码间干扰。

use ham_web_core::waveform_lab::{Scheme, eye_traces, raised_cosine, scatter};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Chip, ChipGroup, Slider};

use super::plot::Axes;
use super::{NOTE, PLOT, RESULT, fmt_num};

/// 点云数量（够看出簇的形状，又不至于让 DOM 太重）。
const POINTS: usize = 600;
/// 每个符号的采样点数。
const SPS: usize = 16;
/// 成形滤波器的截断符号数。
const SPAN: usize = 6;
/// 眼图轨迹条数。
const TRACES: usize = 40;
/// 固定随机种子：换参数时符号序列不变，眼图与点云才可比较。
const SEED: u64 = 0x5EED_2026_1006;

const CONST_W: f64 = 340.0;
const CONST_H: f64 = 340.0;
const EYE_W: f64 = 640.0;
const EYE_H: f64 = 200.0;
const PAD: f64 = 16.0;

const SCHEMES: [Scheme; 3] = [Scheme::Bpsk, Scheme::Qpsk, Scheme::Qam16];

/// chip 的选项值：用语义名而不是下标，重排 [`SCHEMES`] 不会换掉用户的选择。
fn scheme_key(s: Scheme) -> &'static str {
  match s {
    Scheme::Bpsk => "bpsk",
    Scheme::Qpsk => "qpsk",
    Scheme::Qam16 => "qam16",
  }
}

fn scheme_label(s: Scheme) -> &'static str {
  match s {
    Scheme::Bpsk => "BPSK",
    Scheme::Qpsk => "QPSK",
    Scheme::Qam16 => "16QAM",
  }
}

#[component]
pub(super) fn SymbolSection() -> impl IntoView {
  let scheme = RwSignal::new(Scheme::Qpsk);
  let snr = RwSignal::new(12.0);
  let beta = RwSignal::new(0.35);

  let cloud = Memo::new(move |_| scatter(scheme.get(), snr.get(), POINTS, SEED));
  let ideal = Memo::new(move |_| scheme.get().constellation());
  let eye = Memo::new(move |_| eye_traces(beta.get(), SPS, SPAN, TRACES, SEED));

  let const_axes = Axes {
    x0: -1.5,
    x1: 1.5,
    y0: -1.5,
    y1: 1.5,
    w: CONST_W,
    h: CONST_H,
    pad: PAD,
  };
  let eye_axes = Axes {
    x0: 0.0,
    x1: (2 * SPS) as f64,
    y0: -1.8,
    y1: 1.8,
    w: EYE_W,
    h: EYE_H,
    pad: PAD,
  };

  view! {
    <section id="symbols" class="scroll-mt-24 rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.constellation-and-eye-diagram")}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">
        {move || t("tools.a-constellation-shows-how")}
      </p>

      <ChipGroup
        value=Signal::derive(move || scheme_key(scheme.get()).to_owned())
        on_change=Callback::new(move |v: String| {
          if let Some(s) = SCHEMES.into_iter().find(|s| scheme_key(*s) == v) {
            scheme.set(s);
          }
        })
        aria_label=Signal::derive(move || t("tools.constellation-mapping"))
        class="px-4 pt-3"
      >
        {SCHEMES
          .iter()
          .map(|&s| view! { <Chip value=scheme_key(s).to_owned()>{scheme_label(s)}</Chip> })
          .collect_view()}
      </ChipGroup>

      <div class="grid gap-4 p-4 sm:grid-cols-[auto_1fr] sm:items-start">
        <svg
          viewBox=format!("0 0 {CONST_W} {CONST_H}")
          class="mx-auto w-full max-w-[340px]"
          role="img"
          aria-label=move || t("tools.constellation-diagram")
        >
          <line
            x1=PAD
            y1=CONST_H / 2.0
            x2=CONST_W - PAD
            y2=CONST_H / 2.0
            class="stroke-muted-foreground/30"
            stroke-width="1"
          />
          <line
            x1=CONST_W / 2.0
            y1=PAD
            x2=CONST_W / 2.0
            y2=CONST_H - PAD
            class="stroke-muted-foreground/30"
            stroke-width="1"
          />
          {move || {
            cloud
              .get()
              .into_iter()
              .map(|(re, im)| {
                view! {
                  <circle
                    cx=const_axes.px(re)
                    cy=const_axes.py(im)
                    r="1.9"
                    class="fill-primary/45"
                  />
                }
              })
              .collect_view()
          }}
          {move || {
            ideal
              .get()
              .into_iter()
              .map(|(re, im)| {
                view! {
                  <circle
                    cx=const_axes.px(re)
                    cy=const_axes.py(im)
                    r="4.5"
                    class="fill-none stroke-amber-500"
                    stroke-width="1.6"
                  />
                }
              })
              .collect_view()
          }}
        </svg>

        <div class="flex flex-col gap-3">
          <div class="flex flex-col gap-2">
            <span class=NOTE>{move || t("tools.signal-to-noise-ratio-2")}</span>
            <Slider
              value=Signal::derive(move || snr.get())
              on_change=Callback::new(move |v: f64| snr.set(v))
              min=-5.0
              max=25.0
              step=1.0
              aria_label=Signal::derive(move || t("tools.signal-to-noise-ratio"))
              aria_valuetext=Signal::derive(move || format!("{:.0} dB", snr.get()))
            />
          </div>
          <div class=RESULT>
            {move || {
              let bits = f64::from(scheme.get().bits_per_symbol());
              // 符号功率归一到 1 时，Eb/N0 = Es/N0 − 10·lg(每符号比特数)。
              let ebn0 = snr.get() - 10.0 * bits.log2();
              tf(
                "tools.bit-per-symbol-equivalent",
                &[
                  scheme_label(scheme.get()),
                  &format!("{bits:.0}"),
                  &fmt_num(ebn0),
                ],
              )
            }}
          </div>
          <p class=NOTE>
            {move || t("tools.the-amber-hollow-circles")}
          </p>
        </div>
      </div>

      <div class="border-t px-4 py-3">
        <div class="flex flex-wrap items-center gap-3">
          <span class=NOTE>{move || t("tools.pulse-shaping-roll-off-2")}</span>
          <Slider
            value=Signal::derive(move || beta.get())
            on_change=Callback::new(move |v: f64| beta.set(v))
            min=0.0
            max=1.0
            step=0.05
            aria_label=Signal::derive(move || t("tools.pulse-shaping-roll-off"))
            aria_valuetext=Signal::derive(move || format!("β = {:.2}", beta.get()))
            class="w-48"
          />
          <span class=NOTE>
            {move || {
              tf(
                "tools.occupied-bandwidth-1-2",
                &[&fmt_num(raised_cosine(0.5, beta.get()))],
              )
            }}
          </span>
        </div>
        <svg
          viewBox=format!("0 0 {EYE_W} {EYE_H}")
          class=PLOT
          role="img"
          aria-label=move || t("tools.eye-diagram")
        >
          {eye_axes
            .hlines(&[-1.0, 0.0, 1.0])
            .into_iter()
            .map(|y| {
              view! {
                <line
                  x1=PAD
                  y1=y
                  x2=EYE_W - PAD
                  y2=y
                  class="stroke-muted-foreground/20"
                  stroke-width="1"
                />
              }
            })
            .collect_view()}
          <line
            x1=eye_axes.px(SPS as f64)
            y1=PAD
            x2=eye_axes.px(SPS as f64)
            y2=EYE_H - PAD
            class="stroke-amber-500/70"
            stroke-dasharray="4 3"
            stroke-width="1"
          />
          {move || {
            eye
              .get()
              .into_iter()
              .map(|trace| {
                let pts: Vec<(f64, f64)> = trace
                  .iter()
                  .enumerate()
                  .map(|(i, &v)| (i as f64, v))
                  .collect();
                let d = eye_axes.path(&pts);
                view! {
                  <path
                    d=d
                    class="fill-none stroke-primary/45"
                    stroke-width="1"
                    vector-effect="non-scaling-stroke"
                  />
                }
              })
              .collect_view()
          }}
        </svg>
        <p class=NOTE>
          {move || t("tools.the-amber-dashed-line")}
        </p>
      </div>
    </section>
  }
}

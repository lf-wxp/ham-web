//! 滤波器响应曲线：幅频、相频、群延迟三条曲线共用一次频点扫描。

use ham_web_core::filter_design::FilterKind;
use ham_web_core::waveform_lab::{FilterResponse, FilterSpec, response_at, response_curve_log};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Chip, ChipGroup, ControlSize, NumberField};

use super::plot::Axes;
use super::{NOTE, PLOT, RESULT};

const W: f64 = 640.0;
const MAG_H: f64 = 190.0;
const PHASE_H: f64 = 150.0;
const DELAY_H: f64 = 150.0;
const PAD: f64 = 26.0;
/// 扫描点数：步长足够细，相位解卷绕才可靠。
const POINTS: usize = 260;

const KINDS: [FilterKind; 4] = [
  FilterKind::LowPass,
  FilterKind::HighPass,
  FilterKind::BandPass,
  FilterKind::BandStop,
];

/// chip 的选项值：用语义名而不是下标，重排 [`KINDS`] 不会换掉用户的选择。
fn kind_key(kind: FilterKind) -> &'static str {
  match kind {
    FilterKind::LowPass => "low-pass",
    FilterKind::HighPass => "high-pass",
    FilterKind::BandPass => "band-pass",
    FilterKind::BandStop => "band-stop",
  }
}

/// chip 的选项值（与 `pages/tools/filter_design.rs` 用的是同一套语义名）。
fn response_key(response: FilterResponse) -> &'static str {
  match response {
    FilterResponse::Butterworth => "butterworth",
    FilterResponse::Chebyshev1 => "chebyshev-1",
    FilterResponse::Chebyshev2 => "chebyshev-2",
  }
}

/// 滤波器类型 → 译文取值函数（见 `super::wave_section::kind_text` 关于「为什么返回函数指针」）。
fn kind_text(kind: FilterKind) -> fn() -> String {
  match kind {
    FilterKind::LowPass => || t("knowledge.low-pass"),
    FilterKind::HighPass => || t("knowledge.high-pass"),
    FilterKind::BandPass => || t("knowledge.band-pass"),
    FilterKind::BandStop => || t("knowledge.band-stop"),
  }
}

const RESPONSES: [FilterResponse; 3] = [
  FilterResponse::Butterworth,
  FilterResponse::Chebyshev1,
  FilterResponse::Chebyshev2,
];

/// 切换原型时的波纹默认值：Butterworth 无波纹参数，两类 Chebyshev 各取常用档。
fn default_ripple(response: FilterResponse) -> f64 {
  match response {
    FilterResponse::Butterworth => 0.0,
    FilterResponse::Chebyshev1 => 1.0,
    FilterResponse::Chebyshev2 => 40.0,
  }
}

/// Chebyshev I 的通带波纹候选（dB）。
const CHEBY1_RIPPLES: [f64; 4] = [0.1, 0.5, 1.0, 3.0];
/// Chebyshev II 的阻带最小衰减候选（dB）。
const CHEBY2_RIPPLES: [f64; 4] = [20.0, 40.0, 60.0, 80.0];

/// 频率文案：≥1 MHz 用 MHz，否则用 kHz。
fn fmt_freq(hz: f64) -> String {
  if hz >= 1e6 {
    format!("{:.3} MHz", hz / 1e6)
  } else if hz >= 1e3 {
    format!("{:.0} kHz", hz / 1e3)
  } else {
    format!("{hz:.0} Hz")
  }
}

#[component]
pub(super) fn FilterSection() -> impl IntoView {
  let kind = RwSignal::new(FilterKind::LowPass);
  let response = RwSignal::new(FilterResponse::Butterworth);
  let ripple_db = RwSignal::new(0.0);
  let order = RwSignal::new(3usize);
  // 数值录入沿用项目习惯：`String` 承载，空串视为未填写。
  let fc_mhz = RwSignal::new(String::from("7.1"));
  let bw_khz = RwSignal::new(String::from("500"));

  let is_band = move || matches!(kind.get(), FilterKind::BandPass | FilterKind::BandStop);
  let fc_hz = move || fc_mhz.get().trim().parse::<f64>().unwrap_or(7.1) * 1e6;
  let bw_hz = move || bw_khz.get().trim().parse::<f64>().unwrap_or(500.0) * 1e3;

  let spec = Memo::new(move |_| FilterSpec {
    kind: kind.get(),
    response: response.get(),
    order: order.get(),
    ripple_db: ripple_db.get(),
    fc_hz: fc_hz(),
    bw_hz: bw_hz(),
  });

  // 扫描区间：低通 / 高通看两个十倍频程；带通 / 带阻围绕中心频率看 ±3 个带宽。
  let range = move || {
    let fc = fc_hz();
    if is_band() {
      let bw = bw_hz();
      ((fc - 3.0 * bw).max(fc * 0.05), fc + 3.0 * bw)
    } else {
      (fc / 100.0, fc * 100.0)
    }
  };

  let curve = Memo::new(move |_| {
    // 读 `spec`（Memo）而不是各信号本身：`spec` 已依赖类型 / 阶数 / 频率 / 带宽，
    // 这样改阶数也会触发重算。
    let s = spec.get();
    let (lo, hi) = range();
    // 频率按对数均匀取点：低通 / 高通的十倍频程与带通 / 带阻的窄带都能兼顾。
    // 幅频 / 相频 / 群延迟（含相位解卷绕与中心差分）统一由 core 算 —— 这里以前自己
    // 抄了一份，那份没有单测，core 里有单测的那份反倒没人用。
    response_curve_log(&s, lo, hi, POINTS)
  });

  let x_axes = move |h: f64| {
    let (lo, hi) = range();
    Axes {
      x0: lo.log10(),
      x1: hi.log10(),
      y0: -80.0,
      y1: 5.0,
      w: W,
      h,
      pad: PAD,
    }
  };

  let mag_path = move || {
    let pts: Vec<(f64, f64)> = curve
      .get()
      .iter()
      .map(|p| (p.f_hz.log10(), p.mag_db))
      .collect();
    x_axes(MAG_H).path(&pts)
  };

  let phase_axes = move || {
    let c = curve.get();
    let lo = c.iter().map(|p| p.phase_deg).fold(f64::INFINITY, f64::min);
    let hi = c
      .iter()
      .map(|p| p.phase_deg)
      .fold(f64::NEG_INFINITY, f64::max);
    let margin = ((hi - lo) * 0.05).max(1.0);
    Axes {
      x0: x_axes(PHASE_H).x0,
      x1: x_axes(PHASE_H).x1,
      y0: lo - margin,
      y1: hi + margin,
      w: W,
      h: PHASE_H,
      pad: PAD,
    }
  };
  let phase_path = move || {
    let axes = phase_axes();
    let pts: Vec<(f64, f64)> = curve
      .get()
      .iter()
      .map(|p| (p.f_hz.log10(), p.phase_deg))
      .collect();
    axes.path(&pts)
  };

  let delay_axes = move || {
    let c = curve.get();
    let hi = c
      .iter()
      .map(|p| p.group_delay_s * 1e6)
      .fold(f64::NEG_INFINITY, f64::max);
    let lo = c
      .iter()
      .map(|p| p.group_delay_s * 1e6)
      .fold(f64::INFINITY, f64::min);
    let pad = ((hi - lo) * 0.05).max(0.01);
    Axes {
      x0: x_axes(DELAY_H).x0,
      x1: x_axes(DELAY_H).x1,
      y0: lo - pad,
      y1: hi + pad,
      w: W,
      h: DELAY_H,
      pad: PAD,
    }
  };
  let delay_path = move || {
    let axes = delay_axes();
    let pts: Vec<(f64, f64)> = curve
      .get()
      .iter()
      .map(|p| (p.f_hz.log10(), p.group_delay_s * 1e6))
      .collect();
    axes.path(&pts)
  };

  // 刻度位置（数据域 = log10(f)）与文案。
  let ticks = move || {
    let fc = fc_hz();
    let (lo, hi) = range();
    let mut list = vec![fc * 0.01, fc * 0.1, fc, fc * 10.0, fc * 100.0];
    if is_band() {
      let bw = bw_hz();
      list.push(fc - bw);
      list.push(fc + bw);
    }
    list.retain(|f| *f >= lo && *f <= hi);
    list.sort_by(f64::total_cmp);
    list.dedup_by(|a, b| (*a - *b).abs() < lo * 1e-6);
    list
  };

  view! {
    <section id="filter" class="scroll-mt-24 rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.filter-response-curves")}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">
        {move || t("tools.all-three-prototypes-butterworth")}
      </p>

      <div class="flex flex-wrap items-center gap-2 px-4 pt-3">
        <ChipGroup
          value=Signal::derive(move || kind_key(kind.get()).to_owned())
          on_change=Callback::new(move |v: String| {
            if let Some(k) = KINDS.into_iter().find(|k| kind_key(*k) == v) {
              kind.set(k);
            }
          })
          aria_label=Signal::derive(move || t("tools.filter-type"))
        >
          {KINDS
            .iter()
            .map(|&k| view! { <Chip value=kind_key(k).to_owned()>{move || kind_text(k)()}</Chip> })
            .collect_view()}
        </ChipGroup>
        <span class="ml-2 text-xs text-muted-foreground">{move || t("tools.order")}</span>
        <ChipGroup
          value=Signal::derive(move || order.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(n) = v.parse::<usize>() {
              order.set(n);
            }
          })
          aria_label=Signal::derive(move || t("tools.order"))
        >
          {(1..=5usize)
            .map(|n| view! { <Chip value=n.to_string()>{n}</Chip> })
            .collect_view()}
        </ChipGroup>
      </div>

      <ChipGroup
        value=Signal::derive(move || response_key(response.get()).to_owned())
        on_change=Callback::new(move |v: String| {
          if let Some(r) = RESPONSES.into_iter().find(|r| response_key(*r) == v) {
            // 切换原型时把波纹参数换到该原型的常用档：1 dB 的通带波纹搬到
            // Chebyshev II 上会变成一个几乎没有阻带的滤波器。
            response.set(r);
            ripple_db.set(default_ripple(r));
          }
        })
        aria_label=Signal::derive(move || t("tools.filter-prototype"))
        class="px-4 pt-2"
      >
        {RESPONSES
          .iter()
          .map(|&r| {
            // 逐条写成字面量：`i18n-check` 只采集调用点上的字符串字面量，
            // 走函数返回值会漏统计（词条仍能翻译，但覆盖率对不上）。
            let label = match r {
              FilterResponse::Butterworth => t("tools.butterworth-maximally-flat"),
              FilterResponse::Chebyshev1 => t("tools.chebyshev-i-passband-ripple"),
              FilterResponse::Chebyshev2 => t("tools.chebyshev-ii-stopband-ripple"),
            };
            view! { <Chip value=response_key(r).to_owned()>{label}</Chip> }
          })
          .collect_view()}
      </ChipGroup>

      {move || {
        let r = response.get();
        if r == FilterResponse::Butterworth {
          return ().into_any();
        }
        let cheby1 = r == FilterResponse::Chebyshev1;
        let values = if cheby1 {
          &CHEBY1_RIPPLES
        } else {
          &CHEBY2_RIPPLES
        };
        view! {
          <div class="flex flex-wrap items-center gap-2 px-4 pt-2">
            <span class="text-xs text-muted-foreground">
              {move || {
                if cheby1 {
                  t("tools.passband-ripple-db")
                } else {
                  t("tools.minimum-stopband-attenuation-db")
                }
              }}
            </span>
            <ChipGroup
              value=Signal::derive(move || ripple_db.get().to_string())
              on_change=Callback::new(move |v: String| {
                if let Ok(v) = v.parse::<f64>() {
                  ripple_db.set(v);
                }
              })
              aria_label=Signal::derive(move || {
                if cheby1 {
                  t("tools.passband-ripple-db")
                } else {
                  t("tools.minimum-stopband-attenuation-db")
                }
              })
            >
              {values
                .iter()
                .map(|&v| {
                  let label = if v.fract() == 0.0 {
                    format!("{v:.0} dB")
                  } else {
                    format!("{v:.1} dB")
                  };
                  view! { <Chip value=v.to_string()>{label}</Chip> }
                })
                .collect_view()}
            </ChipGroup>
          </div>
        }
        .into_any()
      }}

      <div class="grid gap-3 p-4 sm:grid-cols-2">
        <label class="flex flex-col gap-1.5">
          <span class=NOTE>{move || if is_band() { t("tools.centre-frequency-mhz") } else { t("tools.cut-off-frequency-mhz") }}</span>
          <NumberField
            value=Signal::derive(move || fc_mhz.get())
            on_change=Callback::new(move |v: String| fc_mhz.set(v))
            step=0.1
            min=0.01
            max=6000.0
            size=ControlSize::Sm
            aria_label=Signal::derive(move || t("tools.frequency-mhz"))
          />
        </label>
        <label class="flex flex-col gap-1.5">
          <span class=NOTE>{move || t("tools.bandwidth-khz-band-pass")}</span>
          <NumberField
            value=Signal::derive(move || bw_khz.get())
            on_change=Callback::new(move |v: String| bw_khz.set(v))
            step=10.0
            min=0.1
            max=100000.0
            size=ControlSize::Sm
            aria_label=Signal::derive(move || t("tools.bandwidth-khz"))
          />
        </label>
      </div>

      <div class="px-4 pb-2">
        <div class=NOTE>{move || t("tools.magnitude-response-db")}</div>
        <svg
          viewBox=format!("0 0 {W} {MAG_H}")
          class=PLOT
          role="img"
          aria-label=move || t("tools.magnitude-response")
        >
          {move || {
            x_axes(MAG_H)
              .hlines(&[0.0, -3.0103, -20.0, -40.0, -60.0])
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
              .collect_view()
          }}
          {move || {
            let axes = x_axes(MAG_H);
            ticks()
              .into_iter()
              .map(|f| {
                let x = axes.px(f.log10());
                view! {
                  <g>
                    <line
                      x1=x
                      y1=PAD
                      x2=x
                      y2=MAG_H - PAD
                      class="stroke-muted-foreground/25"
                      stroke-width="1"
                    />
                    <text
                      x=x
                      y=MAG_H - 6.0
                      text-anchor="middle"
                      class="fill-muted-foreground text-[10px]"
                    >
                      {fmt_freq(f)}
                    </text>
                  </g>
                }
              })
              .collect_view()
          }}
          <path
            d=mag_path
            class="fill-none stroke-primary"
            stroke-width="1.6"
            vector-effect="non-scaling-stroke"
          />
        </svg>
      </div>

      <div class="grid gap-3 px-4 pb-2 sm:grid-cols-2">
        <div>
          <div class=NOTE>{move || t("tools.phase-response-degrees-unwrapped")}</div>
          <svg
            viewBox=format!("0 0 {W} {PHASE_H}")
            class=PLOT
            role="img"
            aria-label=move || t("tools.phase-response")
          >
            <path
              d=phase_path
              class="fill-none stroke-sky-500"
              stroke-width="1.6"
              vector-effect="non-scaling-stroke"
            />
          </svg>
        </div>
        <div>
          <div class=NOTE>{move || t("tools.group-delay-s")}</div>
          <svg
            viewBox=format!("0 0 {W} {DELAY_H}")
            class=PLOT
            role="img"
            aria-label=move || t("tools.group-delay")
          >
            <path
              d=delay_path
              class="fill-none stroke-emerald-500"
              stroke-width="1.6"
              vector-effect="non-scaling-stroke"
            />
          </svg>
        </div>
      </div>

      <div class="space-y-2 px-4 pb-4">
        <div class=RESULT>
          {move || {
            let s = spec.get();
            let (lo, hi) = range();
            let (at_lo, _) = response_at(&s, lo);
            let (at_hi, _) = response_at(&s, hi);
            // 中心 / 截止点：带通的插损、带阻的抑制深度。
            // 带阻在 f₀ 处是精确传输零点，偏 1e-6 倍频避开 0/0。
            let (at_fc, _) = response_at(&s, s.fc_hz * 1.000_001);
            // 原型名是通用型号，不进词典；波纹值随原型给出不同量纲。
            let proto = match s.response {
              FilterResponse::Butterworth => "Butterworth".to_owned(),
              FilterResponse::Chebyshev1 => format!("Chebyshev I · {:.1} dB", s.ripple_db),
              FilterResponse::Chebyshev2 => format!("Chebyshev II · {:.0} dB", s.ripple_db),
            };
            // 阶数限定语也必须走词典：直接塞中文原文会在 en / es 下漏出中文。
            // 两条 `t()` 都写成字面量 key，`check-i18n` 的覆盖率与死条目检查才认得出。
            let order_label = if is_band() {
              t("tools.order-2n")
            } else {
              t("tools.order-n")
            };
            tf(
              "tools.centre-cutoff-sweep-ends",
              &[
                &format!("{at_fc:.1} dB"),
                &format!("{at_lo:.1} dB"),
                &format!("{at_hi:.1} dB"),
                &format!("{:.0}", s.order),
                &order_label,
                &proto,
              ],
            )
          }}
        </div>
        <p class=NOTE>
          {move || t("tools.group-delay-describes-how")}
        </p>
      </div>
    </section>
  }
}

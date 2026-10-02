use crate::i18n::t;
use ham_web_core::antenna_pattern::{PatternKind, elevation_pattern, pattern};
use leptos::prelude::*;

const SIZE: f64 = 300.0;
const MAX_R: f64 = SIZE / 2.0 - 26.0;
const RINGS: [f64; 4] = [0.25, 0.5, 0.75, 1.0];

/// 方位角图类型。
const AZIMUTH: &[PatternKind] = &[
  PatternKind::DipoleE,
  PatternKind::DipoleH,
  PatternKind::Yagi2,
  PatternKind::Yagi,
  PatternKind::Yagi4,
  PatternKind::SquareLoop,
  PatternKind::DeltaLoop,
];
/// 仰角图类型。
const ELEVATION: &[PatternKind] = &[
  PatternKind::Vertical,
  PatternKind::VerticalGP,
  PatternKind::DipoleEl,
  PatternKind::Loop,
];

/// 从 `(cx + r, cy)`（0°）到 `(cx, cy - r)`（90°）的 1/4 圆弧。
fn quarter_arc(cx: f64, cy: f64, r: f64) -> String {
  format!(
    "M {:.1} {:.1} A {r:.1} {r:.1} 0 0 0 {:.1} {:.1}",
    cx + r,
    cy,
    cx,
    cy - r
  )
}

/// 交互式极坐标方向图：方位角（偶极子 E/H 面、Yagi）与仰角（垂直天线、偶极子、水平环）。
#[component]
pub(super) fn PatternPlot() -> impl IntoView {
  let kind = RwSignal::new(PatternKind::DipoleE);
  let height_wl = RwSignal::new(0.5);
  let cx = SIZE / 2.0;
  let cy = SIZE / 2.0;

  let path = move || {
    let k = kind.get();
    let pts = if k == PatternKind::DipoleEl {
      elevation_pattern(height_wl.get(), 72)
    } else {
      pattern(k, 72)
    };
    let mut d = String::new();
    for (i, (theta, g)) in pts.iter().enumerate() {
      let t = theta.to_radians();
      let r = g * MAX_R;
      let x = cx + r * t.cos();
      let y = cy - r * t.sin();
      if i == 0 {
        d.push_str(&format!("M {x:.1} {y:.1}"));
      } else {
        d.push_str(&format!(" L {x:.1} {y:.1}"));
      }
    }
    d.push_str(" Z");
    d
  };

  let tab_class = move |active: bool| {
    if active {
      "rounded-md bg-primary px-2.5 py-1.5 text-xs font-medium text-primary-foreground"
    } else {
      "rounded-md px-2.5 py-1.5 text-xs font-medium text-muted-foreground hover:bg-accent"
    }
  };

  let button = |k: PatternKind| {
    view! {
      <button
        type="button"
        class=move || tab_class(kind.get() == k)
        on:click=move |_| kind.set(k)
      >
        {k.label()}
      </button>
    }
  };

  view! {
    <div class="grid gap-4 sm:grid-cols-[auto_1fr] sm:items-center">
      <div class="flex flex-col gap-2 sm:w-40">
        <div class="text-[10px] font-semibold text-muted-foreground">{move || t("方位角")}</div>
        {AZIMUTH.iter().map(|&k| button(k)).collect_view()}
        <div class="mt-1 text-[10px] font-semibold text-muted-foreground">{move || t("仰角")}</div>
        {ELEVATION.iter().map(|&k| button(k)).collect_view()}
        {move || {
          (kind.get() == PatternKind::DipoleEl).then(|| {
            view! {
              <div class="mt-2 rounded-lg border bg-muted/30 px-3 py-2">
                <div class="flex items-center justify-between text-[10px] text-muted-foreground">
                  <span>{move || t("架高")}</span>
                  <span class="font-mono tabular-nums">{format!("{:.2} λ", height_wl.get())}</span>
                </div>
                <input
                  type="range"
                  min="0.1"
                  max="1.0"
                  step="0.05"
                  aria-label=move || t("架高（波长）")
                  prop:value=move || height_wl.get()
                  on:input=move |e| {
                    if let Ok(v) = event_target_value(&e).parse::<f64>() {
                      height_wl.set(v.clamp(0.1, 1.0));
                    }
                  }
                  class="mt-1 w-full accent-primary"
                />
              </div>
            }
          })
        }}
      </div>
      <svg
        viewBox=format!("0 0 {SIZE} {SIZE}")
        class="mx-auto w-full max-w-[320px]"
        role="img"
        aria-label=move || t("天线方向图")
      >
        {move || {
          if kind.get().is_elevation() {
            // 仰角图：1/4 圆弧 + 地平线/天顶轴 + 0°/45°/90° 标注
            view! {
              {RINGS
                .iter()
                .map(|&g| {
                  view! {
                    <path
                      d=quarter_arc(cx, cy, g * MAX_R)
                      class="fill-none stroke-muted-foreground/30"
                      stroke-width="1"
                      vector-effect="non-scaling-stroke"
                    />
                  }
                })
                .collect_view()}
              <line x1=cx y1=cy x2=cx + MAX_R y2=cy class="stroke-muted-foreground/40" stroke-width="1" vector-effect="non-scaling-stroke" />
              <line x1=cx y1=cy x2=cx y2=cy - MAX_R class="stroke-muted-foreground/40" stroke-width="1" vector-effect="non-scaling-stroke" />
              <text x=cx + MAX_R + 4.0 y=cy + 3.0 font-size="9" class="fill-muted-foreground">"0°"</text>
              <text
                x=cx + MAX_R * std::f64::consts::FRAC_1_SQRT_2 + 4.0
                y=cy - MAX_R * std::f64::consts::FRAC_1_SQRT_2
                font-size="9"
                class="fill-muted-foreground"
              >
                "45°"
              </text>
              <text x=cx - 4.0 y=cy - MAX_R - 4.0 text-anchor="middle" font-size="9" class="fill-muted-foreground">
                "90°"
              </text>
            }
            .into_any()
          } else {
            // 方位角图：完整圆 + 十字轴 + 0°/90°/180°/270° 标注
            view! {
              {RINGS
                .iter()
                .map(|&g| {
                  view! {
                    <circle
                      cx=cx
                      cy=cy
                      r=g * MAX_R
                      class="fill-none stroke-muted-foreground/30"
                      stroke-width="1"
                      vector-effect="non-scaling-stroke"
                    />
                  }
                })
                .collect_view()}
              <line x1=cx - MAX_R y1=cy x2=cx + MAX_R y2=cy class="stroke-muted-foreground/40" stroke-width="1" vector-effect="non-scaling-stroke" />
              <line x1=cx y1=cy - MAX_R x2=cx y2=cy + MAX_R class="stroke-muted-foreground/40" stroke-width="1" vector-effect="non-scaling-stroke" />
              <text x=cx + MAX_R + 4.0 y=cy + 3.0 font-size="9" class="fill-muted-foreground">"0°"</text>
              <text x=cx y=cy - MAX_R - 4.0 text-anchor="middle" font-size="9" class="fill-muted-foreground">
                "90°"
              </text>
              <text x=cx - MAX_R - 4.0 y=cy + 3.0 text-anchor="end" font-size="9" class="fill-muted-foreground">
                "180°"
              </text>
              <text x=cx y=cy + MAX_R + 12.0 text-anchor="middle" font-size="9" class="fill-muted-foreground">
                "270°"
              </text>
            }
            .into_any()
          }
        }}
        <path
          d=path
          class="fill-primary/15 stroke-primary"
          stroke-width="2"
          vector-effect="non-scaling-stroke"
        />
      </svg>
    </div>
  }
}

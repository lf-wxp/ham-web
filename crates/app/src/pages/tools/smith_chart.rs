//! 史密斯圆图：复阻抗定位与驻波比 / 反射系数 / 回波损耗实时显示。

use ham_web_core::smith::{gamma, reactance_arc, resistance_circle, swr_and_return_loss};
use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

const SIZE: f64 = 320.0;
const RADIUS: f64 = 140.0;
const Z0: f64 = 50.0;

const R_CIRCLES: [f64; 4] = [0.5, 1.0, 2.0, 5.0];
const X_ARCS: [f64; 8] = [-5.0, -2.0, -1.0, -0.5, 0.5, 1.0, 2.0, 5.0];

/// 把 Γ 采样点映射为 SVG 路径。
fn to_path(pts: &[(f64, f64)], cx: f64, cy: f64, r: f64) -> String {
  let mut d = String::new();
  for (i, &(gr, gi)) in pts.iter().enumerate() {
    let x = cx + gr * r;
    let y = cy - gi * r;
    if i == 0 {
      d.push_str(&format!("M {x:.1} {y:.1}"));
    } else {
      d.push_str(&format!(" L {x:.1} {y:.1}"));
    }
  }
  d
}

#[component]
pub(super) fn SmithChart() -> impl IntoView {
  let r_ohm = RwSignal::new(50.0);
  let x_ohm = RwSignal::new(0.0);

  let r_id = unique_id("smith-r");
  let x_id = unique_id("smith-x");

  let cx = SIZE / 2.0;
  let cy = SIZE / 2.0;

  let point = move || {
    let (gr, gi) = gamma(r_ohm.get() / Z0, x_ohm.get() / Z0);
    (gr, gi)
  };

  view! {
    <div class="grid gap-4 sm:grid-cols-2">
      <div class="flex flex-col gap-3">
        <Field label=Signal::derive(move || t("tools.resistance-r-2")) r#for=r_id.clone()>
          <NumberField
            id=r_id
            value=Signal::derive(move || r_ohm.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                r_ohm.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.reactance-x-inductive-is")) r#for=x_id.clone()>
          <NumberField
            id=x_id
            value=Signal::derive(move || x_ohm.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                x_ohm.set(v);
              }
            })
            controls=false
          />
        </Field>
        <div class=RESULT>
          {move || {
            let (gr, gi) = point();
            let (swr, rl) = swr_and_return_loss(gr, gi);
            tf(
              "tools.j-swr-return-loss",
              &[&fmt_num(gr), &fmt_num(gi), &fmt_num(swr), &fmt_num(rl)],
            )
          }}
        </div>
        <p class="text-xs text-muted-foreground">
          {move || t("tools.reference-impedance-is-50")}
        </p>
      </div>

      <svg
        viewBox=format!("0 0 {SIZE} {SIZE}")
        class="mx-auto w-full max-w-[320px]"
        role="img"
        aria-label=move || t("tools.smith-chart")
      >
        <circle
          cx=cx
          cy=cy
          r=RADIUS
          class="fill-none stroke-muted-foreground/40"
          stroke-width="1"
          vector-effect="non-scaling-stroke"
        />
        {R_CIRCLES
          .iter()
          .map(|&r| {
            let pts = resistance_circle(r, 120);
            let d = to_path(&pts, cx, cy, RADIUS);
            view! {
              <path
                d=d
                class="fill-none stroke-muted-foreground/25"
                stroke-width="1"
                vector-effect="non-scaling-stroke"
              />
            }
          })
          .collect_view()}
        {X_ARCS
          .iter()
          .map(|&x| {
            let pts = reactance_arc(x, 120);
            let d = to_path(&pts, cx, cy, RADIUS);
            view! {
              <path
                d=d
                class="fill-none stroke-muted-foreground/25"
                stroke-width="1"
                vector-effect="non-scaling-stroke"
              />
            }
          })
          .collect_view()}
        <line
          x1=cx - RADIUS
          y1=cy
          x2=cx + RADIUS
          y2=cy
          class="stroke-muted-foreground/40"
          stroke-width="1"
          vector-effect="non-scaling-stroke"
        />
        {move || {
          let (gr, gi) = point();
          let px = cx + gr * RADIUS;
          let py = cy - gi * RADIUS;
          view! {
            <circle cx=px cy=py r=4 class="fill-primary stroke-background" stroke-width="1.5" />
          }
        }}
      </svg>
    </div>
  }
}

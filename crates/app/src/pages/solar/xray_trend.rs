use leptos::prelude::*;

use crate::i18n::t;

use super::{XrayPoint, fmt_iso_time};

/// y 轴固定范围：log10 通量 -8（A 级下限）到 -4（X 级上限）。
const MIN_LOG: f64 = -8.0;
const MAX_LOG: f64 = -4.0;
const W: f64 = 600.0;
const H: f64 = 160.0;

/// 耀斑级别边界（label, log10 通量）。
const LEVELS: [(&str, f64); 4] = [("B", -7.0), ("C", -6.0), ("M", -5.0), ("X", -4.0)];

fn y_of(flux: f64) -> f64 {
  let lg = flux.max(1e-8).log10().clamp(MIN_LOG, MAX_LOG);
  H - (lg - MIN_LOG) / (MAX_LOG - MIN_LOG) * H
}

/// X 射线通量近 6 小时曲线（对数纵轴，标注 A/B/C/M/X 耀斑级别边界）。
#[component]
pub(super) fn XrayTrend(
  series: Vec<XrayPoint>,
  flux: Option<f64>,
  flare_class: String,
) -> impl IntoView {
  if series.len() < 2 {
    return view! { <div></div> }.into_any();
  }
  let n = series.len();
  let mut d = String::new();
  for (i, p) in series.iter().enumerate() {
    let x = W * i as f64 / (n - 1) as f64;
    let y = y_of(p.flux);
    if i == 0 {
      d.push_str(&format!("M {x:.1} {y:.1}"));
    } else {
      d.push_str(&format!(" L {x:.1} {y:.1}"));
    }
  }
  let first = &series[0];
  let last = &series[n - 1];
  let last_y = flux.map(y_of);

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="flex items-center justify-between border-b px-4 py-3 text-sm font-semibold">
        {t("X 射线通量")}
        <span class="text-xs font-normal text-muted-foreground">
          {move || match flux {
            Some(f) if !flare_class.is_empty() => format!("{} 级 · {:.1e} W/m²", flare_class, f),
            Some(f) => format!("{:.1e} W/m²", f),
            None => t("暂不可用").to_string(),
          }}
        </span>
      </h2>
      <div class="p-4">
        <svg
          viewBox=format!("0 0 {W:.0} {H:.0}")
          class="w-full"
          role="img"
          aria-label=t("X 射线通量曲线")
        >
          {LEVELS
            .iter()
            .map(|(label, lg)| {
              let y = H - (lg - MIN_LOG) / (MAX_LOG - MIN_LOG) * H;
              view! {
                <line
                  x1=0.0
                  y1=y
                  x2=W
                  y2=y
                  class="stroke-muted-foreground/20"
                  stroke-width="1"
                  stroke-dasharray="4 4"
                />
                <text x=3.0 y=y - 3.0 font-size="8" class="fill-muted-foreground">{*label}</text>
              }
            })
            .collect_view()}
          <path
            d=d
            class="fill-none stroke-amber-500"
            stroke-width="1.5"
            vector-effect="non-scaling-stroke"
          />
          {last_y.map(|y| {
            view! { <circle cx=W cy=y r="3" class="fill-amber-500" /> }.into_any()
          })}
        </svg>
        <div class="mt-1 flex justify-between text-[10px] text-muted-foreground">
          <span>{fmt_iso_time(&first.time)}</span>
          <span>{format!("{} UTC", fmt_iso_time(&last.time))}</span>
        </div>
      </div>
    </section>
  }
  .into_any()
}

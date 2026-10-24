use ham_web_core::fresnel::MIN_CLEARANCE_RATIO;
use ham_web_core::path_profile::{ProfileReport, zone_envelope};
use leptos::prelude::*;

use crate::i18n::t;

const W: f64 = 640.0;
const H: f64 = 240.0;
const PAD_L: f64 = 52.0;
const PAD_R: f64 = 12.0;
const PAD_T: f64 = 12.0;
const PAD_B: f64 = 24.0;
/// 菲涅尔区包络的采样步数：椭圆带够圆润，又不会生成过长的路径串。
const ENVELOPE_STEPS: usize = 60;

/// 路径剖面图：地形（已叠加地球曲率）、视线、第一菲涅尔区与 60% 净空线，红点标出最危险的一点。
#[component]
pub(super) fn PathProfileChart(report: ProfileReport) -> impl IntoView {
  let envelope = zone_envelope(&report, ENVELOPE_STEPS);

  // 纵轴范围要把地形、菲涅尔区上下沿都装进去，再留 5% 边距。
  let mut lo = f64::INFINITY;
  let mut hi = f64::NEG_INFINITY;
  for row in &report.rows {
    lo = lo.min(row.elev_m);
    hi = hi.max(row.terrain_effective_m);
  }
  for e in &envelope {
    lo = lo.min(e.los_m - e.fresnel1_m);
    hi = hi.max(e.los_m + e.fresnel1_m);
  }
  let pad = ((hi - lo) * 0.05).max(1.0);
  let (lo, hi) = (lo - pad, hi + pad);

  let plot_w = W - PAD_L - PAD_R;
  let plot_h = H - PAD_T - PAD_B;
  let total = report.total_km;
  let x = |d: f64| PAD_L + d / total * plot_w;
  let y = |v: f64| PAD_T + (hi - v) / (hi - lo) * plot_h;
  let bottom = H - PAD_B;

  let line = |pts: &[(f64, f64)]| {
    pts
      .iter()
      .enumerate()
      .map(|(i, &(px, py))| format!("{} {px:.1} {py:.1}", if i == 0 { "M" } else { "L" }))
      .collect::<Vec<_>>()
      .join(" ")
  };

  let terrain: Vec<(f64, f64)> = report
    .rows
    .iter()
    .map(|r| (x(r.dist_km), y(r.terrain_effective_m)))
    .collect();
  let terrain_fill = format!(
    "{} L {:.1} {bottom:.1} L {:.1} {bottom:.1} Z",
    line(&terrain),
    terrain[terrain.len() - 1].0,
    terrain[0].0,
  );
  let terrain_line = line(&terrain);

  // 上沿正向、下沿反向首尾相接，闭合成一条椭圆带。
  let outline: Vec<(f64, f64)> = envelope
    .iter()
    .map(|e| (x(e.dist_km), y(e.los_m + e.fresnel1_m)))
    .chain(
      envelope
        .iter()
        .rev()
        .map(|e| (x(e.dist_km), y(e.los_m - e.fresnel1_m))),
    )
    .collect();
  let band = format!("{} Z", line(&outline));
  let clear_line = line(
    &envelope
      .iter()
      .map(|e| {
        (
          x(e.dist_km),
          y(e.los_m - MIN_CLEARANCE_RATIO * e.fresnel1_m),
        )
      })
      .collect::<Vec<_>>(),
  );
  let (los_x1, los_y1) = (x(0.0), y(report.tx_abs_m));
  let (los_x2, los_y2) = (x(total), y(report.rx_abs_m));

  let worst = &report.rows[report.worst_index];
  let (worst_x, worst_y) = (x(worst.dist_km), y(worst.terrain_effective_m));

  let top_label = format!("{hi:.0} m");
  let bottom_label = format!("{lo:.0} m");
  let end_label = format!("{total:.1} km");

  view! {
    <svg
      viewBox=format!("0 0 {W} {H}")
      class="w-full"
      role="img"
      aria-label=move || t("tools.profile-chart-label")
    >
      <path d=band class="fill-primary/15 stroke-none" />
      <path d=terrain_fill class="fill-muted-foreground/30 stroke-none" />
      <path
        d=terrain_line
        class="fill-none stroke-muted-foreground"
        stroke-width="1.5"
        vector-effect="non-scaling-stroke"
      />
      <path
        d=clear_line
        class="fill-none stroke-primary/70"
        stroke-width="1"
        stroke-dasharray="4 3"
        vector-effect="non-scaling-stroke"
      />
      <line
        x1=los_x1
        y1=los_y1
        x2=los_x2
        y2=los_y2
        class="stroke-primary"
        stroke-width="1.5"
        vector-effect="non-scaling-stroke"
      />
      <circle cx=worst_x cy=worst_y r=4 class="fill-destructive stroke-background" stroke-width="1.5" />
      <line
        x1=PAD_L
        y1=bottom
        x2=W - PAD_R
        y2=bottom
        class="stroke-muted-foreground/40"
        stroke-width="1"
        vector-effect="non-scaling-stroke"
      />
      <text x=PAD_L - 6.0 y=PAD_T + 8.0 text-anchor="end" class="fill-muted-foreground text-xs">
        {top_label}
      </text>
      <text x=PAD_L - 6.0 y=bottom text-anchor="end" class="fill-muted-foreground text-xs">
        {bottom_label}
      </text>
      <text x=PAD_L y=H - 6.0 class="fill-muted-foreground text-xs">"0 km"</text>
      <text x=W - PAD_R y=H - 6.0 text-anchor="end" class="fill-muted-foreground text-xs">
        {end_label}
      </text>
    </svg>
  }
}

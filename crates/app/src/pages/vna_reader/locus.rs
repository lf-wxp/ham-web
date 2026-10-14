//! 实测 S11 在 Smith 圆图上的轨迹：等电阻圆 / 等电抗弧作底，轨迹折线 + 最低驻波点标记。
//!
//! 网格取样复用核心 `smith::resistance_circle` / `reactance_arc`（与 `/smith` 页同一套），
//! 这里只做 Γ 平面 → SVG 坐标的缩放与 y 轴翻转。

use ham_web_core::smith::{reactance_arc, resistance_circle};
use ham_web_core::touchstone::Sweep;
use leptos::prelude::*;

use crate::i18n::t;

/// 画布边长与圆图半径（SVG 单位）。
const SIZE: f64 = 300.0;
const R: f64 = 140.0;

fn px(gr: f64) -> f64 {
  SIZE / 2.0 + gr * R
}

fn py(gi: f64) -> f64 {
  SIZE / 2.0 - gi * R
}

fn path(pts: &[(f64, f64)]) -> String {
  let mut d = String::new();
  for (i, (gr, gi)) in pts
    .iter()
    .filter(|p| p.0.is_finite() && p.1.is_finite())
    .enumerate()
  {
    d.push_str(&format!(
      "{}{:.2},{:.2} ",
      if i == 0 { "M" } else { "L" },
      px(*gr),
      py(*gi)
    ));
  }
  d
}

#[component]
pub(super) fn S11Locus(
  sweep: Memo<Option<Sweep>>,
  /// 最低驻波点的下标（标记用）。
  best: Signal<Option<usize>>,
) -> impl IntoView {
  let grid: Vec<String> = [0.2, 0.5, 1.0, 2.0, 5.0]
    .iter()
    .map(|&r| path(&resistance_circle(r, 96)))
    .chain(
      [0.2, 0.5, 1.0, 2.0, 5.0, -0.2, -0.5, -1.0, -2.0, -5.0]
        .iter()
        .map(|&x| path(&reactance_arc(x, 96))),
    )
    .collect();
  view! {
    <svg
      viewBox=format!("0 0 {SIZE} {SIZE}")
      class="mx-auto block w-full sm:w-80"
      role="img"
      aria-label=move || t("tools.vna-locus")
      data-slot="vna-locus"
    >
      <circle cx=SIZE / 2.0 cy=SIZE / 2.0 r=R fill="none" class="stroke-muted-foreground" stroke-width="1" />
      <line x1=SIZE / 2.0 - R y1=SIZE / 2.0 x2=SIZE / 2.0 + R y2=SIZE / 2.0 class="stroke-muted-foreground/50" stroke-width="0.5" />
      {grid
        .into_iter()
        .map(|d| view! { <path d=d fill="none" class="stroke-muted-foreground/40" stroke-width="0.5" /> })
        .collect_view()}
      {move || {
        sweep
          .get()
          .map(|s| {
            let pts: Vec<(f64, f64)> = s.points.iter().map(|p| (p.re, p.im)).collect();
            let mark = best.get().and_then(|i| s.points.get(i).copied());
            view! {
              <path d=path(&pts) fill="none" class="stroke-primary" stroke-width="1.5" data-points=pts.len() />
              {mark
                .map(|p| {
                  view! { <circle cx=px(p.re) cy=py(p.im) r="4" class="fill-primary" /> }
                })}
            }
          })
      }}
    </svg>
  }
}

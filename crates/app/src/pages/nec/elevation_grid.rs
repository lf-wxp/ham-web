//! 仰角面网格：横轴刻度 + 纵轴 dB 刻度，以及标出 0° 的琥珀虚线。

use leptos::prelude::*;

use super::{CART_H, CART_PAD, CART_W, DB_FLOOR, el_to_x};

/// 仰角面网格：横轴刻度 + 纵轴 dB 刻度，以及标出 0° 的琥珀虚线。
///
/// 横轴刻度与 [`super::elevation_path`] 共用 [`super::el_to_x`]，天然不会错开。
pub(super) fn elevation_grid(peak: f64, grounded: bool) -> impl IntoView {
  let h = CART_H - 2.0 * CART_PAD;
  // 有地面时下半空间没有辐射，只标 0…90°；自由空间两侧都标。
  let ticks: &[f64] = if grounded {
    &[0.0, 30.0, 60.0, 90.0]
  } else {
    &[-90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0]
  };
  view! {
    {(0..=4)
      .map(|i| {
        let y = CART_PAD + h * i as f64 / 4.0;
        let db = peak - (-DB_FLOOR) * i as f64 / 4.0;
        view! {
          <line
            x1=CART_PAD
            y1=y
            x2=CART_W - CART_PAD
            y2=y
            class="stroke-muted-foreground/20"
            stroke-width="1"
          />
          <text x=2.0 y=y + 3.0 class="fill-muted-foreground text-[10px]">
            {format!("{db:.0}")}
          </text>
        }
      })
      .collect_view()}
    {ticks
      .iter()
      .map(|&el| {
        let x = el_to_x(el);
        view! {
          <line
            x1=x
            y1=CART_PAD
            x2=x
            y2=CART_H - CART_PAD
            class="stroke-muted-foreground/15"
            stroke-width="1"
          />
          <text
            x=x
            y=CART_H - 6.0
            text-anchor="middle"
            class="fill-muted-foreground text-[10px]"
          >
            {format!("{el:.0}°")}
          </text>
        }
      })
      .collect_view()}
    <line
      x1=el_to_x(0.0)
      y1=CART_PAD
      x2=el_to_x(0.0)
      y2=CART_H - CART_PAD
      class="stroke-amber-500/50"
      stroke-dasharray="3 3"
      stroke-width="1"
    />
  }
}

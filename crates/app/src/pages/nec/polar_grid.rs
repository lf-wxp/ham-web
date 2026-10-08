//! 方位面极坐标网格：三个同心圆 + 十字轴 + 方位标注。
//!
//! 网格的坐标约定与曲线一致（`0°` 指向正右 `+X`、`90°` 指向正上 `+Y`，见
//! [`super::polar_xy`]），否则读数会与曲线错开。

use leptos::prelude::*;

use super::POLAR;

/// 极坐标网格：三个同心圆 + 十字轴 + 方位标注。
pub(super) fn polar_grid() -> impl IntoView {
  let c = POLAR / 2.0;
  let r_max = c - 18.0;
  view! {
    {(0..3)
      .map(|i| {
        let r = r_max * (i as f64 + 1.0) / 3.0;
        view! {
          <circle
            cx=c
            cy=c
            r=r
            class="fill-none stroke-muted-foreground/25"
            stroke-width="1"
          />
        }
      })
      .collect_view()}
    <line
      x1=c
      y1=c - r_max
      x2=c
      y2=c + r_max
      class="stroke-muted-foreground/25"
      stroke-width="1"
    />
    <line
      x1=c - r_max
      y1=c
      x2=c + r_max
      y2=c
      class="stroke-muted-foreground/25"
      stroke-width="1"
    />
    <text x=c y=12.0 text-anchor="middle" class="fill-muted-foreground text-[10px]">
      "+Y"
    </text>
    <text x=c y=POLAR - 4.0 text-anchor="middle" class="fill-muted-foreground text-[10px]">
      "−Y"
    </text>
    <text
      x=POLAR - 4.0
      y=c - 4.0
      text-anchor="end"
      class="fill-muted-foreground text-[10px]"
    >
      "+X"
    </text>
    <text x=4.0 y=c - 4.0 class="fill-muted-foreground text-[10px]">
      "−X"
    </text>
  }
}

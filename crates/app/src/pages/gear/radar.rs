//! 六轴雷达图：把各机型的归一化值画成多边形，一眼看出强项与短板。
//!
//! 几何（顶点坐标、轴名排版、视口大小）都在 `ham_web_core::gear_score`，这里只做缩放、
//! 翻转与配色：核心给的是**数学坐标**（y 向上、半径 1），SVG 的 y 朝下，所以取负号即可。
//! 视口也问核心要 —— 轴名长度随语言变化（西语 `DR de separación estrecha` 比中文还长），
//! 写死一个宽度必然在某种语言里把轴名裁掉。

use ham_web_core::gear_score::{
  Axis, LABEL_FONT, LABEL_RADIUS, LabelAnchor, radar_points, radar_viewbox,
};
use leptos::prelude::*;

use crate::i18n::t;

use super::shared::axis_label;

/// 最多同时对比 4 台，配色与 [`GearRadar`] 的取值一一对应（画多边形用）。
pub(super) const SERIES_CLASSES: [&str; 4] = [
  "fill-sky-500/10 stroke-sky-500",
  "fill-emerald-500/10 stroke-emerald-500",
  "fill-amber-500/10 stroke-amber-500",
  "fill-violet-500/10 stroke-violet-500",
];

/// 图例色块的颜色。
///
/// 刻意**不复用** [`SERIES_CLASSES`]：`fill` / `stroke` 只对 SVG 图形生效，写在 HTML
/// `<span>` 上等于没写（色块会是透明的）。图例要的是背景色，两处颜色必须一起改。
pub(super) const SERIES_SWATCHES: [&str; 4] = [
  "bg-sky-500",
  "bg-emerald-500",
  "bg-amber-500",
  "bg-violet-500",
];

/// 图上的一条序列（一台机型；原表一条实线，用户实测一条同色虚线）。
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RadarSeries {
  /// 图例文字。
  pub label: String,
  /// 配色下标（对 [`SERIES_CLASSES`] 取模）。
  pub color: usize,
  /// 各轴的归一化值（顺序与传入的轴一致；`0` 表示该轴无数据）。
  pub values: Vec<f64>,
  /// 虚线 = 用户自己测的值（同色实线是原表/库里的值）。
  pub dashed: bool,
}

/// 归一化值 → SVG 多边形的 `points`（y 翻转；数值被核心夹在 0–1）。
fn polygon_points(values: &[f64]) -> String {
  radar_points(values)
    .into_iter()
    .map(|(x, y)| format!("{x:.4},{:.4}", -y))
    .collect::<Vec<_>>()
    .join(" ")
}

/// 网格环（0.25 / 0.5 / 0.75 / 1.0）。
const RINGS: [f64; 4] = [0.25, 0.5, 0.75, 1.0];

/// 一条轴的名字 + 它在图上的落点与对齐方式。
struct AxisLabel {
  text: String,
  anchor: &'static str,
  x: f64,
  y: f64,
}

/// 按轴序算好每个标签的文字与位置（视口与渲染共用同一份，免得两处各算一次坐标）。
fn axis_labels(axes: &[Axis]) -> Vec<AxisLabel> {
  let n = axes.len().max(1);
  axes
    .iter()
    .map(|&a| axis_label(a))
    .zip(radar_points(&vec![LABEL_RADIUS; n]))
    .map(|(text, (x, y))| AxisLabel {
      text,
      anchor: LabelAnchor::of(x).css(),
      x,
      y,
    })
    .collect()
}

#[component]
pub(super) fn GearRadar(axes: Vec<Axis>, series: Vec<RadarSeries>) -> impl IntoView {
  let n = axes.len().max(1);
  let labels = axis_labels(&axes);
  let ring_points: Vec<String> = RINGS.iter().map(|r| polygon_points(&vec![*r; n])).collect();
  let spokes: Vec<(f64, f64)> = radar_points(&vec![1.0; n]);
  // 视口按轴名实宽撑开：不同语言的文字长度差一倍以上。
  let (vx, vy, vw, vh) = radar_viewbox(&labels.iter().map(|l| l.text.clone()).collect::<Vec<_>>());
  let view_box = format!("{vx:.3} {vy:.3} {vw:.3} {vh:.3}");
  view! {
    <div class="flex flex-col items-center gap-3 sm:flex-row sm:items-start">
      <svg viewBox=view_box class="w-full max-w-md" role="img">
        <title>{move || t("knowledge.radar-title")}</title>
        // 网格与轴
        {ring_points
          .into_iter()
          .map(|points| {
            view! {
              <polygon
                points=points
                class="fill-none stroke-muted-foreground/25"
                stroke-width="0.01"
              />
            }
          })
          .collect_view()}
        {spokes
          .into_iter()
          .map(|(x, y)| {
            view! {
              <line
                x1="0"
                y1="0"
                x2=x
                y2=-y
                class="stroke-muted-foreground/25"
                stroke-width="0.01"
              />
            }
          })
          .collect_view()}
        // 各机型
        {series
          .clone()
          .into_iter()
          .map(|s| {
            let class = SERIES_CLASSES[s.color % SERIES_CLASSES.len()];
            if s.dashed {
              // 虚线用屏幕像素而不是用户单位：viewBox 只有几个单位宽，用户单位下的
              // `4 3` 会变成几乎实线。`vector-effect` 同时把线宽也固定成 2px。
              view! {
                <polygon
                  points=polygon_points(&s.values)
                  class=class
                  stroke-width="2"
                  stroke-dasharray="4 3"
                  vector-effect="non-scaling-stroke"
                  stroke-linejoin="round"
                  fill="none"
                />
              }
                .into_any()
            } else {
              view! {
                <polygon
                  points=polygon_points(&s.values)
                  class=class
                  stroke-width="0.02"
                  stroke-linejoin="round"
                />
              }
                .into_any()
            }
          })
          .collect_view()}
        // 轴名：字号走内联样式而不是 Tailwind 的任意值类 —— 类名是拼出来的，扫描器认不出。
        {labels
          .into_iter()
          .map(|l| {
            view! {
              <text
                x=l.x
                y=-l.y
                text-anchor=l.anchor
                dominant-baseline="middle"
                class="fill-muted-foreground"
                style=format!("font-size: {LABEL_FONT}px")
              >
                {l.text}
              </text>
            }
          })
          .collect_view()}
      </svg>
      <ul class="w-full space-y-1.5 sm:w-56">
        {series
          .into_iter()
          .map(|s| {
            let swatch = SERIES_SWATCHES[s.color % SERIES_SWATCHES.len()];
            view! {
              <li class="flex items-center gap-2 text-xs">
                <span class=format!("h-2.5 w-2.5 shrink-0 rounded-sm {swatch}")></span>
                <span class="truncate">{s.label}</span>
              </li>
            }
          })
          .collect_view()}
      </ul>
    </div>
  }
}

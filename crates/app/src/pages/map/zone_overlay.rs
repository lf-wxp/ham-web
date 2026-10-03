//! CQ / ITU 分区着色叠加层：按分区号为每块 DXCC 区域着色，并给无几何的实体画中心点。
//!
//! 分区没有公开的精确边界数据，且常横跨国界，因此这里以「每个 DXCC 实体的主分区」
//! 着色：整块区域取该实体的 `cq` / `itu`，岛屿等无国界几何的实体以中心点标记。
//! 结果可回答「某分区大致在哪、含哪些实体」，但不是精确的分区边界。

use std::collections::{BTreeMap, BTreeSet};

use ham_web_core::dxcc;
use ham_web_core::dxcc_map::DxccShapes;
use ham_web_core::zone::zone_color;
use leptos::prelude::*;

use super::projection::{polygon_points, project};

/// 分区着色叠加层。
#[component]
pub fn ZoneOverlay(
  #[prop(into)] shapes: Signal<DxccShapes>,
  /// 区域标识（DXCC 编号字符串）→ 分区号。
  #[prop(into)]
  zone_of: Signal<BTreeMap<String, u8>>,
  /// 分区总数（CQ 40 / ITU 90），决定配色。
  #[prop(into)]
  max_zone: Signal<u8>,
  /// 需要高亮的实体（DXCC 编号）。
  #[prop(into)]
  highlight: Signal<Option<u16>>,
  /// 区域标识 → tooltip 文本。
  #[prop(into)]
  title_of: Callback<String, String>,
  /// 点击区域（区域标识为 DXCC 编号字符串）。
  #[prop(into)]
  on_click: Callback<String>,
) -> impl IntoView {
  view! {
    <g>
      // 有国界几何的实体：整块区域按分区着色。
      {move || {
        let zone_map = zone_of.get();
        let max = max_zone.get();
        let hl = highlight.get();
        shapes
          .get()
          .shapes
          .into_iter()
          .map(|s| {
            let key = s.dxcc.to_string();
            // 未登记分区的区域用透明填充，避免 SVG 退回默认黑色。
            let fill = zone_map
              .get(&key)
              .map_or_else(|| "transparent".to_owned(), |z| zone_color(*z, max));
            let is_hl = hl == Some(s.dxcc);
            let stroke = if is_hl {
              "stroke-amber-500"
            } else {
              "stroke-foreground/30"
            };
            let width = if is_hl { "1.5" } else { "0.5" };
            let title = title_of.run(key.clone());
            s.polys
              .into_iter()
              .map(|ring| {
                let points = polygon_points(&ring);
                let click_key = key.clone();
                let title = title.clone();
                let fill = fill.clone();
                view! {
                  <polygon
                    points=points
                    fill=fill
                    fill-opacity="0.55"
                    class=stroke
                    stroke-width=width
                    vector-effect="non-scaling-stroke"
                    on:click=move |_| on_click.run(click_key.clone())
                  >
                    <title>{title.clone()}</title>
                  </polygon>
                }
              })
              .collect_view()
          })
          .collect_view()
      }}

      // 无国界几何的实体（岛屿 / 细分子实体）：以中心点标记，保证分区可见。
      {move || {
        let has_geom: BTreeSet<u16> = shapes.get().shapes.iter().map(|s| s.dxcc).collect();
        let zone_map = zone_of.get();
        let max = max_zone.get();
        let hl = highlight.get();
        dxcc::entities()
          .iter()
          .filter(|e| !has_geom.contains(&e.dxcc))
          .map(|e| {
            let (x, y) = project(e.lon, e.lat);
            let fill = zone_map
              .get(&e.dxcc.to_string())
              .map_or_else(|| "transparent".to_owned(), |z| zone_color(*z, max));
            let is_hl = hl == Some(e.dxcc);
            let radius = if is_hl { "2.6" } else { "1.6" };
            let width = if is_hl { "0.8" } else { "0.4" };
            let title = title_of.run(e.dxcc.to_string());
            let id = e.dxcc;
            view! {
              <circle
                cx=x.to_string()
                cy=y.to_string()
                r=radius
                fill=fill
                fill-opacity="0.85"
                class="stroke-foreground/50"
                stroke-width=width
                on:click=move |_| on_click.run(id.to_string())
              >
                <title>{title}</title>
              </circle>
            }
          })
          .collect_view()
      }}
    </g>
  }
}

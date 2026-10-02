//! 通用区域着色叠加层（choropleth）：把世界地图按「区域状态」着色。
//!
//! DXCC / WAZ / WAS / IOTA 等「按区域看通联进度」的地图都可复用本组件：
//! 传入「区域几何 + 每块区域的状态」，内部完成 SVG 多边形渲染与三色着色。
//! 具体场景（如 DXCC）通过薄包装（[`super::DxccOverlay`]）做数据转换与额外标记。

use std::collections::BTreeMap;

use leptos::prelude::*;

use super::projection::polygon_points;

/// 一块区域的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionStatus {
  /// 未达成（未通联 / 未激活）。
  Empty,
  /// 已达成（已通联 / 已激活）。
  Worked,
  /// 已确认（更高等级）。
  Confirmed,
}

/// 一块区域的几何（key + 一个或多个多边形环）。
#[derive(Debug, Clone, PartialEq)]
pub struct RegionShape {
  /// 区域标识（如 DXCC 编号、CQ 分区号、美国州缩写）。
  pub key: String,
  /// 多边形环（`(经度, 纬度)`，东经为正、北纬为正）。
  pub polys: Vec<Vec<(f64, f64)>>,
}

/// 按状态返回填充/描边类名（完整字面量，供 Tailwind 扫描）。
fn fill_class(status: RegionStatus) -> &'static str {
  match status {
    RegionStatus::Confirmed => "fill-emerald-600/70 stroke-emerald-500/80",
    RegionStatus::Worked => "fill-primary/40 stroke-primary/50",
    RegionStatus::Empty => "fill-foreground/5 stroke-foreground/20",
  }
}

/// 通用区域着色叠加层。
#[component]
pub fn ChoroplethOverlay(
  #[prop(into)] shapes: Signal<Vec<RegionShape>>,
  #[prop(into)] status: Signal<BTreeMap<String, RegionStatus>>,
  /// 区域标识 → tooltip 文本（悬停显示）。
  #[prop(into)]
  title_of: Callback<String, String>,
  /// 点击某区域（区域标识）。
  #[prop(into)]
  on_click: Callback<String>,
) -> impl IntoView {
  view! {
    <g>
      {move || {
        let status = status.get();
        shapes
          .get()
          .into_iter()
          .map(|s| {
            let st = status.get(&s.key).copied().unwrap_or(RegionStatus::Empty);
            let cls = fill_class(st);
            let key = s.key;
            let title = title_of.run(key.clone());
            s.polys
              .into_iter()
              .map(|ring| {
                let points = polygon_points(&ring);
                let click_key = key.clone();
                view! {
                  <polygon
                    points=points
                    class=cls
                    stroke-width="0.5"
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
    </g>
  }
}

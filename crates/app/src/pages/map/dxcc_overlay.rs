//! DXCC 实体着色叠加层：把世界地图按「未通联 / 已通联 / 已确认」三色 choropleth。
//!
//! 通用渲染逻辑在 [`super::ChoroplethOverlay`]，这里只做 DXCC 专属的数据转换
//! （DXCC 编号 → 区域状态）与「无国界实体的中心点标记」。

use std::collections::{BTreeMap, BTreeSet};

use ham_web_core::dxcc_map::DxccShapes;
use leptos::prelude::*;

use super::projection::project;
use super::{ChoroplethOverlay, RegionShape, RegionStatus};
use crate::i18n::tf;

/// DXCC 实体 choropleth 叠加层。
#[component]
pub fn DxccOverlay(
  #[prop(into)] shapes: Signal<DxccShapes>,
  #[prop(into)] worked: Signal<BTreeSet<u16>>,
  #[prop(into)] confirmed: Signal<BTreeSet<u16>>,
  on_entity_click: Callback<u16>,
) -> impl IntoView {
  // DXCC 几何 → 通用区域几何。
  let region_shapes = Signal::derive(move || {
    shapes
      .get()
      .shapes
      .iter()
      .map(|s| RegionShape {
        key: s.dxcc.to_string(),
        polys: s.polys.clone(),
      })
      .collect::<Vec<_>>()
  });
  // 已通联 / 已确认 → 区域状态映射。
  let status = Signal::derive(move || {
    let worked = worked.get();
    let confirmed = confirmed.get();
    let mut map = BTreeMap::new();
    for s in &shapes.get().shapes {
      let st = if confirmed.contains(&s.dxcc) {
        RegionStatus::Confirmed
      } else if worked.contains(&s.dxcc) {
        RegionStatus::Worked
      } else {
        RegionStatus::Empty
      };
      map.insert(s.dxcc.to_string(), st);
    }
    map
  });
  // tooltip：DXCC 编号 → 「实体名 · 状态」。
  let title_of = Callback::new(move |key: String| {
    let dxcc: u16 = key.parse().unwrap_or(0);
    let name =
      ham_web_core::dxcc::entity_by_dxcc(dxcc).map_or_else(|| key.clone(), |e| e.name.to_owned());
    if confirmed.get_untracked().contains(&dxcc) {
      tf("radio.confirmed", &[&name])
    } else if worked.get_untracked().contains(&dxcc) {
      tf("radio.worked", &[&name])
    } else {
      tf("radio.not-worked", &[&name])
    }
  });
  // 点击：区域标识 → DXCC 编号。
  let on_click = Callback::new(move |key: String| {
    if let Ok(dxcc) = key.parse::<u16>() {
      on_entity_click.run(dxcc);
    }
  });

  view! {
    <ChoroplethOverlay shapes=region_shapes status=status title_of=title_of on_click=on_click />
    // 无国界几何的实体（岛屿 / 细分子实体）：以中心点标记，保证全部实体可见。
    {move || {
      let worked = worked.get();
      let confirmed = confirmed.get();
      let has_geom: BTreeSet<u16> = shapes.get().shapes.iter().map(|s| s.dxcc).collect();
      ham_web_core::dxcc::entities()
        .iter()
        .filter(|e| !has_geom.contains(&e.dxcc))
        .map(|e| {
          let (x, y) = project(e.lon, e.lat);
          let is_worked = worked.contains(&e.dxcc);
          let is_confirmed = confirmed.contains(&e.dxcc);
          let cls = if is_confirmed {
            "fill-emerald-600 stroke-emerald-500/80"
          } else if is_worked {
            "fill-primary stroke-primary/60"
          } else {
            "fill-foreground/40 stroke-foreground/30"
          };
          let title = if is_confirmed {
            tf("radio.confirmed", &[e.name])
          } else if is_worked {
            tf("radio.worked", &[e.name])
          } else {
            tf("radio.not-worked", &[e.name])
          };
          let dxcc = e.dxcc;
          view! {
            <circle
              cx=x.to_string()
              cy=y.to_string()
              r="1.5"
              class=cls
              on:click=move |_| on_entity_click.run(dxcc)
            >
              <title>{title.clone()}</title>
            </circle>
          }
        })
        .collect_view()
    }}
  }
}

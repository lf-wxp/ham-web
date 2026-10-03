//! 分区地图组件：加载 DXCC 边界并按 CQ / ITU 分区着色。
//!
//! 独立页 `/zone-map` 与「呼号查询」页共用本组件，避免各自重复写一遍
//! 「加载 `dxcc-entities.bin` + 组装 ZoneOverlay」的样板。

use std::collections::BTreeMap;

use ham_web_core::dxcc;
use ham_web_core::dxcc_map::{DxccShapes, decode_binary};
use ham_web_core::zone::{CQ_ZONE_MAX, ITU_ZONE_MAX};
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::{MapView, ZoneOverlay};
use crate::data;
use crate::i18n::{t, tf};

/// 分区地图：自带边界数据加载，按当前分区制式着色并高亮指定实体。
#[component]
pub fn ZoneMap(
  /// `true` 为 CQ 分区（1–40），`false` 为 ITU 分区（1–90）。
  #[prop(into)]
  is_cq: Signal<bool>,
  /// 需要高亮的实体（DXCC 编号）。
  #[prop(into)]
  highlight: Signal<Option<u16>>,
  /// 可选：外部定位 `(纬度, 经度)`。
  #[prop(optional, into)]
  focus: Signal<Option<(f64, f64)>>,
  /// 可选：点击某个实体（参数为 DXCC 编号字符串）。
  #[prop(optional)]
  on_region_click: Option<Callback<String>>,
) -> impl IntoView {
  let shapes = RwSignal::new(DxccShapes::default());
  let loading = RwSignal::new(true);
  let failed = RwSignal::new(false);

  spawn_local(async move {
    match data::fetch_bytes("/dxcc-entities.bin").await {
      Ok(bytes) => match decode_binary(&bytes) {
        Some(s) => shapes.set(s),
        None => failed.set(true),
      },
      Err(_) => failed.set(true),
    }
    loading.set(false);
  });

  let max_zone = Signal::derive(move || {
    if is_cq.get() {
      CQ_ZONE_MAX
    } else {
      ITU_ZONE_MAX
    }
  });
  // 全部实体（含无几何者）→ 分区号，供区域着色与中心点标记共用。
  let zone_of = Signal::derive(move || {
    let cq = is_cq.get();
    dxcc::entities()
      .iter()
      .map(|e| (e.dxcc.to_string(), if cq { e.cq } else { e.itu }))
      .collect::<BTreeMap<String, u8>>()
  });
  let title_of = Callback::new(move |key: String| {
    let Ok(id) = key.parse::<u16>() else {
      return key;
    };
    let Some(e) = dxcc::entity_by_dxcc(id) else {
      return key;
    };
    let cq = is_cq.get_untracked();
    let zone = if cq { e.cq } else { e.itu };
    let label = if cq { t("CQ") } else { t("ITU") };
    tf("{} · {} {} 区", &[e.name, &label, &zone.to_string()])
  });
  let on_click = on_region_click.unwrap_or_else(|| Callback::new(|_: String| {}));

  view! {
    {move || {
      if loading.get() {
        return view! {
          <div class="py-12 text-center text-sm text-muted-foreground">
            {move || t("加载地图数据…")}
          </div>
        }
        .into_any();
      }
      if failed.get() {
        return view! {
          <div class="py-12 text-center text-sm text-muted-foreground">
            {move || t("边界数据加载失败，请运行 `cargo make dxcc-map` 生成 public/dxcc-entities.bin。")}
          </div>
        }
        .into_any();
      }
      view! {
        <MapView
          aria_label=t("CQ / ITU 分区地图（滚轮缩放、拖拽平移、双击复位）")
          focus=focus
        >
          <ZoneOverlay
            shapes=shapes
            zone_of=zone_of
            max_zone=max_zone
            highlight=highlight
            title_of=title_of
            on_click=on_click
          />
        </MapView>
      }
      .into_any()
    }}
  }
}

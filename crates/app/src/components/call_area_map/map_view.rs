//! 在给定选区下画整张分区号地图。

use ham_web_core::province_map::{CN_MAP_HEIGHT, CN_MAP_WIDTH, ProvinceShapes};
use ham_web_core::reference::province_by_code;
use leptos::prelude::*;

use super::province_label::province_label;
use super::province_polygons::province_polygons;
use super::shared::{Drawn, Layer};
use crate::i18n::t;

/// 两遍绘制，每遍都是「先多边形、后标签」。
///
/// 第一遍画没被选中的省（`All` / `Dimmed`），第二遍画选中的省。不能「所有多边形、再所有标签」：
/// 那样淡出省的标签仍压在选中省的多边形之上 —— 北京的标注点落在河北上，选 3 区时淡灰色的
/// 「北京 1」会浮在被点亮的河北上看不清、还像是河北的字。分两遍之后，选中省的多边形
/// （0.8 不透明）整块盖住下层淡出省的标签；没有选区时第二遍为空，画面与单遍完全一致。
///
/// 选中省的描边也因此不会被后画的邻省压住。
pub(super) fn map_view(
  shapes: &ProvinceShapes,
  sel: Option<&str>,
  on_select: Callback<String>,
) -> AnyView {
  // 先把「产物里有、省份表里也有」的省挑出来并标好层；表里没有的省既不画也不标。
  let drawn: Vec<Drawn<'_>> = shapes
    .shapes
    .iter()
    .filter_map(|shape| {
      let province = province_by_code(&shape.code)?;
      Some(Drawn {
        shape,
        province,
        layer: Layer::of(sel, province.area),
      })
    })
    .collect();

  // 同遍内保持产物里的（地名码）顺序。
  let pass = |n: u8| {
    let members: Vec<&Drawn<'_>> = drawn.iter().filter(|d| d.layer.pass() == n).collect();
    let polygons = members
      .iter()
      .map(|d| province_polygons(d, on_select))
      .collect_view();
    let labels = members.iter().map(|d| province_label(d)).collect_view();
    view! { {polygons} {labels} }
  };
  let (below, above) = (pass(0), pass(1));

  // 手机上 320px 宽会把 6.8px 的字缩到 4px，省名等于看不清：给图一个下限宽度，
  // 窄屏就在自己的容器里横向滚（与 min-w 表格同一套做法）。
  view! {
    <div class="overflow-x-auto">
      <svg
        viewBox=format!("0 0 {CN_MAP_WIDTH:.0} {CN_MAP_HEIGHT:.0}")
        class="block w-full min-w-[30rem] select-none"
        role="img"
        aria-label=move || t("exam.call-area-map-aria")
      >
        {below}
        {above}
      </svg>
    </div>
  }
  .into_any()
}

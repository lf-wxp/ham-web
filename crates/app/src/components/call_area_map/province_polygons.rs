//! 一个省的全部多边形环（岛屿拆成多个环）。

use ham_web_core::province_map::ring_points;
use ham_web_core::reference::call_area_color;
use leptos::prelude::*;

use super::shared::Drawn;
use crate::data;
use crate::i18n::tf;

/// 一个省的多边形。
///
/// 悬停提示是省名 + 分区号，省名走知识库译文，切语言后跟着变。点击回传它所属的分区号
/// （选不选由调用方决定，便于再点一次取消）。
pub(super) fn province_polygons(d: &Drawn<'_>, on_select: Callback<String>) -> impl IntoView {
  let area = d.province.area;
  let name = d.province.name;
  let fill = call_area_color(area);
  let (opacity, stroke, state) = (
    d.layer.fill_opacity(),
    d.layer.stroke_class(),
    d.layer.state(),
  );

  d.shape
    .polys
    .iter()
    .map(|ring| {
      let fill = fill.clone();
      view! {
        <polygon
          points=ring_points(ring)
          fill=fill
          fill-opacity=opacity
          class=format!("cursor-pointer {stroke}")
          stroke-width="0.5"
          vector-effect="non-scaling-stroke"
          data-area=area
          data-state=state
          on:click=move |_| on_select.run(area.to_owned())
        >
          // 悬停提示在闭包里现算：省名走知识库译文，切语言后跟着变。
          <title>{move || tf("exam.province-in-call-area", &[&data::kt_tracked(name), area])}</title>
        </polygon>
      }
    })
    .collect_view()
}

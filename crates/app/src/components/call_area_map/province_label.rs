//! 一个省的标签：省名 + 下面一行的分区号，写不下字的直辖市外加一条引线。

use ham_web_core::province_map::{
  LABEL_DIGIT_SIZE, LABEL_LINE_GAP, LABEL_NAME_SIZE, LEADER_START_RADIUS, label_anchor, project_cn,
};
use leptos::prelude::*;

use super::shared::Drawn;

/// 省名用中文短名而不走译文：地图形制里汉字两三个字，宽度稳定，标签才不会随语言
/// 改成「Inner Mongolia」就顶出省界；译文放在悬停提示（多边形的 `<title>`）里。
pub(super) fn province_label(d: &Drawn<'_>) -> impl IntoView {
  let anchor = label_anchor(d.shape);
  let (x, y) = project_cn(anchor.at.0, anchor.at.1);
  let active = d.layer.is_active();
  let (name_class, digit_class) = if active {
    ("fill-foreground", "fill-muted-foreground")
  } else {
    ("fill-muted-foreground/40", "fill-muted-foreground/30")
  };
  let leader_class = if active {
    "stroke-foreground/40"
  } else {
    "stroke-foreground/15"
  };
  // 直辖市写不下字：标签在省外，用一条细虚线指回省界内。线从文字块边缘出发，
  // 否则它会从省名与分区号中间穿过去。
  let leader = anchor.leader_to.map(|(lon, lat)| {
    let (lx, ly) = project_cn(lon, lat);
    let (dx, dy) = (lx - x, ly - y);
    let len = dx.hypot(dy).max(0.001);
    let (sx, sy) = (
      x + dx / len * LEADER_START_RADIUS,
      y + dy / len * LEADER_START_RADIUS,
    );
    view! {
      <line
        x1=sx.to_string()
        y1=sy.to_string()
        x2=lx.to_string()
        y2=ly.to_string()
        class=leader_class
        stroke-width="0.8"
        stroke-dasharray="1.5 1.5"
        vector-effect="non-scaling-stroke"
        pointer-events="none"
      ></line>
    }
  });

  view! {
    {leader}
    <text
      x=x.to_string()
      y=y.to_string()
      text-anchor="middle"
      pointer-events="none"
      class=name_class
      font-size=LABEL_NAME_SIZE.to_string()
      data-label=d.province.code
    >
      {d.province.name}
    </text>
    <text
      x=x.to_string()
      y=(y + LABEL_LINE_GAP).to_string()
      text-anchor="middle"
      pointer-events="none"
      class=digit_class
      font-size=LABEL_DIGIT_SIZE.to_string()
    >
      {d.province.area}
    </text>
  }
}

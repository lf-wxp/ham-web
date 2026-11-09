//! CQ / ITU 分区着色叠加层：按分区号为每块 DXCC 区域着色，并给无几何的实体画中心点。
//!
//! 分区没有公开的精确边界数据，且常横跨国界，因此这里以「每个 DXCC 实体的主分区」
//! 着色：整块区域取该实体的 `cq` / `itu`，岛屿等无国界几何的实体以中心点标记。
//! 结果可回答「某分区大致在哪、含哪些实体」，但不是精确的分区边界。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ham_web_core::dxcc;
use ham_web_core::dxcc_map::DxccShapes;
use ham_web_core::zone::zone_color;
use leptos::prelude::*;

use super::projection::{project, write_polygon_points};
use super::svg_markup::{KEY_ATTR, clicked_key, push_attr, push_escaped};

/// 分区着色叠加层。
#[component]
pub fn ZoneOverlay(
  #[prop(into)] shapes: Signal<Arc<DxccShapes>>,
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
  // 整层拼成一段标记、`inner_html` 一次解析；点击在外层 `<g>` 上委托（见 `svg_markup`）。
  // 每个节点的 DOM 形状与逐个 `view!` 建出来的完全一样：多边形 / 圆点各带自己的 `<title>`。
  let markup = move || {
    let zone_map = zone_of.get();
    let max = max_zone.get();
    let hl = highlight.get();
    let fill_of = |dxcc: u16| {
      // 未登记分区的区域用透明填充，避免 SVG 退回默认黑色。
      zone_map
        .get(&dxcc.to_string())
        .map_or_else(|| "transparent".to_owned(), |z| zone_color(*z, max))
    };
    let mut out = String::new();
    let shapes = shapes.get();

    // 有国界几何的实体：整块区域按分区着色。
    for s in &shapes.shapes {
      let key = s.dxcc.to_string();
      let fill = fill_of(s.dxcc);
      let is_hl = hl == Some(s.dxcc);
      let stroke = if is_hl {
        "stroke-gold/70"
      } else {
        "stroke-foreground/30"
      };
      let width = if is_hl { "1.5" } else { "0.5" };
      let title = title_of.run(key.clone());
      // 同一块区域的各个环只有 `points` 不同：开头 / 样式属性 / 收尾各拼一次，环与环之间复用。
      let mut head = String::from("<polygon");
      push_attr(&mut head, KEY_ATTR, &key);
      let mut style = String::new();
      push_attr(&mut style, "fill", &fill);
      push_attr(&mut style, "fill-opacity", "0.55");
      push_attr(&mut style, "class", stroke);
      push_attr(&mut style, "stroke-width", width);
      push_attr(&mut style, "vector-effect", "non-scaling-stroke");
      let mut tail = String::from("><title>");
      push_escaped(&mut tail, &title);
      tail.push_str("</title></polygon>");
      for ring in &s.polys {
        out.push_str(&head);
        out.push_str(" points=\"");
        write_polygon_points(&mut out, ring);
        out.push('"');
        out.push_str(&style);
        out.push_str(&tail);
      }
    }

    // 无国界几何的实体（岛屿 / 细分子实体）：以中心点标记，保证分区可见。
    let has_geom: BTreeSet<u16> = shapes.shapes.iter().map(|s| s.dxcc).collect();
    for e in dxcc::entities()
      .iter()
      .filter(|e| !has_geom.contains(&e.dxcc))
    {
      let (x, y) = project(e.lon, e.lat);
      let is_hl = hl == Some(e.dxcc);
      let radius = if is_hl { "2.6" } else { "1.6" };
      let width = if is_hl { "0.8" } else { "0.4" };
      let id = e.dxcc.to_string();
      out.push_str("<circle");
      push_attr(&mut out, KEY_ATTR, &id);
      push_attr(&mut out, "cx", &x.to_string());
      push_attr(&mut out, "cy", &y.to_string());
      push_attr(&mut out, "r", radius);
      push_attr(&mut out, "fill", &fill_of(e.dxcc));
      push_attr(&mut out, "fill-opacity", "0.85");
      push_attr(&mut out, "class", "stroke-foreground/50");
      push_attr(&mut out, "stroke-width", width);
      out.push_str("><title>");
      push_escaped(&mut out, &title_of.run(id));
      out.push_str("</title></circle>");
    }
    out
  };

  view! {
    <g
      inner_html=markup
      on:click=move |ev| {
        if let Some(key) = clicked_key(&ev) {
          on_click.run(key);
        }
      }
    ></g>
  }
}

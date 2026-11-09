//! 通用区域着色叠加层（choropleth）：把世界地图按「区域状态」着色。
//!
//! DXCC / WAZ / WAS / IOTA 等「按区域看通联进度」的地图都可复用本组件：
//! 传入「区域几何 + 每块区域的状态」，内部完成 SVG 多边形渲染与三色着色。
//! 具体场景（如 DXCC）通过薄包装（[`super::DxccOverlay`]）做数据转换与额外标记。

use std::collections::BTreeMap;

use leptos::prelude::*;

use super::projection::write_polygon_points;
use super::svg_markup::{KEY_ATTR, clicked_key, push_attr, push_escaped};

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
    RegionStatus::Confirmed => "fill-win/70 stroke-win/80",
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
  // 整层拼成一段标记、`inner_html` 一次解析；点击在外层 `<g>` 上委托（见 `svg_markup`）。
  // 每块多边形的 DOM 形状与逐个 `view!` 建出来的完全一样（`<polygon><title/></polygon>`）。
  let markup = move || {
    let status = status.get();
    let mut out = String::new();
    shapes.with(|shapes| {
      for s in shapes {
        let st = status.get(&s.key).copied().unwrap_or(RegionStatus::Empty);
        let cls = fill_class(st);
        let title = title_of.run(s.key.clone());
        // 同一块区域的各个环只有 `points` 不同：开头 / 样式属性 / 收尾各拼一次，环与环之间复用。
        let mut head = String::from("<polygon");
        push_attr(&mut head, KEY_ATTR, &s.key);
        let mut style = String::new();
        push_attr(&mut style, "class", cls);
        push_attr(&mut style, "stroke-width", "0.5");
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
    });
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

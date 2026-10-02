//! 网格地图（Maidenhead Grid Locator 世界地图）。
//!
//! 底图绘制与缩放/平移/反子午线环绕等交互统一由 [`crate::pages::map::MapView`] 提供；
//! 本模块专注网格业务：通联记录的 field / square 聚合、波段 / 模式筛选、灰线叠加开关、
//! 搜索定位与 field / square 点击展开。投影方式与交互决策见 [`crate::pages::map`] 模块文档。
//!
//! 拆分为：`state`（共享状态与聚合逻辑）、`toolbar`（筛选工具栏）、`stats_bar`（统计徽章）、
//! `overlay`（SVG 叠加层）、`panels`（侧栏面板）、`details`（图例与明细），每文件一个组件。

mod details;
mod overlay;
mod panels;
mod state;
mod stats_bar;
mod toolbar;

use leptos::prelude::*;

use crate::pages::log::LogEntry;
use crate::pages::map::MapView;

use crate::i18n::t;
use details::GridDetails;
use overlay::GridOverlay;
use panels::GridPanels;
use state::GridMapState;
use stats_bar::GridStatsBar;
use toolbar::GridToolbar;

/// Maidenhead 网格地图：square（2°×1°）精细热力 + 大陆轮廓 + 缩放平移 + field 点击展开。
///
/// - `entries`：全部通联记录（含呼号 / 频率 / 模式 / 网格），用于聚合与明细展示。
/// - `station_grid`：本台网格（MY_GRIDSQUARE），非空时在地图上标记“家”位置。
#[component]
pub fn GridMap(entries: Vec<LogEntry>, station_grid: String) -> impl IntoView {
  let state = GridMapState::new(entries, station_grid);
  let entries_empty = state.data.with_value(|d| d.entries_empty);

  view! {
    <div>
      // 空状态：无通联记录时提示
      {entries_empty.then(|| {
        view! {
          <div class="mb-3 rounded-lg border bg-muted/30 p-4 text-center text-sm text-muted-foreground">
            {move || t("暂无通联记录，请先在「通联日志」中添加记录，或从 ADIF / CSV 导入。")}
          </div>
        }
      })}
      <GridToolbar state=state />
      <GridStatsBar state=state />
      <MapView
        aria_label=t("已通联网格地图（滚轮缩放、拖拽平移、双指缩放、双击复位、反子午线环绕）")
        focus=state.focus
        zoom_signal=state.zoom
        hover_signal=state.hover_pos
        click_signal=state.clicked_pos
      >
        <GridOverlay state=state />
      </MapView>
      <GridPanels state=state />
      <GridDetails state=state />
    </div>
  }
}

#[cfg(test)]
mod tests {

  use super::super::grid_fill::{field_fill_class, square_fill_class};
  use super::super::grid_geo::square_label;

  #[test]
  fn square_label_formats_field_and_square() {
    // (sl, sa) 全局索引 → 4 位网格码。
    assert_eq!(square_label((14 * 10 + 8, 12 * 10 + 9)), "OM89");
    assert_eq!(square_label((9 * 10, 9 * 10)), "JJ00");
  }

  #[test]
  fn fill_classes_bucket_by_ratio() {
    assert_eq!(field_fill_class(0, 100), "fill-primary/15");
    assert_eq!(field_fill_class(100, 100), "fill-primary/60");
    assert_eq!(square_fill_class(0, 100), "fill-primary/30");
    assert_eq!(square_fill_class(100, 100), "fill-primary");
  }
}

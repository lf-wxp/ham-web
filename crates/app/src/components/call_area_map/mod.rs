//! 中国分区号地图：按呼号分区号给 31 个省级行政区着色，点省份即可选中它所属的分区。
//!
//! 几何来自 `public/cn-provinces.bin`（`cargo make cn-map` 生成，按需加载），省份名与
//! 分区号来自 `CHINA_PROVINCES`：两边用 ISO 地名码对齐，两边任何一边没有的省直接不画。
//! 每个省标出省名与分区号 —— 「哪些省同属一个分区」看数字与颜色相同即可，不必记名字。
//! 北京 / 天津 / 上海小到写不下两个汉字，标签推到省外并用引线指回（见 `label_anchor`）。
//!
//! 绘制分两遍，每遍都是「先多边形、后标签」：没被选中的省在下、选中的省在上（见
//! [`map_view`]）。选中区的多边形（0.8 不透明）因此盖得住下层淡出的邻省，也盖得住落在它
//! 身上的别省标签；选中省的描边也不会被后画的邻省压住。
//!
//! 本目录按「一个组件一个文件」组织（见 `AGENTS.md`）：共享的层类型在 [`shared`]，
//! 整图、省多边形、省标签各占一个文件，这里只放带加载状态的外壳。

mod map_view;
mod province_label;
mod province_polygons;
mod shared;

use ham_web_core::province_map::decode_binary;
use leptos::prelude::*;

use crate::data::{self, LoadState};
use crate::i18n::t;

use map_view::map_view;

/// 分区号地图：自带边界数据加载，按当前选中分区高亮。
#[component]
pub fn CallAreaMap(
  /// 选中的分区号（`"1"`–`"9"` / `"0"`）；`None` 表示全部同色显示。
  #[prop(into)]
  selected: Signal<Option<String>>,
  /// 点击某个省份：回传它所属的分区号（选不选由调用方决定，便于再点一次取消）。
  #[prop(into)]
  on_select: Callback<String>,
) -> impl IntoView {
  let state = data::load_binary("/cn-provinces.bin", decode_binary);

  view! {
    {move || match state.get() {
      LoadState::Loading => {
        view! {
          <div class="py-10 text-center text-sm text-muted-foreground">
            {move || t("radio.loading-map-data")}
          </div>
        }
        .into_any()
      }
      LoadState::Failed => {
        view! {
          <div class="py-10 text-center text-sm text-muted-foreground">
            {move || t("exam.call-area-map-failed")}
          </div>
        }
        .into_any()
      }
      LoadState::Ready(shapes) => map_view(&shapes, selected.get().as_deref(), on_select),
    }}
  }
}

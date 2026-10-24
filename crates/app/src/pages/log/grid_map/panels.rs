//! 网格地图侧栏面板：猎取进度、通联最多 square、邻近未通联推荐、点击反查。

use ham_web_core::grid::{distance_bearing, field_index};
use leptos::prelude::*;

use super::super::grid_geo::{square_center, square_label};
use super::state::{GridMapState, WORLD_SQUARES};
use crate::i18n::{t, tf};
use crate::ui::{Button, Size, Tooltip, Variant};

#[component]
pub(super) fn GridPanels(state: GridMapState) -> impl IntoView {
  let data = state.data.with_value(|d| d.clone());
  let field_label = field_index(&data.station_grid)
    .map(|(fl, fa)| ((b'A' + fl as u8) as char, (b'A' + fa as u8) as char));
  let world_pct = data.world_done as f64 / WORLD_SQUARES as f64 * 100.0;

  view! {
    // 网格猎取进度（本台 field 内共 100 个 square）
    {field_label.map(|(f_lon, f_lat)| {
      let done = data.hunt_done.len();
      view! {
        <div class="mt-2 border bg-inset p-3">
          <div class="flex items-center justify-between text-xs">
            <span>{tf("log.grid-hunt-field", &[&f_lon.to_string(), &f_lat.to_string()])}</span>
            <span class="tabular-nums text-muted-foreground">{done} " / 100"</span>
          </div>
          <div class="mt-1.5 h-1.5 w-full overflow-hidden bg-muted">
            <div
              class="h-full bg-primary transition"
              style=format!("width: {}%", done)
            ></div>
          </div>
        </div>
      }
    })}

    // 全球 square 猎取进度（全部 32400 个 square）
    <div class="mt-2 border bg-inset p-3">
      <div class="flex items-center justify-between text-xs">
        <span>{move || t("log.global-square-hunt")}</span>
        <span class="tabular-nums text-muted-foreground">{data.world_done} " / " {WORLD_SQUARES}</span>
      </div>
      <div class="mt-1.5 h-1.5 w-full overflow-hidden bg-muted">
        <div
          class="h-full bg-primary transition"
          style=format!("width: {:.3}%", world_pct)
        ></div>
      </div>
    </div>

    // 通联最多 square（Top 榜，全量）
    {(!data.top_squares.is_empty()).then(|| {
      view! {
        <div class="mt-2 border bg-inset p-3 text-xs">
          <div class="mb-1.5">{move || t("log.most-worked-squares")}</div>
          <div class="flex flex-wrap gap-1.5">
            {data.top_squares
              .iter()
              .map(|((sl, sa), n)| {
                let label = square_label((*sl, *sa));
                view! {
                  <span class="bg-inset px-2 py-0.5 font-mono text-xs">{format!("{label} ×{n}")}</span>
                }
              })
              .collect_view()}
          </div>
        </div>
      }
    })}

    // 邻近未通联推荐（本台 square 8 邻域，按距离排序）
    {(!data.neighbor_hits.is_empty()).then(|| {
      view! {
        <div class="mt-2 border bg-inset p-3">
          <div class="mb-1.5 text-xs">{move || t("log.nearby-unworked-click-to")}</div>
          <div class="flex flex-wrap gap-1.5">
            {data.neighbor_hits
              .iter()
              .map(|key| {
                let k = *key;
                let label = square_label(k);
                let (meta, title) = match data.home_pos {
                  Some((hlat, hlon)) => {
                    let (d, b) =
                      distance_bearing(hlat, hlon, square_center(k).0, square_center(k).1);
                    (
                      format!("{label} {b:.0}°"),
                      tf("log.km-away", &[&(label).to_string(), &(format!("{d:.0}")).to_string()]),
                    )
                  }
                  None => (label.clone(), label.clone()),
                };
                view! {
                  <Tooltip label=title>
                    <button
                      type="button"
                      class="bg-inset px-2 py-0.5 font-mono text-xs transition-colors hover:bg-accent"
                      on:click=move |_| state.focus_square(k)
                    >
                      {meta}
                    </button>
                  </Tooltip>
                }
              })
              .collect_view()}
          </div>
        </div>
      }
    })}

    // 点击反查结果
    {move || {
      state.click_inspect.get().map(|(lat, lon, grid, label, worked)| {
        view! {
          <div class="mt-2 border bg-inset p-3 text-xs">
            <div class="mb-1 flex items-center justify-between">
              <span>{move || t("log.click-inspector")}</span>
              <Button
                variant=Variant::Text
                size=Size::Xs
                on_click=Callback::new(move |_| state.clicked_pos.set(None))
              >
                {move || t("log.close")}
              </Button>
            </div>
            <div class="font-mono text-sm">{grid}</div>
            <div class="mt-0.5 text-muted-foreground tabular-nums">
              {format!("{lat:.3}°, {lon:.3}°")}
            </div>
            <div class="mt-0.5">
              {match (label, worked) {
                (Some(l), true) => tf("log.square-worked", &[&(l).to_string()]),
                (Some(l), false) => tf("log.square-not-worked", &[&(l).to_string()]),
                (None, _) => t("log.cannot-resolve-this-point"),
              }}
            </div>
          </div>
        }
      })
    }}
  }
}

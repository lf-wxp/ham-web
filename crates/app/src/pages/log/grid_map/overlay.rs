//! 网格地图叠加层（SVG）：field / square 热力 + 选中轮廓 + 家标记 + 灰线 + 路径 + 交互层。

use ham_web_core::grid::{distance_bearing, grid_from_lat_lon, lat_lon_from_grid, square_index};
use leptos::prelude::*;

use crate::pages::map::{GraylineOverlay, project};

use super::super::grid_fill::{
  field_fill_class, square_fill_class, square_fill_class_confirmed, square_fill_class_sent,
};
use super::super::grid_geo::{great_circle_d, square_center, square_label};
use super::state::{FIELD_SQUARE_ZOOM, GridMapState};
use crate::i18n::tf;

#[component]
pub(super) fn GridOverlay(state: GridMapState) -> impl IntoView {
  let home_pos = state.data.with_value(|d| d.home_pos);
  let station_grid = state.data.with_value(|d| d.station_grid.clone());

  // 本台“家”标记（静态，非空时构建）。
  let mark: Option<(String, String, String)> =
    lat_lon_from_grid(&station_grid).map(|(lat, lon)| {
      let (x, y) = project(lon, lat);
      (
        x.to_string(),
        y.to_string(),
        tf("本台网格 {}", &[&(station_grid).to_string()]),
      )
    });
  let home_marker = view! {
    <g>
      {mark.map(|(cx, cy, label)| {
        view! {
          <g class="stroke-primary" vector-effect="non-scaling-stroke">
            <circle cx=cx.clone() cy=cy.clone() r="6" fill="none" stroke-width="2" />
            <circle cx=cx cy=cy r="2" class="fill-primary" />
            <title>{label}</title>
          </g>
        }
      })}
    </g>
  };

  view! {
    // L3 field 热力（zoom 较小时显示）
    {move || {
      (state.zoom.get() < FIELD_SQUARE_ZOOM).then(|| {
        let fmax = state.field_max.get();
        state.field_grids
          .get()
          .iter()
          .map(|((fl, fa), list)| {
            let x = (fl * 40).to_string();
            let y = ((17 - fa) * 20).to_string();
            view! {
              <rect x=x y=y width="40" height="20" class=field_fill_class(list.len(), fmax) />
            }
          })
          .collect_view()
      })
    }}
    // L4 square 热力（含悬停 tooltip，zoom 较大时显示）
    {move || {
      (state.zoom.get() >= FIELD_SQUARE_ZOOM).then(|| {
        let smax = state.square_max.get();
        state.square_entries
          .get()
          .iter()
          .map(|((sl, sa), list)| {
            let x = (sl * 4).to_string();
            let y = ((179 - sa) * 2).to_string();
            let label = square_label((*sl, *sa));
            let n = list.len();
            let confirmed = list.iter().filter(|e| e.qsl_rcvd).count();
            let sent = list.iter().filter(|e| e.qsl_sent && !e.qsl_rcvd).count();
            let cls = if confirmed == n {
              square_fill_class_confirmed(n, smax)
            } else if confirmed > 0 || sent > 0 {
              square_fill_class_sent(n, smax)
            } else {
              square_fill_class(n, smax)
            };
            let (lat, lon) = square_center((*sl, *sa));
            let title = match home_pos {
              Some((hlat, hlon)) => {
                let (d, b) = distance_bearing(hlat, hlon, lat, lon);
                tf(
                  "{}：{} 条（{} 确认）· {}° / {} km",
                  &[
                    &label,
                    &n.to_string(),
                    &confirmed.to_string(),
                    &format!("{b:.0}"),
                    &format!("{d:.0}"),
                  ],
                )
              }
              None => tf(
                "{}：{} 条（{} 确认）",
                &[&label, &n.to_string(), &confirmed.to_string()],
              ),
            };
            view! {
              <rect x=x y=y width="4" height="2" class=cls>
                <title>{title}</title>
              </rect>
            }
          })
          .collect_view()
      })
    }}
    // field 标签（zoom 较小时显示）
    {move || {
      (state.zoom.get() < FIELD_SQUARE_ZOOM).then(|| {
        view! {
          <g class="fill-muted-foreground" style="opacity:0.75">
            {state.field_grids
              .get()
              .iter()
              .map(|((fl, fa), _)| {
                let cx = (fl * 40 + 20).to_string();
                let cy = ((17 - fa) * 20 + 10).to_string();
                let f_lon = (b'A' + *fl as u8) as char;
                let f_lat = (b'A' + *fa as u8) as char;
                view! {
                  <text x=cx y=cy text-anchor="middle" dominant-baseline="middle" class="fill-muted-foreground" font-size="7">
                    {format!("{f_lon}{f_lat}")}
                  </text>
                }
              })
              .collect_view()}
          </g>
        }
      })
    }}
    // 选中 field 轮廓（zoom 较小时显示）
    {move || {
      (state.zoom.get() < FIELD_SQUARE_ZOOM)
        .then(|| state.selected.get())
        .flatten()
        .map(|(fl, fa)| {
          let x = (fl * 40).to_string();
          let y = ((17 - fa) * 20).to_string();
          view! {
            <rect
              x=x
              y=y
              width="40"
              height="20"
              fill="none"
              class="stroke-ring"
              stroke-width="2.5"
              vector-effect="non-scaling-stroke"
            />
          }
        })
    }}
    // 选中 square 轮廓（zoom 较大时显示）
    {move || {
      (state.zoom.get() >= FIELD_SQUARE_ZOOM)
        .then(|| state.selected_square.get())
        .flatten()
        .map(|(sl, sa)| {
          let x = (sl * 4).to_string();
          let y = ((179 - sa) * 2).to_string();
          view! {
            <rect
              x=x
              y=y
              width="4"
              height="2"
              fill="none"
              class="stroke-ring"
              stroke-width="2"
              vector-effect="non-scaling-stroke"
            />
          }
        })
    }}
    // 悬停 square 描边（悬停联动，zoom 较大时显示）
    {move || {
      (state.zoom.get() >= FIELD_SQUARE_ZOOM)
        .then(|| {
          state.hover_pos.get().and_then(|(lat, lon)| {
            square_index(&grid_from_lat_lon(lat, lon).unwrap_or_default())
          })
        })
        .flatten()
        .map(|(sl, sa)| {
          let x = (sl * 4).to_string();
          let y = ((179 - sa) * 2).to_string();
          view! {
            <rect
              x=x
              y=y
              width="4"
              height="2"
              fill="none"
              class="stroke-foreground"
              stroke-width="1.5"
              opacity="0.8"
              vector-effect="non-scaling-stroke"
            />
          }
        })
    }}
    // 通联路径（全部筛选结果）
    {move || {
      state.qso_paths.with(|(paths, _, _)| {
        paths
          .iter()
          .map(|(d, color, title)| {
            view! {
              <path
                d=format!("M {d}")
                fill="none"
                stroke=*color
                stroke-width="1.2"
                opacity="0.75"
                vector-effect="non-scaling-stroke"
              >
                <title>{title.clone()}</title>
              </path>
            }
          })
          .collect_view()
      })
    }}
    // 本台“家”标记
    {home_marker}
    // 大圆航线：选中或悬停 square 时绘制本台 → square 中心的最短路径。
    {move || {
      let target = state.selected_square.get().or_else(|| {
        state.hover_pos.get().and_then(|(lat, lon)| {
          square_index(&grid_from_lat_lon(lat, lon).unwrap_or_default())
        })
      });
      home_pos.zip(target).map(|(home, (sl, sa))| {
        let d = great_circle_d(home, square_center((sl, sa)), 128);
        view! {
          <path
            d=format!("M {d}")
            fill="none"
            class="stroke-primary"
            stroke-width="1.5"
            stroke-dasharray="3 3"
            opacity="0.7"
            vector-effect="non-scaling-stroke"
          />
        }
      })
    }}
    // 灰线叠加（晨昏圈 + 夜半球遮罩）
    {move || {
      state.show_grayline.get().then(|| {
        view! { <GraylineOverlay now_ms=state.now /> }
      })
    }}
    // field 交互层（点击展开，zoom 较小时显示）
    {move || {
      (state.zoom.get() < FIELD_SQUARE_ZOOM).then(|| {
        state.field_grids
          .get()
          .iter()
          .map(|((fl, fa), list)| {
            let x = (fl * 40).to_string();
            let y = ((17 - fa) * 20).to_string();
            let f_lon = (b'A' + *fl as u8) as char;
            let f_lat = (b'A' + *fa as u8) as char;
            let key = (*fl, *fa);
            let n = list.len();
            view! {
              <rect
                x=x
                y=y
                width="40"
                height="20"
                fill="transparent"
                class="cursor-pointer"
                on:click=move |_| {
                  state.selected.set(Some(key));
                  state.selected_square.set(None);
                }
              >
                <title>{tf("{}：{} 个网格，点击查看", &[&(format!("{f_lon}{f_lat}")).to_string(), &n.to_string()])}</title>
              </rect>
            }
          })
          .collect_view()
      })
    }}
    // square 交互层（点击展开明细，覆盖在 field 交互层之上，zoom 较大时显示）
    {move || {
      (state.zoom.get() >= FIELD_SQUARE_ZOOM).then(|| {
        state.square_entries
          .get()
          .iter()
          .map(|((sl, sa), list)| {
            let x = (sl * 4).to_string();
            let y = ((179 - sa) * 2).to_string();
            let key = (*sl, *sa);
            let label = square_label(key);
            let n = list.len();
            view! {
              <rect
                x=x
                y=y
                width="4"
                height="2"
                fill="transparent"
                class="cursor-pointer"
                on:click=move |_| {
                  state.selected_square.set(Some(key));
                  state.selected.set(None);
                }
              >
                <title>{tf("{}：{} 条，点击查看明细", &[&(label).to_string(), &n.to_string()])}</title>
              </rect>
            }
          })
          .collect_view()
      })
    }}
    // 查询高亮标记
    {move || {
      state.highlight.get().map(|g| {
        let (lat, lon) = lat_lon_from_grid(&g).unwrap_or((0.0, 0.0));
        let (x, y) = project(lon, lat);
        view! {
          <g class="stroke-red-500" vector-effect="non-scaling-stroke">
            <circle cx=x.to_string() cy=y.to_string() r="9" fill="none" stroke-width="2.5" />
            <line x1=(x - 13.0).to_string() y1=y.to_string() x2=(x + 13.0).to_string() y2=y.to_string() stroke-width="2" />
            <line x1=x.to_string() y1=(y - 13.0).to_string() x2=x.to_string() y2=(y + 13.0).to_string() stroke-width="2" />
          </g>
        }
      })
    }}
  }
}

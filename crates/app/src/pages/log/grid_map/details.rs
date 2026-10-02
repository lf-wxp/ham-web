//! 网格地图详情区：图例 / 交互提示、选中 field 列表、选中 square 通联明细。

use ham_web_core::grid::distance_bearing;
use leptos::prelude::*;

use super::super::grid_fill::band_color;
use super::super::grid_geo::{square_center, square_label};
use super::state::GridMapState;
use crate::i18n::{t, tf};

#[component]
pub(super) fn GridDetails(state: GridMapState) -> impl IntoView {
  let home_pos = state.data.with_value(|d| d.home_pos);

  view! {
    // 图例 + 交互提示
    <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
      <span class="flex items-center gap-1.5">
        <span class="text-muted-foreground">{move || t("密度")}</span>
        <span class="inline-block h-2 w-24 rounded-full bg-gradient-to-r from-primary/15 to-primary"></span>
        <span>{move || t("低 → 高")}</span>
      </span>
      <span class="flex items-center gap-1.5">
        <span class="inline-block h-3 w-3 rounded-full border-2 border-primary"></span>
        <span>{move || t("本台")}</span>
      </span>
      {move || {
        state.show_grayline.get().then(|| {
          view! {
            <span class="flex items-center gap-1.5">
              <span class="inline-block h-3 w-3 rounded-sm bg-amber-500/40"></span>
              <span>{move || t("晨昏圈")}</span>
              <span class="ml-1 inline-block h-2.5 w-2.5 rounded-full bg-amber-500"></span>
              <span>{move || t("太阳直射点")}</span>
              <span class="inline-block h-2.5 w-2.5 rounded-full border border-amber-500"></span>
              <span>{move || t("反日点")}</span>
            </span>
          }
        })
      }}
      {move || {
        state.qso_paths.with(|(_, legend, skipped)| {
          (!legend.is_empty()).then(|| {
            let skipped = *skipped;
            view! {
              <span class="flex flex-wrap items-center gap-x-2 gap-y-1">
                {move || t("路径")}
                {legend
                  .iter()
                  .map(|b| {
                    view! {
                      <span class="inline-flex items-center gap-1">
                        <span class="inline-block h-0.5 w-4 rounded" style=format!("background:{}", band_color(b))></span>
                        {if b.is_empty() { t("未知") } else { b.clone() }}
                      </span>
                    }
                  })
                  .collect_view()}
                {(skipped > 0).then(|| tf("（另有 {} 条未绘制）", &[&skipped.to_string()]))}
              </span>
            }
          })
        })
      }}
      <span class="text-muted-foreground">{move || t("滚轮/双指缩放 · 拖拽平移 · 双击复位 · 输入网格定位 · 点击 field/square 展开")}</span>
    </div>

    // 选中 field 的展开列表
    {move || {
      state.selected.get().map(|(fl, fa)| {
        let f_lon = (b'A' + fl as u8) as char;
        let f_lat = (b'A' + fa as u8) as char;
        let mut list = state.field_grids.get().get(&(fl, fa)).cloned().unwrap_or_default();
        list.sort();
        view! {
          <div class="mt-2 rounded-lg border bg-muted/30 p-3">
            <div class="mb-2 flex items-center justify-between text-xs">
              <span class="font-semibold">{tf("网格 {}", &[&(format!("{f_lon}{f_lat}")).to_string()])}</span>
              <button
                type="button"
                class="text-muted-foreground transition-colors hover:text-foreground"
                on:click=move |_| state.selected.set(None)
              >
                {move || t("关闭")}
              </button>
            </div>
            <div class="flex flex-wrap gap-1.5">
              {list
                .into_iter()
                .map(|g| {
                  view! {
                    <span class="rounded bg-muted/60 px-2 py-0.5 font-mono text-xs">{g}</span>
                  }
                })
                .collect_view()}
            </div>
          </div>
        }
      })
    }}

    // 选中 square 的通联明细
    {move || {
      state.selected_square.get().map(|(sl, sa)| {
        let label = square_label((sl, sa));
        let mut list = state.square_entries.get().get(&(sl, sa)).cloned().unwrap_or_default();
        list.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.time.cmp(&a.time)));
        let confirmed = list.iter().filter(|e| e.qsl_rcvd).count();
        let (lat, lon) = square_center((sl, sa));
        let bearing_line = home_pos.map(|(hlat, hlon)| {
          let (d, b) = distance_bearing(hlat, hlon, lat, lon);
          tf(
            "本台方位 {}° · 距离 {} km",
            &[&format!("{b:.0}"), &format!("{d:.0}")],
          )
        });
        view! {
          <div class="mt-2 rounded-lg border bg-muted/30 p-3">
            <div class="mb-2 flex items-center justify-between text-xs">
              <span class="font-semibold">
                {if confirmed > 0 {
                  tf(
                    "网格 {} · {} 条 · {} 已确认",
                    &[&label, &list.len().to_string(), &confirmed.to_string()],
                  )
                } else {
                  tf("网格 {} · {} 条", &[&(label).to_string(), &list.len().to_string()])
                }}
              </span>
              <button
                type="button"
                class="text-muted-foreground transition-colors hover:text-foreground"
                on:click=move |_| state.selected_square.set(None)
              >
                {move || t("关闭")}
              </button>
            </div>
            {bearing_line.map(|line| {
              view! {
                <div class="mb-2 text-xs tabular-nums text-muted-foreground">{line}</div>
              }
            })}
            <div class="flex flex-wrap gap-1.5">
              {list
                .iter()
                .map(|e| {
                  let band = Some(e.band_label()).filter(|b| !b.is_empty()).unwrap_or_else(|| "—".to_owned());
                  // 显示完整网格码：6 位时体现 subsquare，4 位时仅 square。
                  let grid = e.gridsquare.trim().to_ascii_uppercase();
                  let entity = e.entity().map(|x| x.name).unwrap_or("");
                  let rst = if e.rst_sent.is_empty() && e.rst_rcvd.is_empty() {
                    String::new()
                  } else {
                    let s = if e.rst_sent.is_empty() { "—".to_string() } else { e.rst_sent.clone() };
                    let r = if e.rst_rcvd.is_empty() { "—".to_string() } else { e.rst_rcvd.clone() };
                    format!("RST {s}/{r}")
                  };
                  let span_class = if e.qsl_rcvd {
                    "rounded bg-muted/60 px-2 py-0.5 font-mono text-xs text-emerald-600 dark:text-emerald-400"
                  } else if e.qsl_sent {
                    "rounded bg-muted/60 px-2 py-0.5 font-mono text-xs text-amber-600 dark:text-amber-400"
                  } else {
                    "rounded bg-muted/60 px-2 py-0.5 font-mono text-xs"
                  };
                  let mut detail = format!("{} · {} · {} · {}", e.callsign, grid, band, e.mode);
                  if !rst.is_empty() {
                    detail.push_str(&format!(" · {rst}"));
                  }
                  if !e.name.is_empty() {
                    detail.push_str(&format!(" · {}", e.name));
                  }
                  if !e.qth.is_empty() {
                    detail.push_str(&format!(" · {}", e.qth));
                  }
                  detail.push_str(&format!(" · {}", e.date));
                  if !entity.is_empty() {
                    detail.push_str(&format!(" · {entity}"));
                  }
                  let tooltip = tf(
                    "时间 {} · 备注 {}",
                    &[
                      &if e.time.is_empty() { "—".to_string() } else { e.time.clone() },
                      &if e.remark.is_empty() { "—".to_string() } else { e.remark.clone() },
                    ],
                  );
                  view! {
                    <span class=span_class title=tooltip>{detail}</span>
                  }
                })
                .collect_view()}
            </div>
          </div>
        }
      })
    }}
  }
}

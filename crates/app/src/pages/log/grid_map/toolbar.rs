//! 网格地图工具栏：搜索定位 + 波段 / 模式 / DXCC / 年份 / QSL 筛选 + 灰线 / 通联路径开关。

use leptos::prelude::*;

use crate::ui::{Size, Variant, button_class, input_class};

use super::super::grid_filter::QslStatus;
use super::state::GridMapState;
use crate::i18n::t;

#[component]
pub(super) fn GridToolbar(state: GridMapState) -> impl IntoView {
  let bands = state.data.with_value(|d| d.bands.clone());
  let mode_list = state.data.with_value(|d| d.mode_list.clone());
  let dxcc_list = state.data.with_value(|d| d.dxcc_list.clone());
  let year_list = state.data.with_value(|d| d.year_list.clone());
  let paths_disabled = state.data.with_value(|d| d.home_pos.is_none());

  view! {
    // 工具栏：搜索定位 + 波段 / 模式筛选 + 灰线开关
    <div class="mb-2 flex flex-wrap items-center gap-2 text-xs">
      <div class="flex items-center gap-1">
        <input
          type="text"
          placeholder=move || t("定位网格，如 OM89EW")
          aria-label=move || t("定位网格")
          class=input_class("h-8 w-36 font-mono uppercase")
          prop:value=move || state.search_input.get()
          on:input=move |e| state.search_input.set(event_target_value(&e).to_uppercase())
        />
        <button type="button" class=button_class(Variant::Outline, Size::Sm, "") on:click=move |_| state.do_search()>
          {move || t("定位")}
        </button>
      </div>
      <input
        type="text"
        placeholder=move || t("呼号搜索")
        aria-label=move || t("呼号搜索")
        class=input_class("h-8 w-32 font-mono uppercase")
        prop:value=move || state.callsign_query.get()
        on:input=move |e| state.callsign_query.set(event_target_value(&e).to_uppercase())
      />
      <select
        aria-label=move || t("波段筛选")
        class=input_class("h-8 w-auto")
        on:change=move |e| {
          let v = event_target_value(&e);
          state.band_filter.set(if v.is_empty() { None } else { Some(v) });
        }
      >
        <option value="">{move || t("全部波段")}</option>
        {bands
          .iter()
          .map(|b| {
            view! { <option value=b.clone()>{b.clone()}</option> }
          })
          .collect_view()}
      </select>
      <select
        aria-label=move || t("模式筛选")
        class=input_class("h-8 w-auto")
        on:change=move |e| {
          let v = event_target_value(&e);
          state.mode_filter.set(if v.is_empty() { None } else { Some(v) });
        }
      >
        <option value="">{move || t("全部模式")}</option>
        {mode_list
          .iter()
          .map(|m| {
            view! { <option value=m.clone()>{m.clone()}</option> }
          })
          .collect_view()}
      </select>
      <select
        aria-label=move || t("实体筛选")
        class=input_class("h-8 w-auto")
        on:change=move |e| {
          let v = event_target_value(&e);
          state.dxcc_filter.set(if v.is_empty() { None } else { Some(v) });
        }
      >
        <option value="">{move || t("全部实体")}</option>
        {dxcc_list
          .iter()
          .map(|d| {
            view! { <option value=d.clone()>{d.clone()}</option> }
          })
          .collect_view()}
      </select>
      <select
        aria-label=move || t("年份筛选")
        class=input_class("h-8 w-auto")
        on:change=move |e| {
          let v = event_target_value(&e);
          state.year_filter.set(if v.is_empty() { None } else { Some(v) });
        }
      >
        <option value="">{move || t("全部年份")}</option>
        {year_list
          .iter()
          .map(|y| {
            view! { <option value=y.clone()>{y.clone()}</option> }
          })
          .collect_view()}
      </select>
      <select
        aria-label=move || t("QSL 筛选")
        class=input_class("h-8 w-auto")
        on:change=move |e| {
          let v = event_target_value(&e);
          state.qsl_filter.set(match v.as_str() {
            "confirmed" => Some(QslStatus::Confirmed),
            "sent_pending" => Some(QslStatus::SentPending),
            "not_sent" => Some(QslStatus::NotSent),
            _ => None,
          });
        }
      >
        <option value="">{move || t("全部 QSL")}</option>
        <option value="confirmed">{move || t("已确认")}</option>
        <option value="sent_pending">{move || t("已寄未确认")}</option>
        <option value="not_sent">{move || t("未寄出")}</option>
      </select>
      <label class="flex cursor-pointer items-center gap-1.5 text-muted-foreground">
        <input
          type="checkbox"
          class="size-4 accent-primary"
          prop:checked=move || state.show_grayline.get()
          on:change=move |e| state.show_grayline.set(event_target_checked(&e))
        />
        {move || t("灰线")}
      </label>
      <label
        class="flex cursor-pointer items-center gap-1.5 text-muted-foreground"
        title=if paths_disabled { t("需先在本台信息中填写网格") } else { t("从本台到每个通联网格的大圆路径") }
      >
        <input
          type="checkbox"
          class="size-4 accent-primary"
          prop:disabled=paths_disabled
          prop:checked=move || state.show_paths.get()
          on:change=move |e| state.show_paths.set(event_target_checked(&e))
        />
        {move || t("通联路径")}
      </label>
    </div>
  }
}

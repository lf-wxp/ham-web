//! 网格地图工具栏：搜索定位 + 波段 / 模式 / DXCC / 年份 / QSL 筛选 + 灰线 / 通联路径开关。

use leptos::prelude::*;

use super::super::grid_filter::QslStatus;
use super::state::GridMapState;
use crate::i18n::t;
use crate::ui::{Button, ControlSize, Input, NativeSelect, SelectOption, Size, Switch, Variant};

#[component]
pub(super) fn GridToolbar(state: GridMapState) -> impl IntoView {
  let bands = state.data.with_value(|d| d.bands.clone());
  let mode_list = state.data.with_value(|d| d.mode_list.clone());
  let dxcc_list = state.data.with_value(|d| d.dxcc_list.clone());
  let year_list = state.data.with_value(|d| d.year_list.clone());
  let paths_disabled = state.data.with_value(|d| d.home_pos.is_none());

  // 选项值即展示文案，直接用 `&str` 构造。
  let plain = |list: &[String]| -> Vec<SelectOption> {
    list
      .iter()
      .map(|v| SelectOption::new(v.as_str(), v.as_str()))
      .collect()
  };
  let band_options = plain(&bands);
  let mode_options = plain(&mode_list);
  let dxcc_options = plain(&dxcc_list);
  let year_options = plain(&year_list);
  let qsl_options = vec![
    SelectOption::new("confirmed", t("log.confirmed")),
    SelectOption::new("sent_pending", t("log.sent-unconfirmed")),
    SelectOption::new("not_sent", t("log.not-sent")),
  ];

  view! {
    // 工具栏：搜索定位 + 波段 / 模式筛选 + 灰线开关
    <div class="mb-2 flex flex-wrap items-center gap-2 text-xs">
      <div class="flex items-center gap-1">
        <Input
          value=state.search_input
          on_change=Callback::new(move |v: String| state.search_input.set(v.to_uppercase()))
          size=ControlSize::Sm
          placeholder=Signal::derive(move || t("log.locate-grid-e-g"))
          aria_label=Signal::derive(move || t("log.locate-grid"))
          class="w-36 font-mono uppercase"
        />
        <Button variant=Variant::Outline size=Size::Sm on_click=Callback::new(move |_| state.do_search())>
          {move || t("log.locate")}
        </Button>
      </div>
      <Input
        value=state.callsign_query
        on_change=Callback::new(move |v: String| state.callsign_query.set(v.to_uppercase()))
        size=ControlSize::Sm
        placeholder=Signal::derive(move || t("log.callsign-search"))
        aria_label=Signal::derive(move || t("log.callsign-search"))
        class="w-32 font-mono uppercase"
      />
      <NativeSelect
        value=Signal::derive(move || state.band_filter.get().unwrap_or_default())
        on_change=Callback::new(move |v: String| state.band_filter.set(if v.is_empty() { None } else { Some(v) }))
        options=band_options
        placeholder=Signal::derive(move || t("log.all-bands"))
        size=ControlSize::Sm
        aria_label=Signal::derive(move || t("log.band-filter"))
        class="w-auto"
      />
      <NativeSelect
        value=Signal::derive(move || state.mode_filter.get().unwrap_or_default())
        on_change=Callback::new(move |v: String| state.mode_filter.set(if v.is_empty() { None } else { Some(v) }))
        options=mode_options
        placeholder=Signal::derive(move || t("log.all-modes"))
        size=ControlSize::Sm
        aria_label=Signal::derive(move || t("log.mode-filter"))
        class="w-auto"
      />
      <NativeSelect
        value=Signal::derive(move || state.dxcc_filter.get().unwrap_or_default())
        on_change=Callback::new(move |v: String| state.dxcc_filter.set(if v.is_empty() { None } else { Some(v) }))
        options=dxcc_options
        placeholder=Signal::derive(move || t("log.all-entities"))
        size=ControlSize::Sm
        aria_label=Signal::derive(move || t("log.entity-filter"))
        class="w-auto"
      />
      <NativeSelect
        value=Signal::derive(move || state.year_filter.get().unwrap_or_default())
        on_change=Callback::new(move |v: String| state.year_filter.set(if v.is_empty() { None } else { Some(v) }))
        options=year_options
        placeholder=Signal::derive(move || t("log.all-years"))
        size=ControlSize::Sm
        aria_label=Signal::derive(move || t("log.year-filter"))
        class="w-auto"
      />
      <NativeSelect
        value=Signal::derive(move || {
          match state.qsl_filter.get() {
            Some(QslStatus::Confirmed) => "confirmed".to_owned(),
            Some(QslStatus::SentPending) => "sent_pending".to_owned(),
            Some(QslStatus::NotSent) => "not_sent".to_owned(),
            None => String::new(),
          }
        })
        on_change=Callback::new(move |v: String| {
          state
            .qsl_filter
            .set(match v.as_str() {
              "confirmed" => Some(QslStatus::Confirmed),
              "sent_pending" => Some(QslStatus::SentPending),
              "not_sent" => Some(QslStatus::NotSent),
              _ => None,
            })
        })
        options=qsl_options
        placeholder=Signal::derive(move || t("log.all-qsl"))
        size=ControlSize::Sm
        aria_label=Signal::derive(move || t("log.qsl-filter"))
        class="w-auto"
      />
      <label class="flex cursor-pointer items-center gap-1.5 text-muted-foreground">
        <Switch
          checked=state.show_grayline
          on_change=Callback::new(move |v| state.show_grayline.set(v))
        />
        {move || t("log.grayline")}
      </label>
      <label
        class="flex cursor-pointer items-center gap-1.5 text-muted-foreground"
        title=if paths_disabled { t("log.set-your-grid-in") } else { t("log.great-circle-paths-from") }
      >
        <Switch
          checked=state.show_paths
          on_change=Callback::new(move |v| state.show_paths.set(v))
          disabled=Signal::derive(move || paths_disabled)
        />
        {move || t("log.qso-paths")}
      </label>
    </div>
  }
}

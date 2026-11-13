//! 大气噪声（ITU-R P.372 第 5 部分）要的四个量：月份、UTC 小时、纬度、经度。
//!
//! 台站链路预算与外部噪声两个计算器共用这一组输入，标签、提示与解析口径只在这里写一遍。
//! 四个 `Field` 作为兄弟节点直接铺进调用方的网格（不自带容器），由调用方决定放在哪、何时出现。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Field, NumberField};
use crate::util::unique_id;

/// 月 / UTC 小时 / 纬度 / 经度四个输入框。`id_prefix` 区分同页的不同使用处（`slink`、`xnoise`）。
#[component]
pub(super) fn AtmosphereFields(
  month: RwSignal<String>,
  hour: RwSignal<String>,
  lat: RwSignal<String>,
  lon: RwSignal<String>,
  id_prefix: &'static str,
) -> impl IntoView {
  let month_id = unique_id(&format!("{id_prefix}-atm-month"));
  let hour_id = unique_id(&format!("{id_prefix}-atm-hour"));
  let lat_id = unique_id(&format!("{id_prefix}-atm-lat"));
  let lon_id = unique_id(&format!("{id_prefix}-atm-lon"));

  view! {
    <Field
      label=Signal::derive(move || t("tools.station-link-atm-month"))
      hint=Signal::derive(move || t("tools.station-link-atm-month-hint"))
      r#for=month_id.clone()
    >
      <NumberField id=month_id value=Signal::derive(move || month.get()) on_change=Callback::new(move |v: String| month.set(v)) controls=false />
    </Field>
    <Field
      label=Signal::derive(move || t("tools.station-link-atm-hour"))
      hint=Signal::derive(move || t("tools.station-link-atm-hour-hint"))
      r#for=hour_id.clone()
    >
      <NumberField id=hour_id value=Signal::derive(move || hour.get()) on_change=Callback::new(move |v: String| hour.set(v)) controls=false />
    </Field>
    <Field
      label=Signal::derive(move || t("tools.station-link-atm-lat"))
      hint=Signal::derive(move || t("tools.station-link-atm-position-hint"))
      r#for=lat_id.clone()
    >
      <NumberField id=lat_id value=Signal::derive(move || lat.get()) on_change=Callback::new(move |v: String| lat.set(v)) controls=false />
    </Field>
    <Field
      label=Signal::derive(move || t("tools.station-link-atm-lon"))
      hint=Signal::derive(move || t("tools.station-link-atm-position-hint"))
      r#for=lon_id.clone()
    >
      <NumberField id=lon_id value=Signal::derive(move || lon.get()) on_change=Callback::new(move |v: String| lon.set(v)) controls=false />
    </Field>
  }
}

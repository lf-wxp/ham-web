//! 本台信息面板（站点级设置）：呼号 / 操作员 / 网格 / 设备 / 天线，写入 ADIF 的 STATION_* 字段。

use leptos::prelude::*;

use crate::ui::{Size, Variant, button_class, input_class};

use super::StationInfo;
use crate::i18n::t;

#[component]
pub(super) fn StationPanel(station: RwSignal<StationInfo>, on_save: Callback<()>) -> impl IntoView {
  let show_station = RwSignal::new(false);

  view! {
    <section class="rounded-xl border bg-card">
      <button
        type="button"
        class="flex w-full items-center justify-between px-4 py-3 text-left text-sm font-semibold"
        on:click=move |_| show_station.update(|v| *v = !*v)
      >
        <span>{move || t("本台信息")}</span>
        <span class="text-xs font-normal text-muted-foreground">
          {move || {
            let cs = station.get().callsign.clone();
            if cs.is_empty() {
              t("未设置呼号（点击设置）")
            } else {
              cs
            }
          }}
        </span>
      </button>
      {move || {
        show_station.get().then(|| {
          view! {
            <div class="border-t p-4">
              <p class="mb-3 text-xs text-muted-foreground">
                {move || t("本台信息将写入每条 ADIF 记录的 STATION_CALLSIGN / OPERATOR / MY_GRIDSQUARE / MY_RIG / MY_ANTENNA 字段。")}
              </p>
              <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
                <label class="flex flex-col gap-1.5 text-sm">
                  <span class="text-xs text-muted-foreground">{move || t("本台呼号")}</span>
                  <input
                    type="text"
                    placeholder="BG4XXX"
                    class=input_class("uppercase")
                    prop:value=move || station.get().callsign.clone()
                    on:input=move |e| station.update(|s| s.callsign = event_target_value(&e).to_uppercase())
                  />
                </label>
                <label class="flex flex-col gap-1.5 text-sm">
                  <span class="text-xs text-muted-foreground">{move || t("操作员")}</span>
                  <input
                    type="text"
                    placeholder=move || t("同呼号")
                    class=input_class("")
                    prop:value=move || station.get().operator.clone()
                    on:input=move |e| station.update(|s| s.operator = event_target_value(&e))
                  />
                </label>
                <label class="flex flex-col gap-1.5 text-sm">
                  <span class="text-xs text-muted-foreground">{move || t("本台网格")}</span>
                  <input
                    type="text"
                    placeholder="OM89EW"
                    class=input_class("uppercase")
                    prop:value=move || station.get().gridsquare.clone()
                    on:input=move |e| station.update(|s| s.gridsquare = event_target_value(&e).to_uppercase())
                  />
                </label>
                <label class="flex flex-col gap-1.5 text-sm">
                  <span class="text-xs text-muted-foreground">{move || t("设备")}</span>
                  <input
                    type="text"
                    placeholder="FT-710"
                    class=input_class("")
                    prop:value=move || station.get().rig.clone()
                    on:input=move |e| station.update(|s| s.rig = event_target_value(&e))
                  />
                </label>
                <label class="flex flex-col gap-1.5 text-sm">
                  <span class="text-xs text-muted-foreground">{move || t("天线")}</span>
                  <input
                    type="text"
                    placeholder="DP 20m"
                    class=input_class("")
                    prop:value=move || station.get().antenna.clone()
                    on:input=move |e| station.update(|s| s.antenna = event_target_value(&e))
                  />
                </label>
              </div>
              <div class="mt-3">
                <button
                  type="button"
                  class=button_class(Variant::Default, Size::Sm, "")
                  on:click=move |_| on_save.run(())
                >
                  {move || t("保存本台信息")}
                </button>
              </div>
            </div>
          }
        })
      }}
    </section>
  }
}

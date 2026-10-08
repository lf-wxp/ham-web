//! 本台信息面板（站点级设置）：呼号 / 操作员 / 网格 / 设备 / 天线，写入 ADIF 的 STATION_* 字段。

use leptos::prelude::*;

use crate::ui::{Button, Field, Input, Size, Variant};

use super::StationInfo;
use crate::i18n::t;
use crate::util::unique_id;

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
        <span>{move || t("log.station-info")}</span>
        <span class="text-xs font-normal text-muted-foreground">
          {move || {
            let cs = station.get().callsign.clone();
            if cs.is_empty() {
              t("log.no-callsign-set-click")
            } else {
              cs
            }
          }}
        </span>
      </button>
      {move || {
        show_station.get().then(|| {
          // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框。
          // id 在这里现生成：`then` 的闭包是 `FnOnce`，可以自由移出，而外层 `move ||`
          // 必须保持 `Fn`（可重复重渲染），不能捕获这些 id。
          let callsign_id = unique_id("station-callsign");
          let operator_id = unique_id("station-operator");
          let grid_id = unique_id("station-grid");
          let rig_id = unique_id("station-rig");
          let antenna_id = unique_id("station-antenna");
          view! {
            <div class="border-t p-4">
              <p class="mb-3 text-xs text-muted-foreground">
                {move || t("log.station-info-is-written")}
              </p>
              <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
                <Field label=Signal::derive(move || t("log.station-callsign")) r#for=callsign_id.clone()>
                  <Input
                    id=callsign_id.clone()
                    value=Signal::derive(move || station.get().callsign.clone())
                    on_change=Callback::new(move |v: String| station.update(|s| s.callsign = v.to_uppercase()))
                    placeholder="BG4XXX"
                    class="uppercase"
                  />
                </Field>
                <Field label=Signal::derive(move || t("log.operator")) r#for=operator_id.clone()>
                  <Input
                    id=operator_id.clone()
                    value=Signal::derive(move || station.get().operator.clone())
                    on_change=Callback::new(move |v: String| station.update(|s| s.operator = v))
                    placeholder=Signal::derive(move || t("log.same-as-callsign"))
                  />
                </Field>
                <Field label=Signal::derive(move || t("log.station-grid")) r#for=grid_id.clone()>
                  <Input
                    id=grid_id.clone()
                    value=Signal::derive(move || station.get().gridsquare.clone())
                    on_change=Callback::new(move |v: String| station.update(|s| s.gridsquare = v.to_uppercase()))
                    placeholder="OM89EW"
                    class="uppercase"
                  />
                </Field>
                <Field label=Signal::derive(move || t("log.rig")) r#for=rig_id.clone()>
                  <Input
                    id=rig_id.clone()
                    value=Signal::derive(move || station.get().rig.clone())
                    on_change=Callback::new(move |v: String| station.update(|s| s.rig = v))
                    placeholder="FT-710"
                  />
                </Field>
                <Field label=Signal::derive(move || t("log.antenna")) r#for=antenna_id.clone()>
                  <Input
                    id=antenna_id.clone()
                    value=Signal::derive(move || station.get().antenna.clone())
                    on_change=Callback::new(move |v: String| station.update(|s| s.antenna = v))
                    placeholder="DP 20m"
                  />
                </Field>
              </div>
              <div class="mt-3">
                <Button
                  variant=Variant::Default
                  size=Size::Sm
                  on_click=Callback::new(move |_| on_save.run(()))
                >
                  {move || t("log.save-station-info")}
                </Button>
              </div>
            </div>
          }
        })
      }}
    </section>
  }
}

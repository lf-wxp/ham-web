use ham_web_core::eme_plan::{self, Mutual, signed, utc_stamp};
use ham_web_core::moon::path_loss_delta_db;
use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::ui::{
  Button, Field, Input, NumberField, SelectOption, SimpleSelect, Size, Table, TableBody, TableCell,
  TableDensity, TableHead, TableHeader, TableRow, Variant,
};
use crate::util::unique_id;

use super::window_list::WindowList;

/// 常见 EME 频段（MHz）。取各业余波段的名义频率：回波多普勒与频率成正比，
/// 0.1 MHz 的差只是 0.07%（144 MHz 下约 0.2 Hz），不必精确到各国的 EME 小段。
const BANDS_MHZ: &[u32] = &[50, 144, 432, 1296, 2304, 3400, 5760, 10368];
fn now_secs() -> f64 {
  js_sys::Date::now() / 1000.0
}

/// 月球位置与 EME 窗口：本台（以及对方）的方位 / 仰角、月距、回波多普勒、月相与可用窗口。
#[component]
pub(super) fn EmePlanner() -> impl IntoView {
  // 默认取日志里当前台站的网格；缺网格不硬算（`compute` 前就挡住），见模板里的 `radio.eme-need-grid` 提示。
  let my_grid = RwSignal::new(use_log_store().active_station_untracked().gridsquare);
  let dx_grid = RwSignal::new(String::new());
  let band = RwSignal::new("144".to_owned());
  let min_el = RwSignal::new(5.0_f64);
  let now = RwSignal::new(now_secs());

  let my_id = unique_id("eme-my-grid");
  let dx_id = unique_id("eme-dx-grid");
  let band_id = unique_id("eme-band");
  let el_id = unique_id("eme-min-el");

  let band_options: Vec<SelectOption> = BANDS_MHZ
    .iter()
    .map(|m| SelectOption::new(m.to_string(), format!("{m} MHz")))
    .collect();

  // 输入校验与数值都在 core 的 `eme_plan::plan` 里（带单测），这里只把信号喂进去。
  let plan = Memo::new(move |_| {
    let mhz = band.get().parse::<u32>().ok()?;
    eme_plan::plan(&my_grid.get(), &dx_grid.get(), mhz, min_el.get(), now.get())
  });

  let readout = if crate::kit::is_modern_at_mount() {
    "md-readout sm:col-span-2 space-y-1 tabular-nums"
  } else {
    "sm:col-span-2 space-y-1 bg-inset px-3 py-2 text-sm text-muted-foreground tabular-nums"
  };

  view! {
    <SectionCard title=Signal::derive(move || t("radio.eme-planner-title"))>
      <div class="grid gap-3 p-4 sm:grid-cols-2 [&>*]:min-w-0">
        <p class="sm:col-span-2 text-sm text-muted-foreground">{move || t("radio.eme-planner-intro")}</p>
        <Field
          label=Signal::derive(move || t("log.station-grid"))
          hint=Signal::derive(move || t("radio.eme-my-grid-hint"))
          r#for=my_id.clone()
        >
          <Input id=my_id value=my_grid on_change=Callback::new(move |v: String| my_grid.set(v)) />
        </Field>
        <Field
          label=Signal::derive(move || t("radio.eme-dx-grid"))
          hint=Signal::derive(move || t("radio.eme-dx-grid-hint"))
          r#for=dx_id.clone()
        >
          <Input id=dx_id value=dx_grid on_change=Callback::new(move |v: String| dx_grid.set(v)) />
        </Field>
        <Field label=Signal::derive(move || t("radio.eme-band")) r#for=band_id.clone()>
          <SimpleSelect
            id=band_id
            value=band
            on_change=Callback::new(move |v: String| band.set(v))
            options=band_options
            aria_label=Signal::derive(move || t("radio.eme-band"))
          />
        </Field>
        <Field label=Signal::derive(move || t("radio.eme-min-el")) r#for=el_id.clone()>
          <NumberField
            id=el_id
            value=Signal::derive(move || min_el.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                min_el.set(v);
              }
            })
            controls=false
          />
        </Field>
        <div class="sm:col-span-2">
          <Button
            variant=Variant::Outline
            size=Size::Sm
            on_click=Callback::new(move |_| now.set(now_secs()))
          >
            {move || t("radio.eme-refresh")}
          </Button>
        </div>

        <div class=readout role="status">
          {move || match plan.get() {
            None => view! { <span>{t("radio.eme-need-grid")}</span> }.into_any(),
            Some(p) => {
              let up = p.here.el_deg >= min_el.get();
              let trend = if p.phase.waxing { t("radio.eme-waxing") } else { t("radio.eme-waning") };
              let loss = path_loss_delta_db(p.here.range_km).map_or_else(|| "—".to_owned(), |d| signed(d, 2));
              view! {
                <div class="text-foreground">{tf("radio.eme-now", &[&utc_stamp(p.now)])}</div>
                <div>
                  {tf(
                    "radio.eme-now-pos",
                    &[
                      &format!("{:.1}", p.here.az_deg),
                      &format!("{:.1}", p.here.el_deg),
                      &format!("{:.0}", p.here.range_km),
                    ],
                  )}
                </div>
                <div>{if up { t("radio.eme-state-up") } else { t("radio.eme-state-down") }}</div>
                <div>{tf("radio.eme-now-loss", &[&loss])}</div>
                <div>
                  {tf(
                    "radio.eme-now-doppler",
                    &[&band.get(), &signed(p.doppler_hz, 0), &signed(p.rate, 3)],
                  )}
                </div>
                <div>{tf("radio.eme-phase", &[&format!("{:.0}", p.phase.illumination_pct), &trend])}</div>
              }
              .into_any()
            }
          }}
        </div>

        {move || {
          plan.get().map(|p| {
            let rows = p.rows;
            let none = rows.is_empty();
            view! {
              <div class="sm:col-span-2 space-y-2">
                <div class="text-xs text-muted-foreground">{move || t("radio.eme-table")}</div>
                {if none {
                  view! { <p class="text-sm text-muted-foreground">{move || t("radio.eme-no-rows")}</p> }.into_any()
                } else {
                  view! {
                    <Table aria_label=Signal::derive(move || t("radio.eme-table")) density=TableDensity::Compact>
                      <TableHead class="text-xs text-muted-foreground">
                        <TableRow>
                          <TableHeader scope="col" class="px-2 py-1.5 font-normal">{move || t("radio.eme-col-time")}</TableHeader>
                          <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("radio.eme-col-az")}</TableHeader>
                          <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("radio.eme-col-el")}</TableHeader>
                          <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("radio.eme-col-range")}</TableHeader>
                          <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("radio.eme-col-loss")}</TableHeader>
                          <TableHeader scope="col" class="px-2 py-1.5 text-right font-normal">{move || t("radio.eme-col-doppler")}</TableHeader>
                        </TableRow>
                      </TableHead>
                      <TableBody class="font-mono tabular-nums">
                        {rows
                          .into_iter()
                          .map(|r| {
                            let loss = path_loss_delta_db(r.topo.range_km)
                              .map_or_else(|| "—".to_owned(), |d| signed(d, 2));
                            view! {
                              <TableRow>
                                <TableCell class="px-2 py-1">{utc_stamp(r.t)}</TableCell>
                                <TableCell class="px-2 py-1 text-right">{format!("{:.1}", r.topo.az_deg)}</TableCell>
                                <TableCell class="px-2 py-1 text-right">{format!("{:.1}", r.topo.el_deg)}</TableCell>
                                <TableCell class="px-2 py-1 text-right">{format!("{:.0}", r.topo.range_km)}</TableCell>
                                <TableCell class="px-2 py-1 text-right">{loss}</TableCell>
                                <TableCell class="px-2 py-1 text-right">{signed(r.doppler_hz, 0)}</TableCell>
                              </TableRow>
                            }
                          })
                          .collect_view()}
                      </TableBody>
                    </Table>
                  }
                  .into_any()
                }}
              </div>
              <div class="sm:col-span-2 space-y-2">
                <div class="text-xs text-muted-foreground">{move || t("radio.eme-windows-mine")}</div>
                <WindowList list=p.mine id="eme-windows-mine" />
                {match p.mutual {
                  Mutual::NotRequested => ().into_any(),
                  Mutual::InvalidGrid => view! { <p class="text-sm text-muted-foreground">{move || t("radio.eme-invalid-dx")}</p> }.into_any(),
                  Mutual::Windows(list) => view! {
                    <div class="text-xs text-muted-foreground">{move || t("radio.eme-windows-mutual")}</div>
                    <WindowList list=list id="eme-windows-mutual" />
                  }
                  .into_any(),
                }}
              </div>
            }
          })
        }}
        <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("radio.eme-note")}</p>
      </div>
    </SectionCard>
  }
}

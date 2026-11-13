//! 外部噪声（ITU-R P.372）：给定月份、UTC 小时、地点与环境，按业余波段列出大气、人为、银河三种噪声
//! 与它们的合成值，并指出哪一种在主导。

use ham_web_core::atmospheric_noise::{NoiseSource, band_table, month_from_f64};
use ham_web_core::noise_model::Environment;
use leptos::prelude::*;

use super::atmosphere_fields::AtmosphereFields;
use super::num;
use crate::i18n::t;
use crate::ui::{
  Field, SelectOption, SimpleSelect, Table, TableBody, TableCell, TableDensity, TableHead,
  TableHeader, TableRow,
};
use crate::util::unique_id;

/// 可选的环境（银河噪声是合成里固定存在的分量，不是一种环境）。
const ENVIRONMENTS: [Environment; 4] = [
  Environment::City,
  Environment::Residential,
  Environment::Rural,
  Environment::QuietRural,
];

/// 环境的词条 key（与台站链路预算共用同一批名字，名字里带 P.372 的曲线字母）。
fn env_key(env: Environment) -> &'static str {
  match env {
    Environment::City => "tools.station-link-env-city",
    Environment::Residential => "tools.station-link-env-residential",
    Environment::Rural => "tools.station-link-env-rural",
    Environment::QuietRural | Environment::Galactic => "tools.station-link-env-quiet-rural",
  }
}

/// 主导来源的词条 key。
fn source_key(s: NoiseSource) -> &'static str {
  match s {
    NoiseSource::Atmospheric => "tools.external-noise-src-atmospheric",
    NoiseSource::ManMade => "tools.external-noise-src-man-made",
    NoiseSource::Galactic => "tools.external-noise-src-galactic",
  }
}

#[component]
pub(super) fn ExternalNoise() -> impl IntoView {
  let env = RwSignal::new(String::from("1"));
  // 默认「7 月、UTC 12 时、北京」，与台站链路预算的大气噪声示例同口径。
  let month = RwSignal::new(String::from("7"));
  let hour = RwSignal::new(String::from("12"));
  let lat = RwSignal::new(String::from("40"));
  let lon = RwSignal::new(String::from("116"));

  let env_options: Vec<SelectOption> = ENVIRONMENTS
    .iter()
    .enumerate()
    .map(|(i, &e)| SelectOption::new(i.to_string(), Signal::derive(move || t(env_key(e)))))
    .collect();

  let env_id = unique_id("xnoise-env");

  let table = move || {
    let e = ENVIRONMENTS[env.get().parse::<usize>().ok()?.min(ENVIRONMENTS.len() - 1)];
    // 月份必须是 1–12 的整数：`as u32` 会把 7.9 截成 7，悄悄算了另一个月。
    let m = month_from_f64(num(month)?)?;
    band_table(e, m, num(hour)?, num(lat)?, num(lon)?)
  };

  view! {
    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("tools.station-link-env")) r#for=env_id.clone()>
        <SimpleSelect
          id=env_id
          value=Signal::derive(move || env.get())
          on_change=Callback::new(move |v: String| env.set(v))
          options=env_options
          aria_label=Signal::derive(move || t("tools.station-link-env"))
        />
      </Field>
      <AtmosphereFields month=month hour=hour lat=lat lon=lon id_prefix="xnoise" />

      {move || match table() {
        None => {
          view! {
            <div class=super::result_class()>
              <p>{t("tools.external-noise-invalid")}</p>
            </div>
          }
            .into_any()
        }
        Some(rows) => {
          view! {
            // 不要给 `Table` 再套一层 `div.sm:col-span-2`（见 `atu_synth.rs` 的同一条注释）。
            <Table
              aria_label=Signal::derive(move || t("tools.external-noise-table"))
              class="min-w-[560px]"
              wrapper_class="border sm:col-span-2"
              density=TableDensity::Compact
            >
              <TableHead>
                <TableRow>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.frequency-mhz")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.external-noise-col-atm")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.external-noise-col-man-made")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.external-noise-col-galactic")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.external-noise-col-total")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.external-noise-col-deciles")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.external-noise-col-dominant")}</TableHeader>
                </TableRow>
              </TableHead>
              <TableBody>
                {rows
                  .into_iter()
                  .map(|r| {
                    let mono = crate::kit::table_cell_class("font-mono tabular-nums");
                    view! {
                      <TableRow>
                        <TableCell class=mono.clone()>{r.freq_mhz.to_string()}</TableCell>
                        <TableCell class=mono.clone()>{format!("{:.1}", r.atmospheric_db)}</TableCell>
                        <TableCell class=mono.clone()>{format!("{:.1}", r.man_made_db)}</TableCell>
                        <TableCell class=mono.clone()>{format!("{:.1}", r.galactic_db)}</TableCell>
                        <TableCell class=mono.clone()>{format!("{:.1}", r.total.fa_db)}</TableCell>
                        <TableCell class=mono>
                          {format!("+{:.1} / −{:.1}", r.total.du_db, r.total.dl_db)}
                        </TableCell>
                        <TableCell class=crate::kit::table_cell_class("")>
                          {move || t(source_key(r.dominant()))}
                        </TableCell>
                      </TableRow>
                    }
                  })
                  .collect_view()}
              </TableBody>
            </Table>
          }
            .into_any()
        }
      }}

      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.external-noise-notes")}</p>
    </div>
  }
}

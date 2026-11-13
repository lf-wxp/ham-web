//! 地波场强（ITU-R P.368 平滑地球法）：给定频率、地面常数、天线高度，算一点的场强与传输损耗，
//! 并列出 1–1000 km 一路的衰减。

use ham_web_core::ground_wave::{
  BELOW_NOISE_FLOOR_DBU_VM, GROUND_TYPES, GroundWaveInput, Method, Polarization,
  TABLE_DISTANCES_KM, solve,
};
use leptos::prelude::*;

use super::{num, result_class};
use crate::i18n::{t, tf};
use crate::ui::{
  Field, NumberField, SelectOption, SimpleSelect, Table, TableBody, TableCell, TableDensity,
  TableHead, TableHeader, TableRow,
};
use crate::util::unique_id;

/// 地面预设的词条 key（`GROUND_TYPES` 的 `id` 对应的显示名）。
///
/// 一条一条写出来而不是拼 key：`check-i18n` 只认字面量实参，拼出来的 key 会被判成死条目。
fn ground_key(id: &str) -> &'static str {
  match id {
    "sea-average" => "tools.ground-wave-g-sea-average",
    "sea-low-salinity" => "tools.ground-wave-g-sea-low-salinity",
    "land-good" => "tools.ground-wave-g-land-good",
    "wet" => "tools.ground-wave-g-wet",
    "fresh-water" => "tools.ground-wave-g-fresh-water",
    "land-fair" => "tools.ground-wave-g-land-fair",
    "medium-dry" => "tools.ground-wave-g-medium-dry",
    "dry" => "tools.ground-wave-g-dry",
    _ => "tools.ground-wave-g-very-dry",
  }
}

/// 解法的短名（表格列里用）。
fn method_key(m: Method) -> &'static str {
  match m {
    Method::FlatEarthCurve => "tools.ground-wave-method-flat",
    Method::ResidueSeries => "tools.ground-wave-method-residue",
  }
}

/// 下拉里「自己填 εr / σ」那一项的值（不会与任何 `GROUND_TYPES` 的 `id` 相同）。
const CUSTOM: &str = "custom";

/// dB 类读数：两位小数。
fn fmt_db(v: f64) -> String {
  format!("{v:.2}")
}

#[component]
pub(super) fn GroundWave() -> impl IntoView {
  let freq = RwSignal::new(String::from("1"));
  let dist = RwSignal::new(String::from("100"));
  let h_tx = RwSignal::new(String::from("0"));
  let h_rx = RwSignal::new(String::from("0"));
  // 1 kW 是 P.368 曲线与官方 GRWAVE 的参考功率，默认取它：读出来的场强可以直接对着曲线看。
  let power = RwSignal::new(String::from("1000"));
  let ns = RwSignal::new(String::from("315"));
  let ground = RwSignal::new(String::from("medium-dry"));
  let eps = RwSignal::new(String::from("15"));
  let sigma = RwSignal::new(String::from("0.001"));
  let vertical = RwSignal::new(true);

  let ground_options: Vec<SelectOption> = GROUND_TYPES
    .iter()
    .map(|g| {
      let key = ground_key(g.id);
      SelectOption::new(g.id, Signal::derive(move || t(key)))
    })
    .chain(std::iter::once(SelectOption::new(
      CUSTOM,
      Signal::derive(move || t("tools.ground-wave-custom")),
    )))
    .collect();
  let pol_options = vec![
    SelectOption::new(
      "v",
      Signal::derive(move || t("tools.ground-wave-pol-vertical")),
    ),
    SelectOption::new(
      "h",
      Signal::derive(move || t("tools.ground-wave-pol-horizontal")),
    ),
  ];

  let freq_id = unique_id("gwave-freq");
  let dist_id = unique_id("gwave-dist");
  let h_tx_id = unique_id("gwave-h-tx");
  let h_rx_id = unique_id("gwave-h-rx");
  let power_id = unique_id("gwave-power");
  let ns_id = unique_id("gwave-ns");
  let ground_id = unique_id("gwave-ground");
  let eps_id = unique_id("gwave-eps");
  let sigma_id = unique_id("gwave-sigma");
  let pol_id = unique_id("gwave-pol");

  // 当前的地面常数：预设取表，自填取输入框。
  let ground_constants = move || -> Option<(f64, f64)> {
    let g = ground.get();
    match GROUND_TYPES.iter().find(|x| x.id == g) {
      Some(x) => Some((x.epsilon_r, x.sigma_s_m)),
      None => Some((num(eps)?, num(sigma)?)),
    }
  };
  let polarization = move || {
    if vertical.get() {
      Polarization::Vertical
    } else {
      Polarization::Horizontal
    }
  };
  // 除距离以外的输入；距离由主结果与距离表分别填。
  let base = move |distance_km: f64| -> Option<GroundWaveInput> {
    let (epsilon_r, sigma_s_m) = ground_constants()?;
    Some(GroundWaveInput {
      h_tx_m: num(h_tx)?,
      h_rx_m: num(h_rx)?,
      freq_mhz: num(freq)?,
      power_w: num(power)?,
      surface_refractivity: num(ns)?,
      distance_km,
      epsilon_r,
      sigma_s_m,
      polarization: polarization(),
    })
  };
  let outcome = move || {
    let input = base(num(dist)?)?;
    solve(&input).map(|r| (input, r))
  };
  // 距离表只依赖「除距离以外」的输入：先用 1 km 把它们验一遍，通过了再逐行算。
  let rows = move || -> Option<Vec<(f64, Option<ham_web_core::ground_wave::GroundWaveResult>)>> {
    base(1.0)?.validate().then_some(())?;
    Some(
      TABLE_DISTANCES_KM
        .iter()
        .map(|&d| (d, base(d).and_then(|i| solve(&i))))
        .collect(),
    )
  };

  view! {
    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField id=freq_id value=Signal::derive(move || freq.get()) on_change=Callback::new(move |v: String| freq.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.distance-km")) r#for=dist_id.clone()>
        <NumberField id=dist_id value=Signal::derive(move || dist.get()) on_change=Callback::new(move |v: String| dist.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.ground-wave-ground"))
        hint=Signal::derive(move || t("tools.ground-wave-ground-hint"))
        r#for=ground_id.clone()
      >
        <SimpleSelect
          id=ground_id
          value=Signal::derive(move || ground.get())
          on_change=Callback::new(move |v: String| {
            // 选预设时顺手把 εr / σ 栏同步成它的值：切到「自己填」时是从这一项接着改，而不是跳回旧值。
            if let Some(g) = GROUND_TYPES.iter().find(|x| x.id == v) {
              eps.set(g.epsilon_r.to_string());
              sigma.set(g.sigma_s_m.to_string());
            }
            ground.set(v);
          })
          options=ground_options
          aria_label=Signal::derive(move || t("tools.ground-wave-ground"))
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.rain-pol")) r#for=pol_id.clone()>
        <SimpleSelect
          id=pol_id
          value=Signal::derive(move || String::from(if vertical.get() { "v" } else { "h" }))
          on_change=Callback::new(move |v: String| vertical.set(v == "v"))
          options=pol_options
          aria_label=Signal::derive(move || t("tools.rain-pol"))
        />
      </Field>
      // εr / σ 只在「自己填」时出现：选了预设还留着一个不生效的栏会误导。
      {move || {
        let (eps_id, sigma_id) = (eps_id.clone(), sigma_id.clone());
        (ground.get() == CUSTOM)
          .then(|| {
            view! {
              <Field label=Signal::derive(move || t("tools.nec-eps-r")) r#for=eps_id.clone()>
                <NumberField id=eps_id value=Signal::derive(move || eps.get()) on_change=Callback::new(move |v: String| eps.set(v)) controls=false />
              </Field>
              <Field label=Signal::derive(move || t("tools.nec-sigma-ground")) r#for=sigma_id.clone()>
                <NumberField id=sigma_id value=Signal::derive(move || sigma.get()) on_change=Callback::new(move |v: String| sigma.set(v)) controls=false />
              </Field>
            }
          })
      }}
      <Field
        label=Signal::derive(move || t("tools.ground-wave-h-tx"))
        hint=Signal::derive(move || t("tools.ground-wave-height-hint"))
        r#for=h_tx_id.clone()
      >
        <NumberField id=h_tx_id value=Signal::derive(move || h_tx.get()) on_change=Callback::new(move |v: String| h_tx.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.ground-wave-h-rx"))
        hint=Signal::derive(move || t("tools.ground-wave-height-hint"))
        r#for=h_rx_id.clone()
      >
        <NumberField id=h_rx_id value=Signal::derive(move || h_rx.get()) on_change=Callback::new(move |v: String| h_rx.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.transmit-power-w"))
        hint=Signal::derive(move || t("tools.ground-wave-power-hint"))
        r#for=power_id.clone()
      >
        <NumberField id=power_id value=Signal::derive(move || power.get()) on_change=Callback::new(move |v: String| power.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.ground-wave-ns"))
        hint=Signal::derive(move || t("tools.ground-wave-ns-hint"))
        r#for=ns_id.clone()
      >
        <NumberField id=ns_id value=Signal::derive(move || ns.get()) on_change=Callback::new(move |v: String| ns.set(v)) controls=false />
      </Field>

      {move || match outcome() {
        None => {
          view! {
            <div class=result_class()>
              <p>{t("tools.ground-wave-invalid")}</p>
            </div>
          }
            .into_any()
        }
        Some((input, r)) => {
          let line = tf(
            "tools.ground-wave-result",
            &[&fmt_db(r.field_dbu_vm), &fmt_db(r.basic_loss_db), &fmt_db(r.rx_power_dbm)],
          );
          let how = t(method_key(r.method));
          let constants = tf(
            "tools.ground-wave-constants",
            &[&input.epsilon_r.to_string(), &input.sigma_s_m.to_string()],
          );
          view! {
            <div class=format!("{} space-y-1 tabular-nums", result_class())>
              <p>{line}</p>
              <p>{tf("tools.ground-wave-method", &[&how])}</p>
              <p>{constants}</p>
              {r
                .is_below_any_noise_floor()
                .then(|| {
                  view! {
                    <p>
                      {tf("tools.ground-wave-below-noise-floor", &[&BELOW_NOISE_FLOOR_DBU_VM.to_string()])}
                    </p>
                  }
                })}
            </div>
          }
            .into_any()
        }
      }}

      {move || {
        rows()
          .map(|rows| {
            view! {
              // 不要给 `Table` 再套一层 `div.sm:col-span-2`（会把整页撑出手机宽度，见
              // `atu_synth.rs` 的同一条注释）：跨列写进 `wrapper_class`。
              <Table
                aria_label=Signal::derive(move || t("tools.ground-wave-table"))
                class="min-w-[360px]"
                wrapper_class="border sm:col-span-2"
                density=TableDensity::Compact
              >
                <TableHead>
                  <TableRow>
                    <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.distance-km")}</TableHeader>
                    <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.ground-wave-col-field")}</TableHeader>
                    <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.ground-wave-col-loss")}</TableHeader>
                    <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.ground-wave-col-method")}</TableHeader>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {rows
                    .into_iter()
                    .map(|(d, r)| {
                      view! {
                        <TableRow>
                          <TableCell class=crate::kit::table_cell_class("font-mono tabular-nums")>{d.to_string()}</TableCell>
                          <TableCell class=crate::kit::table_cell_class("font-mono tabular-nums")>
                            {r.map_or_else(|| "—".to_string(), |r| fmt_db(r.field_dbu_vm))}
                          </TableCell>
                          <TableCell class=crate::kit::table_cell_class("font-mono tabular-nums")>
                            {r.map_or_else(|| "—".to_string(), |r| fmt_db(r.basic_loss_db))}
                          </TableCell>
                          <TableCell class=crate::kit::table_cell_class("")>
                            {move || r.map_or_else(|| "—".to_string(), |r| t(method_key(r.method)))}
                          </TableCell>
                        </TableRow>
                      }
                    })
                    .collect_view()}
                </TableBody>
              </Table>
            }
          })
      }}

      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.ground-wave-notes")}</p>
    </div>
  }
}

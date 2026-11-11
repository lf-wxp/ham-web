//! 台站链路预算：发射机 → 馈线 → 天线 → 路径 → 天线 → 馈线 → 接收机 → 噪声 → 所需 SNR。

use ham_web_core::feedline::FEEDLINE_SPECS;
use ham_web_core::station_link::{
  self, FAR_FIELD_WAVELENGTHS, LinkInput, Stage, cable_loss_db, format_power_w, format_reach_km,
};
use leptos::prelude::*;

use super::result_class;
use crate::i18n::{t, tf};
use crate::ui::{
  Field, NumberField, SelectOption, SimpleSelect, Table, TableBody, TableCell, TableDensity,
  TableHead, TableHeader, TableRow,
};
use crate::util::unique_id;

/// 环节的词条 key。
fn stage_key(stage: Stage) -> &'static str {
  match stage {
    Stage::TxLine => "tools.station-link-stage-tx-line",
    Stage::RxLine => "tools.station-link-stage-rx-line",
    Stage::Misc => "tools.station-link-stage-misc",
    Stage::RxNoiseFigure => "tools.station-link-stage-rx-nf",
    Stage::ExternalNoise => "tools.station-link-stage-ext",
  }
}

/// dB / dBm 读数：两位小数，符号照实显示（接收电平、噪声几乎总是负的）。
/// 余量的「够 / 不够」由调用方选 `verdict-ok` / `verdict-short` 词条表达，那里传的是非负的绝对值。
fn fmt_db(v: f64) -> String {
  format!("{v:.2}")
}

/// 文本输入解析：不是有限数就 `None`（整条链路「输入无效」，不拿默认值悄悄顶上）。
/// 取值范围（如馈线长度不能为负）由内核判，这里只管「是不是一个数」。
fn num(s: RwSignal<String>) -> Option<f64> {
  s.get().trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

#[component]
pub(super) fn StationLink() -> impl IntoView {
  let rg213 = FEEDLINE_SPECS
    .iter()
    .position(|s| s.name == "RG-213")
    .unwrap_or(0);

  let freq = RwSignal::new(String::from("145.5"));
  let dist = RwSignal::new(String::from("200"));
  let path_extra = RwSignal::new(String::from("0"));
  let power = RwSignal::new(String::from("5"));
  let tx_kind = RwSignal::new(rg213);
  let tx_len = RwSignal::new(String::from("10"));
  let tx_gain = RwSignal::new(String::from("3"));
  let rx_gain = RwSignal::new(String::from("3"));
  let rx_kind = RwSignal::new(rg213);
  let rx_len = RwSignal::new(String::from("10"));
  let misc = RwSignal::new(String::from("1"));
  let nf = RwSignal::new(String::from("2"));
  let ext = RwSignal::new(String::from("0"));
  let bw = RwSignal::new(String::from("12500"));
  let req = RwSignal::new(String::from("12"));

  let options = || -> Vec<SelectOption> {
    FEEDLINE_SPECS
      .iter()
      .enumerate()
      .map(|(i, s)| SelectOption::new(i.to_string(), s.name))
      .collect()
  };
  let tx_options = options();
  let rx_options = options();

  let freq_id = unique_id("slink-freq");
  let dist_id = unique_id("slink-dist");
  let extra_id = unique_id("slink-extra");
  let power_id = unique_id("slink-power");
  let tx_kind_id = unique_id("slink-tx-kind");
  let tx_len_id = unique_id("slink-tx-len");
  let tx_gain_id = unique_id("slink-tx-gain");
  let rx_gain_id = unique_id("slink-rx-gain");
  let rx_kind_id = unique_id("slink-rx-kind");
  let rx_len_id = unique_id("slink-rx-len");
  let misc_id = unique_id("slink-misc");
  let nf_id = unique_id("slink-nf");
  let ext_id = unique_id("slink-ext");
  let bw_id = unique_id("slink-bw");
  let req_id = unique_id("slink-req");

  // 输入 → (馈线损耗, 结果)。纯派生，不进 Effect。
  // 任何一栏不是有限数就整条链路「输入无效」，不拿默认值悄悄顶上；
  // 取值范围（负长度、非正频率……）由内核拒绝，同样落到「输入无效」。
  let outcome = move || {
    let f = num(freq)?;
    let (tx_line_db, rx_line_db) = (
      cable_loss_db(tx_kind.get(), num(tx_len)?, f)?,
      cable_loss_db(rx_kind.get(), num(rx_len)?, f)?,
    );
    let input = LinkInput {
      freq_mhz: f,
      dist_km: num(dist)?,
      path_extra_db: num(path_extra)?,
      tx_power_w: num(power)?,
      tx_line_db,
      tx_gain_dbi: num(tx_gain)?,
      rx_gain_dbi: num(rx_gain)?,
      rx_line_db,
      misc_loss_db: num(misc)?,
      rx_nf_db: num(nf)?,
      ext_noise_db: num(ext)?,
      bandwidth_hz: num(bw)?,
      required_snr_db: num(req)?,
    };
    station_link::evaluate(&input).map(|r| (tx_line_db, rx_line_db, r))
  };

  view! {
    <div class="grid gap-3 sm:grid-cols-2">
      <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
        <NumberField id=freq_id value=Signal::derive(move || freq.get()) on_change=Callback::new(move |v: String| freq.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.distance-km")) r#for=dist_id.clone()>
        <NumberField id=dist_id value=Signal::derive(move || dist.get()) on_change=Callback::new(move |v: String| dist.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.station-link-path-extra"))
        hint=Signal::derive(move || t("tools.station-link-path-extra-hint"))
        r#for=extra_id.clone()
      >
        <NumberField id=extra_id value=Signal::derive(move || path_extra.get()) on_change=Callback::new(move |v: String| path_extra.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.transmit-power-w")) r#for=power_id.clone()>
        <NumberField id=power_id value=Signal::derive(move || power.get()) on_change=Callback::new(move |v: String| power.set(v)) controls=false />
      </Field>

      <Field label=Signal::derive(move || t("tools.station-link-tx-cable")) r#for=tx_kind_id.clone()>
        <SimpleSelect
          id=tx_kind_id
          value=Signal::derive(move || tx_kind.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(n) = v.parse::<usize>() {
              tx_kind.set(n);
            }
          })
          options=tx_options
          aria_label=Signal::derive(move || t("tools.station-link-tx-cable"))
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.station-link-tx-length")) r#for=tx_len_id.clone()>
        <NumberField id=tx_len_id value=Signal::derive(move || tx_len.get()) on_change=Callback::new(move |v: String| tx_len.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.transmit-antenna-gain-dbi")) r#for=tx_gain_id.clone()>
        <NumberField id=tx_gain_id value=Signal::derive(move || tx_gain.get()) on_change=Callback::new(move |v: String| tx_gain.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.receive-antenna-gain-dbi")) r#for=rx_gain_id.clone()>
        <NumberField id=rx_gain_id value=Signal::derive(move || rx_gain.get()) on_change=Callback::new(move |v: String| rx_gain.set(v)) controls=false />
      </Field>

      <Field label=Signal::derive(move || t("tools.station-link-rx-cable")) r#for=rx_kind_id.clone()>
        <SimpleSelect
          id=rx_kind_id
          value=Signal::derive(move || rx_kind.get().to_string())
          on_change=Callback::new(move |v: String| {
            if let Ok(n) = v.parse::<usize>() {
              rx_kind.set(n);
            }
          })
          options=rx_options
          aria_label=Signal::derive(move || t("tools.station-link-rx-cable"))
        />
      </Field>
      <Field label=Signal::derive(move || t("tools.station-link-rx-length")) r#for=rx_len_id.clone()>
        <NumberField id=rx_len_id value=Signal::derive(move || rx_len.get()) on_change=Callback::new(move |v: String| rx_len.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.station-link-misc"))
        hint=Signal::derive(move || t("tools.station-link-misc-hint"))
        r#for=misc_id.clone()
      >
        <NumberField id=misc_id value=Signal::derive(move || misc.get()) on_change=Callback::new(move |v: String| misc.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.noise-figure-nf-db")) r#for=nf_id.clone()>
        <NumberField id=nf_id value=Signal::derive(move || nf.get()) on_change=Callback::new(move |v: String| nf.set(v)) controls=false />
      </Field>

      <Field
        label=Signal::derive(move || t("tools.station-link-ext-noise"))
        hint=Signal::derive(move || t("tools.station-link-ext-noise-hint"))
        r#for=ext_id.clone()
      >
        <NumberField id=ext_id value=Signal::derive(move || ext.get()) on_change=Callback::new(move |v: String| ext.set(v)) controls=false />
      </Field>
      <Field label=Signal::derive(move || t("tools.station-link-bandwidth")) r#for=bw_id.clone()>
        <NumberField id=bw_id value=Signal::derive(move || bw.get()) on_change=Callback::new(move |v: String| bw.set(v)) controls=false />
      </Field>
      <Field
        label=Signal::derive(move || t("tools.station-link-required-snr"))
        hint=Signal::derive(move || t("tools.station-link-required-snr-hint"))
        r#for=req_id.clone()
      >
        <NumberField id=req_id value=Signal::derive(move || req.get()) on_change=Callback::new(move |v: String| req.set(v)) controls=false />
      </Field>

      {move || match outcome() {
        None => {
          view! {
            <div class=result_class()>
              <p>{t("tools.station-link-invalid")}</p>
            </div>
          }
            .into_any()
        }
        Some((tx_line_db, rx_line_db, r)) => {
          let verdict = if r.margin_db >= 0.0 {
            tf(
              "tools.station-link-verdict-ok",
              &[&fmt_db(r.margin_db), &fmt_db(r.snr_db), &fmt_db(r.snr_db - r.margin_db)],
            )
          } else {
            tf(
              "tools.station-link-verdict-short",
              &[&fmt_db(-r.margin_db), &fmt_db(r.snr_db), &fmt_db(r.snr_db - r.margin_db)],
            )
          };
          let levels = tf(
            "tools.station-link-levels",
            &[
              &fmt_db(r.eirp_dbm),
              &fmt_db(r.path_loss_db),
              &fmt_db(r.signal_dbm),
              &fmt_db(r.noise_dbm),
            ],
          );
          let share = match r.ext_share {
            Some(s) => tf(
              "tools.station-link-noise-share",
              &[&format!("{:.1}%", s * 100.0), &format!("{:.1}%", (1.0 - s) * 100.0)],
            ),
            None => t("tools.station-link-noise-thermal"),
          };
          let lines = tf(
            "tools.station-link-lines",
            &[&fmt_db(tx_line_db), &fmt_db(rx_line_db)],
          );
          let reach = tf(
            "tools.station-link-reach",
            &[&format_reach_km(r.max_dist_km), &format_power_w(r.required_tx_w)],
          );
          // 距离不到十个波长：自由空间公式失效，结果只能当量级参考。
          let near_field = r.near_field.then(|| {
            tf(
              "tools.station-link-near-field",
              &[&FAR_FIELD_WAVELENGTHS.to_string(), &format_reach_km(r.far_field_km)],
            )
          });
          // 第一名回收不到 0.005 dB（显示精度之下）就当「都接近理想」，不拿浮点噪声排名。
          let first = r.reclaim[0];
          let first_line = if first.gain_db < 0.005 {
            t("tools.station-link-first-none")
          } else {
            tf(
              "tools.station-link-first",
              &[&t(stage_key(first.stage)), &fmt_db(first.gain_db)],
            )
          };
          let rows = r.reclaim;
          view! {
            <div class=format!("{} space-y-1 tabular-nums", result_class())>
              {near_field.map(|text| view! { <p role="alert">{text}</p> })}
              <p>{verdict}</p>
              <p>{levels}</p>
              <p>{share}</p>
              <p>{lines}</p>
              <p>{reach}</p>
              <p>{first_line}</p>
            </div>
            // 不要给 `Table` 再套一层 `div.sm:col-span-2`（会把整页撑出手机宽度，见
            // `atu_synth.rs` 的同一条注释）：跨列写进 `wrapper_class`。
            <Table
              aria_label=Signal::derive(move || t("tools.station-link-table"))
              class="min-w-[320px]"
              wrapper_class="border sm:col-span-2"
              density=TableDensity::Compact
            >
              <TableHead>
                <TableRow>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.station-link-col-stage")}</TableHeader>
                  <TableHeader class=crate::kit::table_cell_class("")>{move || t("tools.station-link-col-gain")}</TableHeader>
                </TableRow>
              </TableHead>
              <TableBody>
                {rows
                  .into_iter()
                  .map(|x| {
                    view! {
                      <TableRow>
                        <TableCell class=crate::kit::table_cell_class("")>{move || t(stage_key(x.stage))}</TableCell>
                        <TableCell class=crate::kit::table_cell_class("font-mono tabular-nums")>
                          {fmt_db(x.gain_db)}
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

      <p class="sm:col-span-2 text-xs text-muted-foreground">{move || t("tools.station-link-notes")}</p>
    </div>
  }
}

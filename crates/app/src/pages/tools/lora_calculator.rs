//! LoRa 链路计算器：空中时间、接收灵敏度与链路预算 / 最大距离（Semtech AN1200.13 口径）。
//!
//! 内核在 `ham_web_core::lora`；页面只做输入与展示。数值存 `Option<f64>`，空输入 = 未填写，
//! 交给内核校验；输入不完整时结果区给提示，不用旧值算出与界面不符的数字。

use ham_web_core::lora::{
  BW_HZ_CHOICES, CR_RANGE, PL_RANGE, PREAMBLE_RANGE, PacketParams, SF_RANGE, coding_rate,
  link_margin_db, low_data_rate_optimize_auto, max_distance_km, payload_symbols, preamble_time_s,
  sensitivity_dbm, symbol_time_s, time_on_air_s, whole_u16,
};
use leptos::prelude::*;

use super::{RESULT, fmt_num};
use crate::i18n::{t, tf};
use crate::ui::{Field, NativeSelect, NumberField, SelectOption};
use crate::util::unique_id;

/// LoRa 链路计算器。
#[component]
pub(super) fn LoraCalculator() -> impl IntoView {
  let sf = RwSignal::new("7".to_owned());
  // 带宽以 Hz 存（62.5 kHz 不是整数 kHz）。
  let bw = RwSignal::new("125000".to_owned());
  let cr = RwSignal::new("1".to_owned());
  let payload = RwSignal::new(Some(12.0));
  let preamble = RwSignal::new(Some(8.0));
  let header = RwSignal::new("explicit".to_owned());
  let crc = RwSignal::new("on".to_owned());
  let ldro = RwSignal::new("auto".to_owned());
  // 链路预算（灵敏度共用 SF / BW）。
  let nf = RwSignal::new(Some(6.0));
  let freq = RwSignal::new(Some(470.0));
  let tx_dbm = RwSignal::new(Some(14.0));
  let tx_gain = RwSignal::new(Some(3.0));
  let rx_gain = RwSignal::new(Some(3.0));
  let extra = RwSignal::new(Some(0.0));
  let dist = RwSignal::new(Some(500.0));

  let sf_id = unique_id("lora-sf");
  let bw_id = unique_id("lora-bw");
  let cr_id = unique_id("lora-cr");
  let payload_id = unique_id("lora-payload");
  let preamble_id = unique_id("lora-preamble");
  let header_id = unique_id("lora-header");
  let crc_id = unique_id("lora-crc");
  let ldro_id = unique_id("lora-ldro");
  let nf_id = unique_id("lora-nf");
  let freq_id = unique_id("lora-freq");
  let tx_id = unique_id("lora-tx-dbm");
  let tx_gain_id = unique_id("lora-tx-gain");
  let rx_gain_id = unique_id("lora-rx-gain");
  let extra_id = unique_id("lora-extra");
  let dist_id = unique_id("lora-dist");

  // 选项全部取自内核常量，页面与内核的合法范围不会各写一份。
  let sf_options: Vec<SelectOption> = SF_RANGE
    .map(|s| SelectOption::new(s.to_string(), format!("SF{s}")))
    .collect();
  let bw_options: Vec<SelectOption> = BW_HZ_CHOICES
    .iter()
    .map(|&hz| SelectOption::new(hz.to_string(), format!("{} kHz", f64::from(hz) / 1000.0)))
    .collect();
  let cr_options: Vec<SelectOption> = CR_RANGE
    .filter_map(|r| {
      coding_rate(r).map(|(n, d)| SelectOption::new(r.to_string(), format!("{n}/{d}")))
    })
    .collect();
  let header_options = vec![
    SelectOption::new(
      "explicit",
      Signal::derive(move || t("tools.lora-header-explicit")),
    ),
    SelectOption::new(
      "implicit",
      Signal::derive(move || t("tools.lora-header-implicit")),
    ),
  ];
  let crc_options = vec![
    SelectOption::new("on", Signal::derive(move || t("tools.lora-crc-on"))),
    SelectOption::new("off", Signal::derive(move || t("tools.lora-crc-off"))),
  ];
  let ldro_options = vec![
    SelectOption::new("auto", Signal::derive(move || t("tools.lora-ldro-auto"))),
    SelectOption::new("on", Signal::derive(move || t("tools.lora-ldro-on"))),
    SelectOption::new("off", Signal::derive(move || t("tools.lora-ldro-off"))),
  ];

  let sf_u8 = move || sf.get().parse::<u8>().ok().filter(|s| SF_RANGE.contains(s));
  let bw_hz = move || bw.get().parse::<f64>().ok();
  let cr_u8 = move || cr.get().parse::<u8>().ok();

  // 非法输入给控件本身标红（`aria-invalid`），不只是结果区一句文字。
  let payload_invalid = Signal::derive(move || {
    payload
      .get()
      .and_then(whole_u16)
      .is_none_or(|v| !PL_RANGE.contains(&v))
  });
  let preamble_invalid = Signal::derive(move || {
    preamble
      .get()
      .and_then(whole_u16)
      .is_none_or(|v| !PREAMBLE_RANGE.contains(&v))
  });
  let not_finite =
    move |s: RwSignal<Option<f64>>| Signal::derive(move || s.get().is_none_or(|v| !v.is_finite()));
  let positive_invalid = move |s: RwSignal<Option<f64>>| {
    Signal::derive(move || s.get().is_none_or(|v| !(v > 0.0 && v.is_finite())))
  };
  let nf_invalid = not_finite(nf);
  let tx_invalid = not_finite(tx_dbm);
  let tx_gain_invalid = not_finite(tx_gain);
  let rx_gain_invalid = not_finite(rx_gain);
  let extra_invalid = not_finite(extra);
  let freq_invalid = positive_invalid(freq);
  let dist_invalid = positive_invalid(dist);

  let toa = Memo::new(move |_| {
    let sf = sf_u8()?;
    let bw = bw_hz()?;
    let params = PacketParams {
      sf,
      bw_hz: bw,
      // 载荷字节数与前导码符号数都是整数：`12.5` 不能悄悄四舍五入去算。
      npreamble: whole_u16(preamble.get()?)?,
      payload_bytes: whole_u16(payload.get()?)?,
      rate: cr_u8()?,
      crc_enabled: crc.get() == "on",
      implicit_header: header.get() == "implicit",
      low_data_rate_opt: match ldro.get().as_str() {
        "on" => true,
        // 自动档看符号时间 > 16 ms，不是单看 SF（同一 SF 换带宽结论会变）。
        "auto" => low_data_rate_optimize_auto(sf, bw)?,
        _ => false,
      },
    };
    let tsym = symbol_time_s(sf, bw)?;
    let pre = preamble_time_s(params.npreamble, tsym)?;
    let psym = payload_symbols(
      sf,
      params.payload_bytes,
      params.rate,
      params.crc_enabled,
      params.implicit_header,
      params.low_data_rate_opt,
    )?;
    let toa_s = time_on_air_s(&params)?;
    Some((
      tsym,
      pre,
      psym,
      toa_s,
      f64::from(params.payload_bytes) * 8.0 / toa_s,
    ))
  });

  let link = Memo::new(move |_| {
    let s = sensitivity_dbm(sf_u8()?, bw_hz()?, nf.get()?)?;
    let (f, tx, tg, rg, x, d) = (
      freq.get()?,
      tx_dbm.get()?,
      tx_gain.get()?,
      rx_gain.get()?,
      extra.get()?,
      dist.get()?,
    );
    let rx = ham_web_core::link_budget::received_dbm(tx, tg, rg, f, d, x)?;
    let margin = link_margin_db(tx, tg, rg, f, d, x, s)?;
    let maxd = max_distance_km(tx, tg, rg, f, x, s)?;
    Some((s, rx, margin, maxd))
  });

  view! {
    <div class="space-y-4">
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 [&>*]:min-w-0">
        <Field label=Signal::derive(move || t("tools.lora-sf")) r#for=sf_id.clone()>
          <NativeSelect id=sf_id value=sf on_change=Callback::new(move |v: String| sf.set(v)) options=sf_options aria_label=Signal::derive(move || t("tools.lora-sf")) />
        </Field>
        <Field label=Signal::derive(move || t("tools.lora-bw-khz")) r#for=bw_id.clone()>
          <NativeSelect id=bw_id value=bw on_change=Callback::new(move |v: String| bw.set(v)) options=bw_options aria_label=Signal::derive(move || t("tools.lora-bw-khz")) />
        </Field>
        <Field label=Signal::derive(move || t("tools.lora-cr")) r#for=cr_id.clone()>
          <NativeSelect id=cr_id value=cr on_change=Callback::new(move |v: String| cr.set(v)) options=cr_options aria_label=Signal::derive(move || t("tools.lora-cr")) />
        </Field>
        <Field label=Signal::derive(move || t("tools.lora-payload-bytes")) r#for=payload_id.clone()>
          <NumberField id=payload_id value=Signal::derive(move || payload.get().map_or_else(String::new, |v| v.to_string())) on_change=Callback::new(move |v: String| payload.set(v.trim().parse::<f64>().ok())) controls=false invalid=payload_invalid />
        </Field>
        <Field label=Signal::derive(move || t("tools.lora-preamble")) r#for=preamble_id.clone()>
          <NumberField id=preamble_id value=Signal::derive(move || preamble.get().map_or_else(String::new, |v| v.to_string())) on_change=Callback::new(move |v: String| preamble.set(v.trim().parse::<f64>().ok())) controls=false invalid=preamble_invalid />
        </Field>
        <Field label=Signal::derive(move || t("tools.lora-header")) r#for=header_id.clone()>
          <NativeSelect id=header_id value=header on_change=Callback::new(move |v: String| header.set(v)) options=header_options aria_label=Signal::derive(move || t("tools.lora-header")) />
        </Field>
        <Field label=Signal::derive(move || t("tools.lora-crc")) r#for=crc_id.clone()>
          <NativeSelect id=crc_id value=crc on_change=Callback::new(move |v: String| crc.set(v)) options=crc_options aria_label=Signal::derive(move || t("tools.lora-crc")) />
        </Field>
        <Field label=Signal::derive(move || t("tools.lora-ldro")) r#for=ldro_id.clone()>
          <NativeSelect id=ldro_id value=ldro on_change=Callback::new(move |v: String| ldro.set(v)) options=ldro_options aria_label=Signal::derive(move || t("tools.lora-ldro")) />
        </Field>
      </div>

      <div class=RESULT role="status">
        {move || match toa.get() {
          Some((tsym, pre, psym, toa_s, bps)) => {
            tf(
              "tools.lora-toa-result",
              &[
                &fmt_msec(tsym),
                &fmt_msec(pre),
                &psym.to_string(),
                &fmt_msec(toa_s),
                &fmt_num(bps),
              ],
            )
          }
          None => t("tools.lora-invalid-params"),
        }}
      </div>

      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 [&>*]:min-w-0">
        <Field label=Signal::derive(move || t("tools.noise-figure-nf-db")) r#for=nf_id.clone()>
          <NumberField id=nf_id value=Signal::derive(move || nf.get().map_or_else(String::new, |v| v.to_string())) on_change=Callback::new(move |v: String| nf.set(v.trim().parse::<f64>().ok())) controls=false invalid=nf_invalid />
        </Field>
        <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
          <NumberField id=freq_id value=Signal::derive(move || freq.get().map_or_else(String::new, |v| v.to_string())) on_change=Callback::new(move |v: String| freq.set(v.trim().parse::<f64>().ok())) controls=false invalid=freq_invalid />
        </Field>
        <Field label=Signal::derive(move || t("tools.lora-tx-dbm")) r#for=tx_id.clone()>
          <NumberField id=tx_id value=Signal::derive(move || tx_dbm.get().map_or_else(String::new, |v| v.to_string())) on_change=Callback::new(move |v: String| tx_dbm.set(v.trim().parse::<f64>().ok())) controls=false invalid=tx_invalid />
        </Field>
        <Field label=Signal::derive(move || t("tools.transmit-antenna-gain-dbi")) r#for=tx_gain_id.clone()>
          <NumberField id=tx_gain_id value=Signal::derive(move || tx_gain.get().map_or_else(String::new, |v| v.to_string())) on_change=Callback::new(move |v: String| tx_gain.set(v.trim().parse::<f64>().ok())) controls=false invalid=tx_gain_invalid />
        </Field>
        <Field label=Signal::derive(move || t("tools.receive-antenna-gain-dbi")) r#for=rx_gain_id.clone()>
          <NumberField id=rx_gain_id value=Signal::derive(move || rx_gain.get().map_or_else(String::new, |v| v.to_string())) on_change=Callback::new(move |v: String| rx_gain.set(v.trim().parse::<f64>().ok())) controls=false invalid=rx_gain_invalid />
        </Field>
        <Field label=Signal::derive(move || t("tools.lora-extra-loss-db")) r#for=extra_id.clone()>
          <NumberField id=extra_id value=Signal::derive(move || extra.get().map_or_else(String::new, |v| v.to_string())) on_change=Callback::new(move |v: String| extra.set(v.trim().parse::<f64>().ok())) controls=false invalid=extra_invalid />
        </Field>
        <Field label=Signal::derive(move || t("tools.distance-km")) r#for=dist_id.clone()>
          <NumberField id=dist_id value=Signal::derive(move || dist.get().map_or_else(String::new, |v| v.to_string())) on_change=Callback::new(move |v: String| dist.set(v.trim().parse::<f64>().ok())) controls=false invalid=dist_invalid />
        </Field>
      </div>

      <div class=RESULT role="status">
        {move || match link.get() {
          Some((s, rx, margin, maxd)) => {
            tf(
              "tools.lora-link-result",
              &[&fmt_num(s), &fmt_num(rx), &fmt_num(margin), &fmt_num(maxd)],
            )
          }
          None => t("tools.lora-invalid-params"),
        }}
      </div>

      <p class="text-xs text-muted-foreground">{move || t("tools.lora-note")}</p>
    </div>
  }
}

/// 把秒格式化成毫秒或微秒的可读文本：≥ 1 ms 用 ms，否则用 μs。
fn fmt_msec(secs: f64) -> String {
  let ms = secs * 1e3;
  if ms >= 1.0 {
    format!("{ms:.3} ms")
  } else {
    format!("{:.0} μs", ms * 1e3)
  }
}

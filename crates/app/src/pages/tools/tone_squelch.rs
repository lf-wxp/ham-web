use ham_web_core::tone_squelch::{CTCSS_TONES, DCS_CODES, REPEATER_OFFSETS};
use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, NumberField, RadioGroup, RadioGroupItem};
use crate::util::unique_id;

const CHIP: &str =
  "rounded-full border bg-card px-2.5 py-1 text-xs tabular-nums text-muted-foreground";

/// 中继频差 + CTCSS / DCS 亚音码表。
#[component]
pub(super) fn ToneSquelch() -> impl IntoView {
  let band = RwSignal::new("2 m".to_string());
  let rx = RwSignal::new(145.0);
  let positive = RwSignal::new(false);

  let rx_id = unique_id("tone-squelch-rx");

  let offset = move || {
    REPEATER_OFFSETS
      .iter()
      .find(|(name, _, _, _)| *name == band.get())
      .map(|(_, _, o, note)| (*o, *note))
  };

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-2">
        <RadioGroup
          value=band
          on_change=Callback::new(move |v: String| band.set(v))
          class="sm:col-span-2 flex flex-wrap items-center gap-4"
        >
          {REPEATER_OFFSETS
            .iter()
            .map(|(name, _, _, _)| {
              let band_name = *name;
              view! {
                <label class="flex cursor-pointer items-center gap-2 text-sm">
                  <RadioGroupItem value=band_name.to_string() />
                  {band_name}
                </label>
              }
            })
            .collect_view()}
        </RadioGroup>
        <Field label=Signal::derive(move || t("tools.receive-frequency-mhz")) r#for=rx_id.clone()>
          <NumberField
            id=rx_id
            value=Signal::derive(move || rx.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                rx.set(v);
              }
            })
            controls=false
          />
        </Field>
        <RadioGroup
          value=Signal::derive(move || {
            if positive.get() { "positive" } else { "negative" }.to_string()
          })
          on_change=Callback::new(move |v: String| positive.set(v == "positive"))
          class="flex flex-wrap items-end gap-4 pb-2"
        >
          <label class="flex cursor-pointer items-center gap-2 text-sm">
            <RadioGroupItem value="negative" />
            {move || t("tools.negative-offset")}
          </label>
          <label class="flex cursor-pointer items-center gap-2 text-sm">
            <RadioGroupItem value="positive" />
            {move || t("tools.positive-offset")}
          </label>
        </RadioGroup>
        <div class="sm:col-span-2 space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
          {move || {
            match offset() {
              Some((o, note)) => {
                let tx = if positive.get() { rx.get() + o } else { rx.get() - o };
                view! {
                  <div class="tabular-nums">
                    {tf("tools.offset-mhz", &[&fmt_num(o), note])}
                  </div>
                  <div class="tabular-nums">
                    {tf("tools.receive-mhz-transmit-mhz", &[&fmt_num(rx.get()), &fmt_num(tx)])}
                  </div>
                }
                .into_any()
              }
              None => view! { <span>{move || t("tools.please-select-a-band")}</span> }.into_any(),
            }
          }}
        </div>
      </div>

      <div>
        <div class="mb-1.5 text-xs text-muted-foreground">{move || t("tools.ctcss-tone-hz-50")}</div>
        <div class="flex flex-wrap gap-1.5">
          {CTCSS_TONES
            .iter()
            .map(|f| {
              view! { <span class=CHIP>{format!("{f:.1}")}</span> }
            })
            .collect_view()}
        </div>
      </div>

      <div>
        <div class="mb-1.5 text-xs text-muted-foreground">{move || t("tools.dcs-digital-squelch-code")}</div>
        <div class="flex flex-wrap gap-1.5">
          {DCS_CODES
            .iter()
            .map(|c| {
              view! { <span class=CHIP>{format!("{c:03o}")}</span> }
            })
            .collect_view()}
        </div>
      </div>
    </div>
  }
}

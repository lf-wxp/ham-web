use ham_web_core::aprs_codec::{decode_position, encode_position};
use leptos::prelude::*;

use super::fmt_num;
use crate::i18n::{t, tf};
use crate::ui::{Field, Input, NumberField};
use crate::util::unique_id;

/// APRS 未压缩位置报文编解码。
#[component]
pub(super) fn AprsCodec() -> impl IntoView {
  let lat = RwSignal::new(31.2417);
  let lon = RwSignal::new(121.475);
  let pos = RwSignal::new("3114.50N/12128.50E".to_string());

  let lat_id = unique_id("aprs-lat");
  let lon_id = unique_id("aprs-lon");
  let pos_id = unique_id("aprs-pos");

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-2">
        <Field label=Signal::derive(move || t("tools.latitude-degrees-90")) r#for=lat_id.clone()>
          <NumberField
            id=lat_id
            value=Signal::derive(move || lat.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                lat.set(v);
              }
            })
            controls=false
          />
        </Field>
        <Field label=Signal::derive(move || t("tools.longitude-degrees-180")) r#for=lon_id.clone()>
          <NumberField
            id=lon_id
            value=Signal::derive(move || lon.get().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(v) = v.trim().parse::<f64>() {
                lon.set(v);
              }
            })
            controls=false
          />
        </Field>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          match encode_position(lat.get(), lon.get()) {
            Some(s) => tf("tools.aprs-position-field", &[&s]),
            None => t("tools.enter-a-valid-latitude").to_string(),
          }
        }}
      </div>

      <Field label=Signal::derive(move || t("tools.aprs-position-field-e")) r#for=pos_id.clone()>
        <Input id=pos_id value=pos on_change=Callback::new(move |v: String| pos.set(v)) />
      </Field>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          match decode_position(&pos.get()) {
            Some((la, lo)) => tf("tools.decoded-latitude-longitude", &[&fmt_num(la), &fmt_num(lo)]),
            None => t("tools.cannot-parse-it-please").to_string(),
          }
        }}
      </div>
      <p class="text-xs text-muted-foreground">
        {move || t("tools.uncompressed-format-latitude-ddmm")}
      </p>
    </div>
  }
}

use ham_web_core::aprs_codec::{decode_position, encode_position};
use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

/// APRS 未压缩位置报文编解码。
#[component]
pub(super) fn AprsCodec() -> impl IntoView {
  let lat = RwSignal::new(31.2417);
  let lon = RwSignal::new(121.475);
  let pos = RwSignal::new("3114.50N/12128.50E".to_string());

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-2">
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("纬度（度，±90）")}</span>
          <input
            type="number"
            prop:value=move || lat.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                lat.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("经度（度，±180）")}</span>
          <input
            type="number"
            prop:value=move || lon.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                lon.set(v);
              }
            }
            class=INPUT
          />
        </label>
      </div>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          match encode_position(lat.get(), lon.get()) {
            Some(s) => tf("APRS 位置字段：{}", &[&s]),
            None => t("请输入有效经纬度。").to_string(),
          }
        }}
      </div>

      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("APRS 位置字段（如 3114.50N/12128.50E）")}</span>
        <input
          type="text"
          prop:value=move || pos.get()
          on:input=move |e| pos.set(event_target_value(&e))
          class=INPUT
        />
      </label>
      <div class="rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground tabular-nums">
        {move || {
          match decode_position(&pos.get()) {
            Some((la, lo)) => tf("解码：纬度 {}°，经度 {}°", &[&fmt_num(la), &fmt_num(lo)]),
            None => t("无法解析，请检查格式。").to_string(),
          }
        }}
      </div>
      <p class="text-xs text-muted-foreground">
        {move || t("未压缩位置格式：纬度 ddmm.mm + N/S，经度 dddmm.mm + E/W。本工具不涉及 AX.25 帧与 Mic-E 压缩。")}
      </p>
    </div>
  }
}

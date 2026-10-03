use ham_web_core::tone_squelch::{CTCSS_TONES, DCS_CODES, REPEATER_OFFSETS};
use leptos::prelude::*;

use super::{INPUT, fmt_num};
use crate::i18n::{t, tf};

const CHIP: &str =
  "rounded-full border bg-card px-2.5 py-1 text-xs tabular-nums text-muted-foreground";

/// 中继频差 + CTCSS / DCS 亚音码表。
#[component]
pub(super) fn ToneSquelch() -> impl IntoView {
  let band = RwSignal::new("2 m".to_string());
  let rx = RwSignal::new(145.0);
  let positive = RwSignal::new(false);

  let offset = move || {
    REPEATER_OFFSETS
      .iter()
      .find(|(name, _, _, _)| *name == band.get())
      .map(|(_, _, o, note)| (*o, *note))
  };

  view! {
    <div class="space-y-4">
      <div class="grid gap-3 sm:grid-cols-2">
        <div class="sm:col-span-2 flex flex-wrap gap-1.5">
          {REPEATER_OFFSETS
            .iter()
            .map(|(name, _, _, _)| {
              view! {
                <button
                  type="button"
                  class=move || {
                    if band.get() == *name {
                      "rounded-full border border-primary bg-primary/10 px-2.5 py-1 text-xs font-medium text-primary"
                    } else {
                      "rounded-full border bg-card px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                    }
                  }
                  on:click=move |_| band.set((*name).to_string())
                >
                  {*name}
                </button>
              }
            })
            .collect_view()}
        </div>
        <label class="flex flex-col gap-1.5 text-sm">
          <span class="text-xs text-muted-foreground">{move || t("接收频率（MHz）")}</span>
          <input
            type="number"
            prop:value=move || rx.get().to_string()
            on:input=move |e| {
              if let Ok(v) = event_target_value(&e).parse::<f64>() {
                rx.set(v);
              }
            }
            class=INPUT
          />
        </label>
        <label class="flex items-end gap-2 text-sm">
          <button
            type="button"
            class=move || {
              if positive.get() {
                "rounded-lg border border-primary bg-primary/10 px-3 py-2 text-xs font-medium text-primary"
              } else {
                "rounded-lg border bg-card px-3 py-2 text-xs text-muted-foreground transition-colors hover:bg-accent"
              }
            }
            on:click=move |_| positive.set(false)
          >
            {move || t("负偏移 −")}
          </button>
          <button
            type="button"
            class=move || {
              if positive.get() {
                "rounded-lg border bg-card px-3 py-2 text-xs text-muted-foreground transition-colors hover:bg-accent"
              } else {
                "rounded-lg border border-primary bg-primary/10 px-3 py-2 text-xs font-medium text-primary"
              }
            }
            on:click=move |_| positive.set(true)
          >
            {move || t("正偏移 +")}
          </button>
        </label>
        <div class="sm:col-span-2 space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
          {move || {
            match offset() {
              Some((o, note)) => {
                let tx = if positive.get() { rx.get() + o } else { rx.get() - o };
                view! {
                  <div class="tabular-nums">
                    {tf("频差 {} MHz（{}）", &[&fmt_num(o), note])}
                  </div>
                  <div class="tabular-nums">
                    {tf("接收 {} MHz → 发射 {} MHz", &[&fmt_num(rx.get()), &fmt_num(tx)])}
                  </div>
                }
                .into_any()
              }
              None => view! { <span>{move || t("请选择波段。")}</span> }.into_any(),
            }
          }}
        </div>
      </div>

      <div>
        <div class="mb-1.5 text-xs text-muted-foreground">{move || t("CTCSS 亚音（Hz，共 50 组）")}</div>
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
        <div class="mb-1.5 text-xs text-muted-foreground">{move || t("DCS 数字静噪码（八进制）")}</div>
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

//! 反馈与读数：分格进度条、指示灯、七段读数、统计格。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::ui::{Chip, ChipGroup, Led, LedState, LedTone, Progress, Readout, Slider, Stat};

const TONES: &[(LedTone, &str)] = &[
  (LedTone::Rx, "led-rx"),
  (LedTone::Tx, "led-tx"),
  (LedTone::Warn, "led-warn"),
  (LedTone::Info, "led-info"),
];

#[component]
pub fn FeedbackGallery() -> impl IntoView {
  let amount = RwSignal::new(68_i64);
  let lit = RwSignal::new(String::from("on"));
  let led_state = Signal::derive(move || LedState::lit(lit.get() == "on"));
  let progress = Signal::derive(move || amount.get() as f64);
  let readout = Signal::derive(move || format!("{:02}", amount.get()));

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-feedback"))>
      <div class="grid gap-4 p-4 md:grid-cols-2">
        <div class=crate::kit::tile_class("space-y-3 p-3")>
          <div class="flex items-center justify-between gap-2">
            <code class="text-xs text-muted-foreground">"Progress / Slider"</code>
            <span class=crate::kit::stat_text_class("text-xs")>
              {move || format!("{}%", amount.get())}
            </span>
          </div>
          <Progress
            value=amount
            label=Signal::derive(move || t("design.comp-progress"))
          />
          <Progress
            value=amount
            label=Signal::derive(move || t("design.comp-progress"))
            class="pxl-bar-hp"
          />
          <Progress
            value=amount
            label=Signal::derive(move || t("design.comp-progress"))
            class="pxl-bar-gold"
          />
          <Slider
            value=progress
            on_change=Callback::new(move |v: f64| amount.set(v.round().clamp(0.0, 100.0) as i64))
            aria_label=Signal::derive(move || t("design.comp-progress"))
            aria_valuetext=Signal::derive(move || format!("{}%", amount.get()))
          />
        </div>
        <div class=crate::kit::tile_class("space-y-3 p-3")>
          <div class="flex flex-wrap items-center justify-between gap-2">
            <code class="text-xs text-muted-foreground">"Led"</code>
            <ChipGroup
              value=lit
              on_change=Callback::new(move |v: String| lit.set(v))
              aria_label=Signal::derive(move || t("design.comp-led"))
            >
              <Chip value="on">"ON"</Chip>
              <Chip value="off">"OFF"</Chip>
            </ChipGroup>
          </div>
          <div class="flex flex-wrap gap-3">
            {TONES
              .iter()
              .map(|(tone, name)| {
                view! {
                  <span class="flex items-center gap-2 text-xs text-muted-foreground">
                    <Led tone=*tone state=led_state />
                    {*name}
                  </span>
                }
              })
              .collect_view()}
          </div>
          <div class="flex flex-wrap items-center gap-4 pt-2">
            <div class="pxl-display flex flex-col gap-1 p-2">
              <Readout
                text=readout
                label=Signal::derive(move || t("design.comp-readout"))
              />
            </div>
            <Stat
              label=Signal::derive(move || t("design.comp-stat"))
              value=Signal::derive(move || usize::try_from(amount.get()).unwrap_or(0))
            />
          </div>
        </div>
      </div>
    </SectionCard>
  }
}

//! 反馈与读数：3 条进度、1 个驱动滑杆、4 档指示灯、读数与统计格。
//!
//! 像素风的 `Led` / `Readout` 只有 `pxl-*` 实现，现代风用已登记的
//! `md-led` / `md-readout` 展示类补上，项目数量与交互保持一致。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::ui::{Chip, ChipGroup, Progress, Slider, Stat};

use super::data::MD_LED_TONES;

#[component]
pub fn ModernFeedback() -> impl IntoView {
  let amount = RwSignal::new(68_i64);
  let progress = Signal::derive(move || amount.get() as f64);
  let lit = RwSignal::new(String::from("on"));
  let readout = Signal::derive(move || format!("{:02}", amount.get()));

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-feedback"))>
      <div class="grid gap-4 p-4 md:grid-cols-2">
        <div class=crate::kit::tile_class("space-y-3 p-3")>
          <div class="flex items-center justify-between gap-2">
            <code class="md-muted md-sm">"Progress / Slider"</code>
            <span class=crate::kit::stat_text_class("md-sm")>
              {move || format!("{}%", amount.get())}
            </span>
          </div>
          <Progress value=amount label=Signal::derive(move || t("design.comp-progress")) />
          <Progress
            value=Signal::derive(move || amount.get() / 2)
            label=Signal::derive(move || t("design.comp-progress"))
          />
          <Progress
            value=Signal::derive(move || 100 - amount.get())
            label=Signal::derive(move || t("design.comp-progress"))
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
            <code class="md-muted md-sm">"md-led"</code>
            <ChipGroup
              value=lit
              on_change=Callback::new(move |v: String| lit.set(v))
              aria_label=Signal::derive(move || t("design.comp-led"))
            >
              <Chip value="on">"ON"</Chip>
              <Chip value="off">"OFF"</Chip>
            </ChipGroup>
          </div>
          <div data-spec-list="leds" class="flex flex-wrap gap-3">
            <For each=move || MD_LED_TONES key=|tone| *tone let:tone>
              <span class="flex items-center gap-2 md-sm">
                // 色相与 `data-lit` 都写在灯本体上：`01-base.css` 的 `.md-led[data-tone]` /
                // `.md-led[data-lit="off"]` 直接读这两个属性，不依赖某个父级类名。
                <span
                  class="md-led"
                  data-tone=*tone
                  data-lit=move || lit.get()
                  aria-hidden="true"
                ></span>
                <span>{*tone}</span>
              </span>
            </For>
          </div>
          <div class="flex flex-wrap items-center gap-4 pt-2">
            <div class="md-readout md-mono">{move || readout.get()}</div>
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

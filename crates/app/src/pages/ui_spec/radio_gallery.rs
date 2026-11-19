//! 电台面板组件：屏幕、七段读数、S 表、电平格、旋钮、信号格、电量格、刻度尺、干扰频谱。

use ham_web_core::rig_panel::{QrmKind, S_SEGMENTS, S_WEAK, S9};
use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::components::radio::{
  Battery, QrmSignature, SignalBars, SignalMeter, TuningScale, qrm_name,
};
use crate::i18n::t;
use crate::ui::{
  Chip, ChipGroup, Knob, Led, LedState, LedTone, Readout, ReadoutTone, Separator, Slider,
};

use super::spec_data::{READOUT_TONES, SCREEN_TONES, slider_u8};

const SPECTRUM_LIT: [u8; 10] = [3, 6, 9, 5, 11, 7, 12, 4, 8, 10];
const QRM_SLUGS: &[(&str, QrmKind)] = &[
  ("bursts", QrmKind::Bursts),
  ("carrier", QrmKind::Carrier),
  ("comb", QrmKind::Comb),
  ("static", QrmKind::Static),
];

fn qrm_from(slug: &str) -> QrmKind {
  QRM_SLUGS
    .iter()
    .find(|(name, _)| *name == slug)
    .map_or(QrmKind::Bursts, |(_, kind)| *kind)
}

fn tone_for(slug: &str) -> ReadoutTone {
  match slug {
    "dim" => ReadoutTone::Dim,
    "hot" => ReadoutTone::Hot,
    "alarm" => ReadoutTone::Alarm,
    "ok" => ReadoutTone::Ok,
    _ => ReadoutTone::Normal,
  }
}

#[component]
pub fn RadioGallery() -> impl IntoView {
  let freq = RwSignal::new(7100.0_f64);
  let units = RwSignal::new(7_u8);
  let peak = RwSignal::new(10_u8);
  let bars = RwSignal::new(3_u8);
  let charge = RwSignal::new(4_u32);
  let needle = RwSignal::new(50_u8);
  let cells = RwSignal::new(10_u8);
  let strength = RwSignal::new(0.8_f64);
  let qrm_slug = RwSignal::new(String::from("bursts"));
  let qrm_kind = Signal::derive(move || qrm_from(&qrm_slug.get()));
  let s_reading = Signal::derive(move || format!("S{}", units.get()));
  let marks = Signal::derive(|| {
    vec![
      ("40".to_owned(), 12_u8),
      ("20".to_owned(), 45_u8),
      ("10".to_owned(), 82_u8),
    ]
  });

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-radio"))>
      <div class="space-y-5 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.comp-radio-hint")}
        </p>

        <div class="grid gap-4 lg:grid-cols-2">
          <div class="pxl-display space-y-3 p-3">
            <div class="flex flex-wrap items-center justify-between gap-2">
              <span class="pxl-label pxl-d-dim text-xs">
                {move || t("design.comp-smeter")}
              </span>
              <Readout
                text=s_reading
                label=Signal::derive(move || t("design.comp-smeter"))
                tone=Signal::derive(move || ReadoutTone::for_s_units(Some(units.get())))
              />
            </div>
            <SignalMeter
              units=Signal::derive(move || Some(units.get()))
              peak=Signal::derive(move || Some(peak.get()))
              label=Signal::derive(move || t("design.comp-smeter"))
            />
            // 展示用的手写分格条：`crate::ui` 没有「可手动调格数」的封装，结构与首页 `base_hero.rs` 同源。
            <span
              class="pxl-meter-bar [--cells:16]"
              role="progressbar"
              aria-label=move || t("design.comp-level-cells")
              aria-valuemin="0"
              aria-valuemax="100"
              aria-valuenow=move || (u16::from(cells.get()) * 100 / 16).to_string()
            >
              {(0..16)
                .map(|i| {
                  view! {
                    <span
                      class="pxl-meter-seg"
                      data-lit=move || (i < cells.get()).then_some("")
                      data-plus=(i >= 12).then_some("")
                    ></span>
                  }
                })
                .collect_view()}
            </span>
            <div class="flex flex-wrap items-center gap-4 pt-2">
              <Knob
                value=freq
                on_change=Callback::new(move |v: f64| freq.set(v))
                min=7000.0
                max=7300.0
                step=10.0
                label=Signal::derive(move || t("design.comp-knob-label"))
                aria_label=Signal::derive(move || t("design.comp-knob-aria"))
                readout=Signal::derive(move || format!("{:.0}", freq.get()))
                unit="kHz"
                aria_valuetext=Signal::derive(move || format!("{} kHz", freq.get()))
              />
              <div class="space-y-2" data-pause-offscreen="">
                <span class="flex items-center gap-2">
                  <Led tone=LedTone::Tx state=Signal::derive(|| LedState::Blink) />
                  <span class="pxl-label pxl-d-alarm text-xs">"TX"</span>
                </span>
                <span class="flex items-center gap-2">
                  <SignalBars lit=bars total=5 />
                  <span class="pxl-label pxl-d-dim text-xs">
                    {move || format!("{}/5", bars.get())}
                  </span>
                </span>
                <span class="flex items-center gap-2">
                  <Battery
                    level=charge
                    max=5
                    label=Signal::derive(move || t("design.comp-battery"))
                  />
                  <span class="pxl-label pxl-d-dim text-xs">
                    {move || format!("{}/5", charge.get())}
                  </span>
                </span>
              </div>
            </div>
          </div>

          <div class="space-y-3">
            <div class=crate::kit::tile_class("space-y-3 p-3")>
              <code class="block text-xs text-muted-foreground">"TuningScale"</code>
              <TuningScale marks=marks needle=needle />
              <Slider
                value=Signal::derive(move || f64::from(needle.get()))
                on_change=Callback::new(move |v: f64| needle.set(slider_u8(v, 100)))
                aria_label=Signal::derive(move || t("design.comp-tuning"))
                aria_valuetext=Signal::derive(move || format!("{}%", needle.get()))
              />
            </div>
            <div class="pxl-display space-y-2 p-3">
              {move || {
                view! {
                  <QrmSignature
                    kind=qrm_kind.get()
                    strength=strength
                    cols=16
                    animate=true
                  />
                }
              }}
              <span class="pxl-label pxl-d-dim block truncate text-xs">
                {move || qrm_name(qrm_kind.get())}
              </span>
            </div>
            <ChipGroup
              value=qrm_slug
              on_change=Callback::new(move |v: String| qrm_slug.set(v))
              aria_label=Signal::derive(move || t("design.comp-qrm"))
            >
              {QRM_SLUGS
                .iter()
                .map(|(slug, _)| view! { <Chip value=*slug>{*slug}</Chip> })
                .collect_view()}
            </ChipGroup>
          </div>
        </div>

        <div class=crate::kit::tile_class("space-y-2 p-3")>
          <div class="flex flex-wrap items-center gap-3">
            <Slider
              value=Signal::derive(move || f64::from(units.get()))
              on_change=Callback::new(move |v: f64| units.set(slider_u8(v, 13)))
              min=0.0
              max=13.0
              aria_label=Signal::derive(move || t("design.comp-smeter"))
              aria_valuetext=Signal::derive(move || format!("S{}", units.get()))
              class="w-28"
            />
            <Slider
              value=Signal::derive(move || f64::from(bars.get()))
              on_change=Callback::new(move |v: f64| bars.set(slider_u8(v, 5)))
              min=0.0
              max=5.0
              aria_label=Signal::derive(move || t("design.comp-sigbars"))
              aria_valuetext=Signal::derive(move || format!("{}", bars.get()))
              class="w-28"
            />
            <Slider
              value=Signal::derive(move || f64::from(charge.get()))
              on_change=Callback::new(move |v: f64| charge.set(u32::from(slider_u8(v, 5))))
              min=0.0
              max=5.0
              aria_label=Signal::derive(move || t("design.comp-battery"))
              aria_valuetext=Signal::derive(move || format!("{}", charge.get()))
              class="w-28"
            />
            <Slider
              value=Signal::derive(move || f64::from(cells.get()))
              on_change=Callback::new(move |v: f64| cells.set(slider_u8(v, 16)))
              min=0.0
              max=16.0
              aria_label=Signal::derive(move || t("design.comp-level-cells"))
              aria_valuetext=Signal::derive(move || format!("{}", cells.get()))
              class="w-28"
            />
            <Slider
              value=strength
              on_change=Callback::new(move |v: f64| strength.set(v))
              min=0.0
              max=1.0
              step=0.1
              aria_label=Signal::derive(move || t("design.comp-qrm-strength"))
              aria_valuetext=Signal::derive(move || format!("{}", strength.get()))
              class="w-28"
            />
          </div>
          <p class="text-xs text-muted-foreground">{move || t("design.comp-radio-controls")}</p>
        </div>

        <Separator />

        <section class="space-y-2">
          <h3 class="pxl-sub-head">{move || t("design.comp-readout-tones")}</h3>
          <div class="grid grid-cols-2 gap-2 sm:grid-cols-5">
            {READOUT_TONES
              .iter()
              .map(|(slug, name)| {
                view! {
                  <div class="pxl-display flex flex-col items-center gap-1 p-2">
                    <Readout
                      text=Signal::derive(|| String::from("S9"))
                      label=Signal::derive(move || format!("S9 {}", name))
                      tone=Signal::derive(move || tone_for(slug))
                    />
                    <code class="pxl-d-dim text-xs">{*name}</code>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="space-y-2">
          <h3 class="pxl-sub-head">{move || t("design.comp-screen-tones")}</h3>
          <div class="pxl-display flex flex-wrap gap-4 p-3">
            {SCREEN_TONES
              .iter()
              .map(|(class, name)| {
                view! { <span class=*class>{*name}</span> }
              })
              .collect_view()}
          </div>
        </section>

        <section class="space-y-2">
          <h3 class="pxl-sub-head">{move || t("design.comp-spectrum")}</h3>
          <ul class="pxl-spectrum" aria-hidden="true">
            {SPECTRUM_LIT
              .iter()
              .enumerate()
              .map(|(index, lit)| {
                let weak = *lit < S_WEAK;
                view! {
                  <li>
                    <span
                      class="pxl-spectrum-bar"
                      data-weak=weak.then_some("")
                      data-active=move || (usize::from(needle.get()) / 10 == index).then_some("")
                    >
                      <span class="pxl-spectrum-cells">
                        {(0..S_SEGMENTS)
                          .map(|cell| {
                            view! {
                              <span
                                data-lit=(cell < *lit).then_some("")
                                data-plus=(cell >= S9).then_some("")
                              ></span>
                            }
                          })
                          .collect_view()}
                      </span>
                      <span class="pxl-spectrum-index">
                        {format!("{:02}", index + 1)}
                      </span>
                    </span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>

        <section class="space-y-2">
          <h3 class="pxl-sub-head">{move || t("design.comp-radio-samples")}</h3>
          <div class="flex flex-wrap items-center gap-3">
            <span class="pxl-onair">
              <Led tone=LedTone::Tx state=Signal::derive(|| LedState::On) small=true />
              "ON AIR"
            </span>
            <span class="pxl-qsl-stamp" data-pass="">"QSL PASS"</span>
            <span class="pxl-band-tag" aria-hidden="true">
              <Led tone=LedTone::Info state=Signal::derive(|| LedState::On) small=true />
              {move || t("design.comp-band-tag")}
            </span>
            <kbd class="pxl-kbd">"/"</kbd>
          </div>
          <div class="pxl-display w-64" aria-hidden="true">
            <div class="pxl-testcard h-18"></div>
            <svg class="pxl-flatline-svg" viewBox="0 0 240 20" preserveAspectRatio="none">
              <rect x="0" y="9" width="96" height="2" />
              <rect x="96" y="5" width="2" height="6" />
              <rect x="100" y="5" width="2" height="12" />
              <rect x="104" y="9" width="2" height="8" />
              <rect x="104" y="9" width="136" height="2" />
            </svg>
          </div>
          <div
            class="pxl-logstrip"
            role="img"
            aria-label=move || t("design.comp-logstrip")
          >
            {(0..28)
              .map(|day| {
                let level = (day * 7) % 5;
                view! {
                  <span
                    data-level=level.to_string()
                    data-today=(day == 27).then_some("")
                    style=format!("--mix: {}%", level * 20)
                  ></span>
                }
              })
              .collect_view()}
          </div>
        </section>
      </div>
    </SectionCard>
  }
}

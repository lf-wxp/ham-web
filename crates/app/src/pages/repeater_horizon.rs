//! 中继台的无线电地平线面板：在地图上画中继台所在点和一圈地平线。
//!
//! 半径与取点全在核心 `ham_web_core::repeater_horizon`（有单测）；天线高度数据源里没有，
//! 由用户填，**默认留空、不编数**。

use ham_web_core::repeater_horizon::{self, Horizon, HorizonError};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::pages::map::{MapView, project};
use crate::ui::{Button, Field, Input, Size, Variant, card_title_class};
use crate::util::unique_id;

/// 被选中的中继台：只带画图要用的几项。
#[derive(Clone, PartialEq)]
pub struct HorizonTarget {
  pub callsign: String,
  pub frequency_mhz: String,
  pub lat: f64,
  pub lon: f64,
}

/// 折线点串：不取整。`project` 的单位是半度，取到 0.1 就是 5 km 的误差，
/// 而这里画的圆半径只有几十公里。
fn ring_points(ring: &[(f64, f64)]) -> String {
  ring
    .iter()
    .map(|&(lat, lon)| {
      let (x, y) = project(lon, lat);
      format!("{x},{y}")
    })
    .collect::<Vec<_>>()
    .join(" ")
}

#[component]
pub fn RepeaterHorizon(selected: RwSignal<Option<HorizonTarget>>) -> impl IntoView {
  // 高度放在条件之外：换一台中继、关掉再选，用户填过的高度还在。
  let height = RwSignal::new(String::new());
  let height_id = unique_id("repeater-horizon-height");

  // 外层只跟「有没有选中」：直接读 `selected` 的话每换一台都把整块（含地图）重建。
  let open = Memo::new(move |_| selected.with(Option::is_some));
  let outcome = Memo::new(move |_| {
    selected.with(|s| {
      s.as_ref()
        .map(|t| repeater_horizon::horizon(t.lat, t.lon, &height.get()))
    })
  });
  let focus = Signal::derive(move || selected.with(|s| s.as_ref().map(|t| (t.lat, t.lon))));

  move || {
    open.get().then(|| {
      let label_for = height_id.clone();
      let input_id = height_id.clone();
      view! {
        <div class="space-y-3 border-t px-4 py-3" data-testid="repeater-horizon">
          <div class="flex flex-wrap items-center gap-3">
            <h3 class=card_title_class("min-w-0 flex-1")>
              {move || {
                selected
                  .get()
                  .map(|s| tf("radio.horizon-title", &[&s.callsign, &s.frequency_mhz]))
              }}
            </h3>
            <Button
              variant=Variant::Outline
              size=Size::Sm
              on_click=Callback::new(move |_| selected.set(None))
            >
              {move || t("log.close")}
            </Button>
          </div>

          <Field label=Signal::derive(move || t("radio.horizon-height")) r#for=label_for class="sm:max-w-xs">
            <Input
              id=input_id
              inputmode="decimal"
              placeholder=Signal::derive(move || t("radio.horizon-height-placeholder"))
              value=Signal::derive(move || height.get())
              on_change=Callback::new(move |v: String| height.set(v))
              invalid=Signal::derive(move || {
                matches!(outcome.get(), Some(Err(HorizonError::BadHeight)))
              })
            />
          </Field>

          <p class="text-xs text-muted-foreground" role="status" data-testid="horizon-status">
            {move || match outcome.get() {
              Some(Ok(h)) => tf("radio.horizon-radius", &[&format!("{:.1}", h.radius_km)]),
              Some(Err(HorizonError::BadHeight)) => t("radio.horizon-bad-height"),
              Some(Err(HorizonError::NoCoordinates)) => t("radio.horizon-no-coords"),
              Some(Err(HorizonError::NoHeight)) | None => t("radio.horizon-need-height"),
            }}
          </p>

          // 聚焦与放大上限都要抬高：地平线只有几十公里，缺省的 45° 宽视野里它不到一个像素。
          <MapView
            aria_label=Signal::derive(move || t("radio.horizon-map-aria"))
            focus=focus
            focus_zoom=120.0
            max_zoom=400.0
          >
            {move || {
              selected
                .get()
                .map(|s| {
                  let (x, y) = project(s.lon, s.lat);
                  let ring = outcome
                    .get()
                    .and_then(Result::ok)
                    .map(|h: Horizon| ring_points(&h.ring));
                  view! {
                    {ring.map(|points| view! {
                      <polygon
                        data-testid="horizon-ring"
                        points=points
                        class="fill-primary/10 stroke-primary"
                        stroke-width="2"
                        vector-effect="non-scaling-stroke"
                      />
                    })}
                    // 中心点：半径取极小、靠不随缩放的粗描边撑出一个固定大小的圆点。
                    <circle
                      data-testid="horizon-centre"
                      cx=x.to_string()
                      cy=y.to_string()
                      r="0.01"
                      class="stroke-destructive"
                      stroke-width="9"
                      stroke-linecap="round"
                      vector-effect="non-scaling-stroke"
                    >
                      <title>{format!("{} {} MHz", s.callsign, s.frequency_mhz)}</title>
                    </circle>
                  }
                })
            }}
          </MapView>

          <p class="text-xs text-muted-foreground">{move || t("radio.horizon-note")}</p>
        </div>
      }
    })
  }
}

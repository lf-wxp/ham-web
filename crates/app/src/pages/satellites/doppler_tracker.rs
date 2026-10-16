//! 多普勒修正：选一颗卫星，加载它的下一次过境，逐秒给出该调的接收 / 发射频率；
//! 可选地连接电台自动调谐（见 [`super::doppler_rig`]）。
//!
//! 过境列表来自 `/api/passes`，轨迹（方位、仰角、距离、径向速度）来自 `/api/sat-track`，
//! 加载完成后整条轨迹都在本地，之后逐秒插值不再联网，所以过境途中网络断了也能继续。
//! 位置沿用「过境预报」面板里保存的经纬度。多普勒公式与插值在 `ham_web_core::sat_doppler`。

use std::time::Duration;

use ham_web_core::sat_doppler::{
  DEFAULT_STEP_SECS, PRESETS, Sample, Track, next_pass, rx_frequency_hz, track_window,
  tx_frequency_hz,
};
use ham_web_core::sat_watch::Pass;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use super::doppler_rig::DopplerRig;
use crate::i18n::{t, tf};
use crate::sat_alert;
use crate::ui::{Button, Field, NativeSelect, NumberField, SelectOption, Size, Variant};
use crate::util::unique_id;

/// 下拉里「自定义」一项的值。
const CUSTOM: &str = "custom";

/// `/api/sat-track` 的返回（只取用到的字段）。
#[derive(Deserialize)]
struct TrackApi {
  samples: Vec<Sample>,
  #[serde(default)]
  stale: bool,
}

/// 已加载好的一次过境。
#[derive(Clone, PartialEq)]
struct Ready {
  pass: Pass,
  track: Track,
}

#[derive(Clone, PartialEq)]
enum Load {
  Idle,
  Loading,
  /// 过境列表取不到。
  PassesFailed,
  /// 未来 24 小时内这颗星没有过境。
  NoPass,
  /// 轨迹取不到（卫星不在 TLE 组里、服务不可用等）。
  TrackFailed,
  /// 轨迹内容不合法。
  TrackInvalid,
  Ready(Box<Ready>),
}

fn now_secs() -> f64 {
  js_sys::Date::now() / 1000.0
}

/// 秒数 → `m:ss` 或 `h:mm:ss`。
fn fmt_countdown(secs: i64) -> String {
  let secs = secs.max(0);
  let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
  if h > 0 {
    format!("{h}:{m:02}:{s:02}")
  } else {
    format!("{m}:{s:02}")
  }
}

/// MHz，保留到 1 Hz（6 位小数）。
fn fmt_mhz(hz: f64) -> String {
  format!("{:.6}", hz / 1e6)
}

/// 带符号的整数 Hz，如 `+10222`。
fn fmt_shift(hz: f64) -> String {
  format!("{hz:+.0}")
}

/// 取下一次过境的轨迹；失败时返回对应的状态。
async fn load_next_pass(norad: u64) -> Load {
  let watch = sat_alert::load();
  let passes = match sat_alert::fetch_passes(&watch, "amateur").await {
    Ok(p) => p,
    Err(_) => return Load::PassesFailed,
  };
  let Some(pass) = next_pass(&passes, norad, now_secs() as i64).cloned() else {
    return Load::NoPass;
  };
  let (start, secs) = track_window(&pass);
  let url = format!(
    "/api/sat-track?norad={norad}&lat={}&lon={}&start={start}&secs={secs}&step={DEFAULT_STEP_SECS}",
    watch.lat, watch.lon
  );
  // 轨迹接口同样可能要等 TLE 回源，用放宽的超时。
  let Ok(api) = crate::data::fetch_external_json_live::<TrackApi>(&url).await else {
    return Load::TrackFailed;
  };
  if api.stale {
    return Load::TrackFailed;
  }
  match Track::new(api.samples) {
    Ok(track) => Load::Ready(Box::new(Ready { pass, track })),
    Err(_) => Load::TrackInvalid,
  }
}

/// 多普勒修正面板。
#[component]
pub(super) fn DopplerTracker() -> impl IntoView {
  let first = PRESETS[0];
  let preset_key = RwSignal::new(first.norad.to_string());
  // 数值存 `Option<f64>`：空输入 = 未填写，而不是沿用旧值。NORAD 为空时 `norad_value`
  // 为 None（加载不触发）；频率为空时以 NAN 传播，`rx/tx_frequency_hz` 校验不过 →
  // `target` 为 None，自动调谐停下，不会用旧频率写电台。
  let norad = RwSignal::new(Some(first.norad as f64));
  let down_mhz = RwSignal::new(Some(first.downlink_hz as f64 / 1e6));
  let up_mhz = RwSignal::new(Some(first.uplink_hz as f64 / 1e6));
  let load = RwSignal::new(Load::Idle);
  let now = RwSignal::new(now_secs());
  // 加载代际：连点两次或中途换了卫星时，晚到的旧响应不能覆盖新状态。
  let load_gen = RwSignal::new(0u32);

  if let Ok(handle) = set_interval_with_handle(move || now.set(now_secs()), Duration::from_secs(1))
  {
    on_cleanup(move || handle.clear());
  }

  let norad_id = unique_id("doppler-norad");
  let down_id = unique_id("doppler-down");
  let up_id = unique_id("doppler-up");
  let preset_id = unique_id("doppler-preset");

  // 手动改任何一个数，都意味着不再是预置卫星，且已加载的过境作废。
  let touched = move || {
    preset_key.set(CUSTOM.to_owned());
    load_gen.update(|g| *g = g.wrapping_add(1));
    load.set(Load::Idle);
  };

  let norad_value = move || {
    let n = norad.get()?;
    (n.fract() == 0.0 && n > 0.0 && n < 1e9).then_some(n as u64)
  };

  let start_load = move || {
    let Some(id) = norad_value() else { return };
    load_gen.update(|g| *g = g.wrapping_add(1));
    let my = load_gen.get_untracked();
    load.set(Load::Loading);
    spawn_local(async move {
      let result = load_next_pass(id).await;
      if load_gen.try_get_untracked() == Some(my) {
        load.set(result);
      }
    });
  };

  // 此刻的插值观测量（卫星在轨迹覆盖范围外为 `None`）。
  let obs = Memo::new(move |_| {
    load.with(|l| match l {
      Load::Ready(r) => r.track.at(now.get()),
      _ => None,
    })
  });
  let down_hz = move || down_mhz.get().unwrap_or(f64::NAN) * 1e6;
  let up_hz = move || up_mhz.get().unwrap_or(f64::NAN) * 1e6;
  // 写电台用的目标：只在卫星高于地平线时才有。
  let target = Signal::derive(move || {
    let o = obs.get()?;
    (o.el >= 0.0)
      .then(|| rx_frequency_hz(down_hz(), o.range_rate_km_s))
      .flatten()
  });

  let preset_options: Vec<SelectOption> = PRESETS
    .iter()
    .map(|p| SelectOption::new(p.norad.to_string(), format!("{} · {}", p.name, p.norad)))
    .chain(std::iter::once(SelectOption::new(
      CUSTOM,
      Signal::derive(move || t("radio.doppler-custom")),
    )))
    .collect();

  view! {
    <section class="pxl-card border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.doppler-title")}</h2>
      <div class="space-y-4 p-4">
        <p class="text-xs text-muted-foreground">{move || t("radio.doppler-description")}</p>

        // `[&>*]:min-w-0`：网格子项默认 `min-width:auto`，长文案会把整列撑宽，手机宽度下整页横向滚动。
        <div class="grid gap-3 sm:grid-cols-2 [&>*]:min-w-0">
          <Field label=Signal::derive(move || t("radio.satellite")) r#for=preset_id.clone()>
            <NativeSelect
              id=preset_id
              value=preset_key
              on_change=Callback::new(move |v: String| {
                load_gen.update(|g| *g = g.wrapping_add(1));
                load.set(Load::Idle);
                if let Some(p) = PRESETS.iter().find(|p| p.norad.to_string() == v) {
                  norad.set(Some(p.norad as f64));
                  down_mhz.set(Some(p.downlink_hz as f64 / 1e6));
                  up_mhz.set(Some(p.uplink_hz as f64 / 1e6));
                }
                preset_key.set(v);
              })
              options=preset_options
              aria_label=Signal::derive(move || t("radio.satellite"))
            />
          </Field>
          <Field label=Signal::derive(move || t("radio.doppler-norad")) r#for=norad_id.clone()>
            <NumberField
              id=norad_id
              value=Signal::derive(move || norad.get().map_or_else(String::new, |v| v.to_string()))
              on_change=Callback::new(move |v: String| {
                norad.set(v.trim().parse::<f64>().ok());
                touched();
              })
              controls=false
            />
          </Field>
          <Field label=Signal::derive(move || t("tools.downlink-frequency-mhz")) r#for=down_id.clone()>
            <NumberField
              id=down_id
              value=Signal::derive(move || down_mhz.get().map_or_else(String::new, |v| v.to_string()))
              on_change=Callback::new(move |v: String| {
                down_mhz.set(v.trim().parse::<f64>().ok());
                touched();
              })
              controls=false
            />
          </Field>
          <Field label=Signal::derive(move || t("radio.doppler-uplink-mhz")) r#for=up_id.clone()>
            <NumberField
              id=up_id
              value=Signal::derive(move || up_mhz.get().map_or_else(String::new, |v| v.to_string()))
              on_change=Callback::new(move |v: String| {
                up_mhz.set(v.trim().parse::<f64>().ok());
                touched();
              })
              controls=false
            />
          </Field>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <Button
            variant=Variant::Default
            size=Size::Default
            disabled=Signal::derive(move || {
              norad_value().is_none() || matches!(load.get(), Load::Loading)
            })
            on_click=Callback::new(move |_| start_load())
          >
            {move || {
              if matches!(load.get(), Load::Loading) {
                t("learning.loading")
              } else {
                t("radio.doppler-load")
              }
            }}
          </Button>
          <span class="text-xs text-muted-foreground">
            {move || {
              let w = sat_alert::load();
              tf("radio.doppler-position", &[&format!("{:.2}", w.lat), &format!("{:.2}", w.lon)])
            }}
          </span>
        </div>

        {move || match load.get() {
          Load::Idle | Load::Loading => None,
          Load::PassesFailed => {
            Some(view! { <p class="text-xs text-destructive" role="alert">{t("radio.pass-data-is-unavailable")}</p> }
              .into_any())
          }
          Load::NoPass => {
            Some(view! { <p class="text-xs text-muted-foreground" role="status">{t("radio.doppler-no-pass")}</p> }
              .into_any())
          }
          Load::TrackFailed => {
            Some(view! { <p class="text-xs text-destructive" role="alert">{t("radio.doppler-error-track")}</p> }
              .into_any())
          }
          Load::TrackInvalid => {
            Some(view! { <p class="text-xs text-destructive" role="alert">{t("radio.doppler-error-invalid")}</p> }
              .into_any())
          }
          Load::Ready(r) => {
            let Ready { pass, .. } = *r;
            let info = tf(
              "radio.doppler-pass-info",
              &[
                &pass.name,
                &sat_alert::local_hm(pass.aos),
                &sat_alert::local_hm(pass.los),
                &format!("{:.0}", pass.max_elev),
              ],
            );
            let (aos, los) = (pass.aos, pass.los);
            Some(view! {
              <div class="space-y-1 border bg-muted/30 px-3 py-2 text-sm tabular-nums">
                <div class="text-xs text-muted-foreground">{info}</div>
                <div role="status" class="font-medium">
                  {move || {
                    let t_now = now.get() as i64;
                    if t_now < aos {
                      tf("radio.doppler-countdown", &[&fmt_countdown(aos - t_now)])
                    } else if t_now <= los {
                      t("radio.doppler-in-pass")
                    } else {
                      t("radio.doppler-after-pass")
                    }
                  }}
                </div>
                {move || {
                  obs.get()
                    .map(|o| {
                      let rx = rx_frequency_hz(down_hz(), o.range_rate_km_s);
                      let tx = tx_frequency_hz(up_hz(), o.range_rate_km_s);
                      view! {
                        <div>
                          {tf(
                            "radio.doppler-look",
                            &[
                              &format!("{:.0}", o.az),
                              &format!("{:.1}", o.el),
                              &format!("{:.0}", o.range_km),
                              &format!("{:.3}", o.range_rate_km_s),
                            ],
                          )}
                        </div>
                        {rx
                          .map(|f| {
                            view! {
                              <div class="font-medium text-foreground">
                                {tf(
                                  "radio.doppler-rx",
                                  &[&fmt_mhz(f), &fmt_shift(f - down_hz())],
                                )}
                              </div>
                            }
                          })}
                        {tx
                          .map(|f| {
                            view! {
                              <div class="font-medium text-foreground">
                                {tf(
                                  "radio.doppler-tx",
                                  &[&fmt_mhz(f), &fmt_shift(f - up_hz())],
                                )}
                              </div>
                            }
                          })}
                        {(o.el < 0.0)
                          .then(|| {
                            view! {
                              <div class="text-xs text-muted-foreground">
                                {t("radio.doppler-below-horizon")}
                              </div>
                            }
                          })}
                      }
                    })
                }}
              </div>
            }
              .into_any())
          }
        }}

        <DopplerRig target=target selection=load_gen />

        <p class="text-xs text-muted-foreground">{move || t("radio.doppler-note")}</p>
      </div>
    </section>
  }
}

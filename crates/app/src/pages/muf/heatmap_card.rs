//! 24 小时 × 波段热力矩阵：纯本地计算（不依赖后端），一眼看出「几点开通、守哪个波段」。
//!
//! 矩阵本身不依赖网络；实时太阳活动（`/api/solar`，HamQSL）只是把太阳通量换算成
//! 太阳黑子数后**填入**输入框，取不到时按手动值计算，离线用法不受影响。

use ham_web_core::muf::ssn_from_sfi;
use ham_web_core::voacap::{Heatmap, heatmap, hop_geometry};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::cn::cn;
use crate::data;
use crate::i18n::{t, tf, tp};
use crate::ui::{ControlSize, Input, InputType, NumberField, Slider};

const NOTE: &str = "text-xs text-muted-foreground";
const RESULT: &str = "rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground";
const CURVE_W: f64 = 600.0;
const CURVE_H: f64 = 110.0;
const CURVE_PAD: f64 = 24.0;

/// 地磁活动达到该 K 指数时给出提示（简化模型不含地磁扰动项）。
const K_ALERT: f64 = 4.0;

/// 服务端 `/api/solar`（HamQSL）返回的字段子集。
#[derive(Deserialize, Clone, Copy)]
struct SolarApi {
  solar_flux: Option<f64>,
  a_index: Option<f64>,
  k_index: Option<f64>,
  sunspots: Option<f64>,
}

/// 实时太阳活动快照。
#[derive(Clone, Copy)]
struct LiveSolar {
  sfi: f64,
  ssn: f64,
  a: Option<f64>,
  k: Option<f64>,
}

/// 被选中的热力图格子：`(波段, UTC 整点)`。
type Cell = (&'static str, usize);

/// 可靠度 → 色块类名（Tailwind 需完整字面量，不能拼接片段）。
fn cell_class(r: f64) -> &'static str {
  if r <= 0.0 {
    "bg-muted"
  } else if r < 0.25 {
    "bg-sky-500/25"
  } else if r < 0.5 {
    "bg-sky-500/50"
  } else if r < 0.75 {
    "bg-emerald-500/60"
  } else {
    "bg-emerald-500/90"
  }
}

fn hour_label(h: usize) -> String {
  if h.is_multiple_of(3) {
    format!("{h:02}")
  } else {
    String::new()
  }
}

#[component]
pub(super) fn HeatmapCard() -> impl IntoView {
  let tx = RwSignal::new(String::from("OM89"));
  let rx = RwSignal::new(String::from("IO91"));
  let month = RwSignal::new(String::from("10"));
  let ssn = RwSignal::new(String::from("100"));
  // 「现在几点」只在挂载时读一次；之后用滑块自由拨动。
  let hour = RwSignal::new(js_sys::Date::new_0().get_utc_hours().min(23));
  // 被点开的热力图格子。
  let selected = RwSignal::new(None::<Cell>);

  // 实时太阳活动：取到后填进 SSN 输入框；用户手动改过就不再覆盖。
  let live = RwSignal::new(None::<LiveSolar>);
  let fetched = RwSignal::new(false);
  let touched = RwSignal::new(false);
  let apply_live = move || {
    if let Some(l) = live.get() {
      ssn.set(format!("{:.0}", l.ssn));
      touched.set(false);
    }
  };
  // 用户在 `/api/solar` 返回前离开 `/muf` 时，本卡片的信号已随组件释放，`await` 之后
  // 再碰它们会 panic（整个 wasm 实例崩掉）。`alive` 必须在组件体里创建 —— `on_cleanup`
  // 在没有 Owner 上下文时是静默空操作（见 `util::mount_guard`）。
  let alive = crate::util::mount_guard();
  spawn_local(async move {
    let api = data::fetch_external_json::<SolarApi>("/api/solar").await;
    if !alive() {
      return;
    }
    if let Ok(api) = api
      && let Some(sfi) = api.solar_flux
    {
      // 上游偶有 sunspots 缺失或为 0，此时用全站统一的 SFI ↔ SSN 换算兜底。
      let ssn_value = api
        .sunspots
        .filter(|v| *v > 0.0)
        .unwrap_or_else(|| ssn_from_sfi(sfi));
      live.set(Some(LiveSolar {
        sfi,
        ssn: ssn_value,
        a: api.a_index,
        k: api.k_index,
      }));
      if !touched.get_untracked() {
        ssn.set(format!("{ssn_value:.0}"));
      }
    }
    fetched.set(true);
  });

  let month_val = move || month.get().trim().parse::<u32>().unwrap_or(10).clamp(1, 12);
  let ssn_val = move || {
    ssn
      .get()
      .trim()
      .parse::<f64>()
      .unwrap_or(100.0)
      .clamp(0.0, 400.0)
  };
  let valid = move || tx.get().trim().len() >= 4 && rx.get().trim().len() >= 4;

  let map: Memo<Option<Heatmap>> = Memo::new(move |_| {
    if !valid() {
      return None;
    }
    heatmap(tx.get().trim(), rx.get().trim(), month_val(), ssn_val())
  });

  let hour_idx = move || (hour.get() as usize).min(23);

  // 选定时刻的波段推荐（按可靠度降序）。
  let top_bands = move || {
    let Some(m) = map.get() else {
      return Vec::new();
    };
    let h = hour_idx();
    let mut v: Vec<(&'static str, f64)> = m
      .rows
      .iter()
      .map(|r| (r.band, r.cells.get(h).copied().unwrap_or(0.0)))
      .filter(|(_, r)| *r > 0.0)
      .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v.truncate(4);
    v
  };

  let curve_path = move |which: u8| -> String {
    let Some(m) = map.get() else {
      return String::new();
    };
    let y_max = m
      .muf_hourly
      .iter()
      .copied()
      .fold(35.0f64, f64::max)
      .max(35.0)
      * 1.1;
    let py = |v: f64| CURVE_H - CURVE_PAD - (v / y_max) * (CURVE_H - 2.0 * CURVE_PAD);
    let px = |h: usize| CURVE_PAD + h as f64 / 23.0 * (CURVE_W - 2.0 * CURVE_PAD);
    let series = if which == 0 {
      &m.muf_hourly
    } else {
      &m.luf_hourly
    };
    series
      .iter()
      .enumerate()
      .map(|(h, &v)| {
        if h == 0 {
          format!("M {:.1} {:.1}", px(h), py(v))
        } else {
          format!(" L {:.1} {:.1}", px(h), py(v))
        }
      })
      .collect()
  };
  let marker_x = move || CURVE_PAD + hour_idx() as f64 / 23.0 * (CURVE_W - 2.0 * CURVE_PAD);

  view! {
    <section id="heatmap" class="scroll-mt-24 rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("tools.24-hour-opening-window")}</h2>
      <p class="px-4 pt-3 text-xs text-muted-foreground">
        {move || t("tools.running-the-prediction-for")}
      </p>

      <div class="space-y-1.5 px-4 pt-2">
        {move || {
          let Some(l) = live.get() else {
            return if fetched.get() {
              view! {
                <p class=NOTE>
                  {move || t("tools.live-solar-data-unavailable")}
                </p>
              }
              .into_any()
            } else {
              view! {
                <p class=NOTE>{move || t("tools.fetching-live-solar-activity")}</p>
              }
              .into_any()
            };
          };
          let sfi = format!("{:.0}", l.sfi);
          let ssn_s = format!("{:.0}", l.ssn);
          let a_s = l.a.map_or_else(|| "—".to_owned(), |v| format!("{v:.0}"));
          let k_s = l.k.map_or_else(|| "—".to_owned(), |v| format!("{v:.0}"));
          let geomagnetic = l.k.is_some_and(|k| k >= K_ALERT);
          view! {
            <div class="flex flex-wrap items-center gap-2">
              <p class=NOTE>
                {tf(
                  "common.live-solar-activity-source",
                  &[&sfi, &ssn_s, &a_s, &k_s],
                )}
              </p>
              <button
                type="button"
                class="cursor-pointer rounded-md border bg-background px-2 py-0.5 text-xs font-medium hover:bg-accent"
                on:click=move |_| apply_live()
              >
                {move || t("tools.update-ssn-from-live")}
              </button>
            </div>
            {if geomagnetic {
              view! {
                <p class=NOTE>
                  {tf(
                    "common.geomagnetically-active-k-high",
                    &[&k_s],
                  )}
                </p>
              }
              .into_any()
            } else {
              ().into_any()
            }}
          }
          .into_any()
        }}
      </div>

      <div class="grid gap-3 p-4 sm:grid-cols-4">
        <label class="flex flex-col gap-1.5">
          <span class=NOTE>{move || t("log.station-grid")}</span>
          <Input
            value=Signal::derive(move || tx.get())
            on_change=Callback::new(move |v: String| tx.set(v))
            kind=InputType::Text
            size=ControlSize::Sm
            placeholder="OM89"
            aria_label=Signal::derive(move || t("radio.transmit-grid"))
          />
        </label>
        <label class="flex flex-col gap-1.5">
          <span class=NOTE>{move || t("log.their-grid")}</span>
          <Input
            value=Signal::derive(move || rx.get())
            on_change=Callback::new(move |v: String| rx.set(v))
            kind=InputType::Text
            size=ControlSize::Sm
            placeholder="IO91"
            aria_label=Signal::derive(move || t("radio.receive-grid"))
          />
        </label>
        <label class="flex flex-col gap-1.5">
          <span class=NOTE>{move || t("radio.month")}</span>
          <NumberField
            value=Signal::derive(move || month.get())
            on_change=Callback::new(move |v: String| month.set(v))
            step=1.0
            min=1.0
            max=12.0
            size=ControlSize::Sm
            aria_label=Signal::derive(move || t("radio.month"))
          />
        </label>
        <label class="flex flex-col gap-1.5">
          <span class=NOTE>{move || t("radio.sunspot-number")}</span>
          <NumberField
            value=Signal::derive(move || ssn.get())
            on_change=Callback::new(move |v: String| {
              touched.set(true);
              ssn.set(v);
            })
            step=10.0
            min=0.0
            max=400.0
            size=ControlSize::Sm
            aria_label=Signal::derive(move || t("radio.sunspot-number"))
          />
        </label>
      </div>

      {move || {
        if !valid() {
          return view! {
            <p class="px-4 pb-4 text-sm text-muted-foreground">
              {move || t("tools.enter-at-least-a")}
            </p>
          }
          .into_any();
        }
        let Some(m) = map.get() else {
          return view! {
            <p class="px-4 pb-4 text-sm text-muted-foreground">
              {move || t("tools.unrecognised-grid-format-check")}
            </p>
          }
          .into_any();
        };
        // 先把需要的字段取成局部量：`m` 会被下面的多个闭包分别捕获，
        // 直接引用会让它被移动两次。逐小时的向量与矩阵行还要被「矩阵」「摘要」
        // 「逐格下钻」三处共用，用 `StoredValue`（Copy）共享而不是克隆三份。
        let distance_km = m.distance_km;
        let bearing_deg = m.bearing_deg;
        let best_hour = m.best_hour_utc;
        let local_hours = StoredValue::new(m.local_hours);
        let muf_hourly = StoredValue::new(m.muf_hourly);
        let luf_hourly = StoredValue::new(m.luf_hourly);
        let rows = StoredValue::new(m.rows);
        view! {
          <div class="space-y-3 px-4 pb-4">
            <div class="overflow-x-auto">
              <div class="min-w-[560px] space-y-0.5">
                <div class="flex items-center gap-0.5">
                  <div class="w-12 shrink-0"></div>
                  <div class="flex flex-1 gap-0.5">
                    {(0..24usize)
                      .map(|h| {
                        view! {
                          <div class="flex-1 text-center text-[10px] tabular-nums text-muted-foreground">
                            {hour_label(h)}
                          </div>
                        }
                      })
                      .collect_view()}
                  </div>
                </div>
                {rows
                  .with_value(|rows| {
                    rows
                      .iter()
                      .rev()
                      .map(|row| {
                        let band = row.band;
                        let cells = row.cells.clone();
                        view! {
                      <div class="flex items-center gap-0.5">
                        <div class="w-12 shrink-0 text-xs font-medium">{band}</div>
                        <div class="flex flex-1 gap-0.5">
                          {cells
                            .iter()
                            .enumerate()
                            .map(|(h, &c)| {
                              let label = format!("{band} · {h:02}:00 UTC · {:.0}%", c * 100.0);
                              let aria = label.clone();
                              let class = move || {
                                cn(&[
                                  "h-5 flex-1 cursor-pointer rounded-[2px] p-0 transition-shadow",
                                  cell_class(c),
                                  if selected.get() == Some((band, h)) {
                                    "ring-2 ring-primary ring-inset"
                                  } else {
                                    ""
                                  },
                                ])
                              };
                              view! {
                                <button
                                  type="button"
                                  class=class
                                  title=label
                                  aria-label=aria
                                  aria-pressed=move || {
                                    if selected.get() == Some((band, h)) { "true" } else { "false" }
                                  }
                                  on:click=move |_| selected.set(Some((band, h)))
                                ></button>
                              }
                            })
                            .collect_view()}
                        </div>
                      </div>
                    }
                  })
                  .collect_view()
                  })}
              </div>
            </div>

            <div class="flex flex-wrap items-center gap-3">
              <button
                type="button"
                class="cursor-pointer rounded-md border bg-background px-3 py-1 text-xs font-medium hover:bg-accent"
                on:click=move |_| hour.set(best_hour)
              >
                {move || tf("tools.jump-to-the-best", &[&format!("{best_hour:02}")])}
              </button>
              <div class="flex min-w-[16rem] flex-1 items-center gap-3">
                <span class=NOTE>{move || tf("tools.viewing-00-utc", &[&format!("{:02}", hour.get())])}</span>
                <Slider
                  value=Signal::derive(move || f64::from(hour.get()))
                  on_change=Callback::new(move |v: f64| hour.set(v.round().clamp(0.0, 23.0) as u32))
                  min=0.0
                  max=23.0
                  step=1.0
                  aria_label=Signal::derive(move || t("tools.hour-to-view-utc"))
                  aria_valuetext=Signal::derive(move || format!("{:02}:00 UTC", hour.get()))
                  class="flex-1"
                />
              </div>
            </div>

            <div class=RESULT>
              {move || {
                let h = hour_idx();
                tf(
                  "tools.distance-km-bearing-local",
                  &[
                    &format!("{distance_km:.0}"),
                    &format!("{bearing_deg:.0}"),
                    &format!("{:.1}", local_hours.with_value(|v| v.get(h).copied().unwrap_or(0.0))),
                    &format!("{:.1}", muf_hourly.with_value(|v| v.get(h).copied().unwrap_or(0.0))),
                    &format!("{:.1}", luf_hourly.with_value(|v| v.get(h).copied().unwrap_or(0.0))),
                  ],
                )
              }}
            </div>

            <div class="space-y-2">
              {move || {
                let Some((band, h)) = selected.get() else {
                  return view! {
                    <p class=NOTE>
                      {move || t("tools.click-a-cell-in")}
                    </p>
                  }
                  .into_any();
                };
                let Some((freq_mhz, reliability)) = rows.with_value(|rows| {
                  rows
                    .iter()
                    .find(|r| r.band == band)
                    .map(|r| (r.freq_mhz, r.cells.get(h).copied().unwrap_or(0.0)))
                }) else {
                  return ().into_any();
                };
                let muf = muf_hourly.with_value(|v| v.get(h).copied().unwrap_or(0.0));
                let luf = luf_hourly.with_value(|v| v.get(h).copied().unwrap_or(0.0));
                let local = local_hours.with_value(|v| v.get(h).copied().unwrap_or(0.0));
                // 跳数与仰角取自与实际计算同一份几何（`hop_geometry`），
                // 保证「说明里的跳数」就是「算 MUF 用的跳数」。
                let geom = hop_geometry(distance_km);
                let usable = freq_mhz <= muf && freq_mhz >= luf;
                let band_s = band.to_owned();
                let hour_s = format!("{h:02}");
                let freq_s = format!("{freq_mhz:.3}");
                let muf_s = format!("{muf:.1}");
                let luf_s = format!("{luf:.1}");
                let rel_s = format!("{:.0}", reliability * 100.0);
                let local_s = format!("{local:.1}");
                let hops_s = format!("{}", geom.hops);
                let hop_km_s = format!("{:.0}", geom.hop_km);
                let elev_s = format!("{:.1}", geom.elevation_deg);
                view! {
                  <div class=RESULT>
                    {tf(
                      "common.00-utc-frequency-mhz",
                      &[&band_s, &hour_s, &freq_s, &muf_s, &luf_s, &rel_s],
                    )}
                  </div>
                  <div class=RESULT>
                    {tp(
                      "common.local-time-at-path",
                      geom.hops,
                      &[&local_s, &hops_s, &hop_km_s, &elev_s],
                    )}
                  </div>
                  {if usable {
                    ().into_any()
                  } else {
                    view! {
                      <p class=NOTE>
                        {move || t("tools.this-band-is-outside")}
                      </p>
                    }
                    .into_any()
                  }}
                }
                .into_any()
              }}
            </div>

            <div>
              <div class=NOTE>{move || t("tools.hourly-muf-luf-blue")}</div>
              <svg
                viewBox=format!("0 0 {CURVE_W} {CURVE_H}")
                class="mx-auto w-full"
                role="img"
                aria-label=move || t("tools.hourly-muf-and-luf")
              >
                <line
                  x1=CURVE_PAD
                  y1=CURVE_H - CURVE_PAD
                  x2=CURVE_W - CURVE_PAD
                  y2=CURVE_H - CURVE_PAD
                  class="stroke-muted-foreground/30"
                  stroke-width="1"
                />
                <line
                  x1=marker_x
                  y1=CURVE_PAD
                  x2=marker_x
                  y2=CURVE_H - CURVE_PAD
                  class="stroke-amber-500/60"
                  stroke-dasharray="3 3"
                  stroke-width="1"
                />
                <path
                  d=move || curve_path(1)
                  class="fill-none stroke-amber-500"
                  stroke-width="1.5"
                  vector-effect="non-scaling-stroke"
                />
                <path
                  d=move || curve_path(0)
                  class="fill-none stroke-sky-500"
                  stroke-width="1.8"
                  vector-effect="non-scaling-stroke"
                />
              </svg>
            </div>

            <div>
              <div class=NOTE>{move || t("tools.recommended-bands-at-this")}</div>
              <div class="mt-1 space-y-1.5">
                {move || {
                  let bands = top_bands();
                  if bands.is_empty() {
                    return view! {
                      <p class="text-sm text-muted-foreground">
                        {move || t("tools.no-usable-band-at")}
                      </p>
                    }
                    .into_any();
                  }
                  let list = bands
                    .into_iter()
                    .map(|(band, r)| {
                      let pct = r * 100.0;
                      view! {
                        <div class="flex items-center gap-2 text-sm">
                          <span class="w-12 shrink-0 font-medium">{band}</span>
                          <div class="h-3 flex-1 overflow-hidden rounded bg-muted">
                            <div class="h-full bg-emerald-500" style=format!("width: {pct:.0}%")></div>
                          </div>
                          <span class="w-14 shrink-0 text-right text-xs tabular-nums text-muted-foreground">
                            {format!("{pct:.0}%")}
                          </span>
                        </div>
                      }
                    })
                    .collect_view();
                  list.into_any()
                }}
              </div>
            </div>
          </div>
        }
        .into_any()
      }}
    </section>
  }
}

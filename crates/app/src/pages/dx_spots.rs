//! DX 实时热点：全球 DX Cluster 实时通联报告，支持按波段 / 模式筛选，对照本地日志标出需要的实体，
//! 并可在出现新 DXCC / 新波段 / 关注呼号时发浏览器通知（页面打开期间每分钟自动刷新）。

use std::collections::HashSet;
use std::time::Duration;

use ham_web_core::dx_watch::{
  AlertReason, AlertSettings, Need, Worked, alert_key, band_of_khz as band_of,
  mode_of_comment as mode_of,
};
use ham_web_core::dxcc::lookup;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::icons::{Icon, IconKind};
use crate::pages::log::use_log_store;
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{notify, request_notify_permission, set_title, storage};

const ALERTS_KEY: &str = "dx-alerts";
const REFRESH: Duration = Duration::from_secs(60);

/// 服务端 `/api/spots` 返回的一条 spot。
#[derive(Deserialize, Clone)]
struct Spot {
  spotter: String,
  freq_khz: u32,
  dx: String,
  country: String,
  comment: String,
  time: String,
}

/// 可筛选的波段。
const BANDS: &[&str] = &[
  "160m", "80m", "40m", "30m", "20m", "17m", "15m", "12m", "10m", "6m", "2m",
];
/// 可筛选的模式。
const MODES: &[&str] = &["CW", "SSB", "FT8", "FT4", "RTTY"];
/// 筛选按钮激活 / 未激活样式。
const CHIP_ON: &str = "shrink-0 whitespace-nowrap rounded-full border bg-primary px-2.5 py-1 text-xs text-primary-foreground";
const CHIP_OFF: &str =
  "shrink-0 whitespace-nowrap rounded-full border px-2.5 py-1 text-xs hover:bg-accent";

/// 波段 → chip 着色样式（深色模式改用更亮的文字色以保证对比度）。
fn band_color(band: &str) -> &'static str {
  match band {
    "160m" => "border-red-500/30 bg-red-500/10 text-red-500 dark:text-red-300",
    "80m" => "border-orange-500/30 bg-orange-500/10 text-orange-500 dark:text-orange-300",
    "40m" => "border-amber-500/30 bg-amber-500/10 text-amber-600 dark:text-amber-300",
    "30m" => "border-yellow-500/30 bg-yellow-500/10 text-yellow-600 dark:text-yellow-300",
    "20m" => "border-green-500/30 bg-green-500/10 text-green-600 dark:text-green-300",
    "17m" => "border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-300",
    "15m" => "border-teal-500/30 bg-teal-500/10 text-teal-600 dark:text-teal-300",
    "12m" => "border-sky-500/30 bg-sky-500/10 text-sky-600 dark:text-sky-300",
    "10m" => "border-blue-500/30 bg-blue-500/10 text-blue-600 dark:text-blue-300",
    "6m" => "border-violet-500/30 bg-violet-500/10 text-violet-600 dark:text-violet-300",
    "2m" => "border-purple-500/30 bg-purple-500/10 text-purple-600 dark:text-purple-300",
    _ => "border bg-muted/40 text-muted-foreground",
  }
}

/// 频率格式化：kHz → MHz（去尾随 0）。
fn fmt_freq(khz: u32) -> String {
  let mhz = khz as f64 / 1000.0;
  let mut s = format!("{mhz:.3}");
  while s.ends_with('0') {
    s.pop();
  }
  if s.ends_with('.') {
    s.pop();
  }
  format!("{s} MHz")
}

#[component]
pub fn DxSpotsPage() -> impl IntoView {
  set_title("DX 实时热点");

  let store = use_log_store();
  let spots = RwSignal::new(Vec::<Spot>::new());
  let loading = RwSignal::new(true);
  let failed = RwSignal::new(false);
  let band_filter = RwSignal::new(None::<&'static str>);
  let mode_filter = RwSignal::new(None::<&'static str>);
  let needed_only = RwSignal::new(false);
  let has_log = Memo::new(move |_| store.logbook.with(|l| !l.entries.is_empty()));
  let needs = Memo::new(move |_| {
    let worked = store.logbook.with(|l| Worked::from_entries(&l.entries));
    spots.with(|list| {
      list
        .iter()
        .map(|s| worked.need(&s.dx, band_of(s.freq_khz)))
        .collect::<Vec<Need>>()
    })
  });
  let needed_count = Memo::new(move |_| needs.with(|n| n.iter().filter(|x| x.is_needed()).count()));

  // 提醒设置与本次打开页面期间已提醒过的 spot。
  let alerts = RwSignal::new(storage::get_json::<AlertSettings>(ALERTS_KEY).unwrap_or_default());
  let show_alerts = RwSignal::new(false);
  let watch_input = RwSignal::new(alerts.with_untracked(|a| a.calls.join(" ")));
  let notified = StoredValue::new(HashSet::<String>::new());
  let update_alerts = move |f: &dyn Fn(&mut AlertSettings)| {
    alerts.update(|a| f(a));
    storage::set_json(ALERTS_KEY, &alerts.get_untracked());
    if alerts.with_untracked(AlertSettings::enabled) {
      request_notify_permission();
    }
  };
  Effect::new(move |_| {
    let settings = alerts.get();
    if !settings.enabled() {
      return;
    }
    let need_list = needs.get();
    let mut hits: Vec<String> = Vec::new();
    spots.with(|list| {
      for (s, need) in list.iter().zip(need_list) {
        let band = band_of(s.freq_khz);
        let Some(reason) = settings.reason(&s.dx, need) else {
          continue;
        };
        if !notified
          .try_update_value(|n| n.insert(alert_key(&s.dx, band)))
          .unwrap_or(false)
        {
          continue;
        }
        let why = match reason {
          AlertReason::Watched(p) => format!("关注 {p}"),
          AlertReason::NewDxcc => "新 DXCC".to_owned(),
          AlertReason::NewBand => "新波段".to_owned(),
        };
        hits.push(format!(
          "{} {} {}（{why}）",
          s.dx,
          fmt_freq(s.freq_khz),
          mode_of(&s.comment)
        ));
      }
    });
    match hits.len() {
      0 => {}
      1..=3 => hits.iter().for_each(|h| notify(&format!("DX 热点：{h}"))),
      n => notify(&format!("DX 热点：{} 等 {n} 条需要的报告", hits[0])),
    }
  });

  let load = move || {
    loading.set(true);
    failed.set(false);
    spawn_local(async move {
      match data::fetch_external_json::<Vec<Spot>>("/api/spots").await {
        Ok(s) => {
          spots.try_set(s);
        }
        Err(_) => {
          failed.try_set(true);
        }
      }
      loading.try_set(false);
    });
  };

  load();
  if let Ok(handle) = set_interval_with_handle(
    move || {
      if alerts.try_with_untracked(AlertSettings::enabled) == Some(true) {
        load();
      }
    },
    REFRESH,
  ) {
    on_cleanup(move || handle.clear());
  }

  // 点击呼号一键记入日志。
  let add_to_log = move |dx: String, freq_khz: u32, comment: String| {
    let mhz = freq_khz as f64 / 1000.0;
    let mut s = format!("{mhz:.3}");
    while s.ends_with('0') {
      s.pop();
    }
    if s.ends_with('.') {
      s.pop();
    }
    let mode = mode_of(&comment);
    let mode = if mode == "其他" { "SSB" } else { mode };
    store.quick_add(&dx, &s, mode);
    crate::util::alert(&format!("已加入日志：{dx}（{s} MHz {mode}）"));
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"DX 实时热点"</h1>
            <div class="text-xs text-muted-foreground">"DX Cluster · 全球实时通联"</div>
          </div>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Sm, "")
            on:click=move |_| load()
          >
            {move || {
              if loading.get() {
                view! {
                  <span class="inline-flex items-center gap-1.5">
                    <Icon kind=IconKind::Loader2 class="h-3.5 w-3.5 animate-spin" />
                    "刷新中"
                  </span>
                }
                .into_any()
              } else {
                "刷新".into_any()
              }
            }}
          </button>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="flex items-center justify-between border-b px-4 py-3 text-sm font-semibold">
            "实时 DX 报告"
            <span class="text-xs font-normal text-muted-foreground">
              {move || format!("{} 条", spots.get().len())}
            </span>
          </h2>
          <div class="space-y-2 p-3">
            <div class="flex flex-wrap items-center gap-1.5">
              <span class="mr-1 text-xs text-muted-foreground">"波段"</span>
              <button
                type="button"
                on:click=move |_| band_filter.set(None)
                class=move || if band_filter.get().is_none() { CHIP_ON } else { CHIP_OFF }
              >
                "全部"
              </button>
              {BANDS
                .iter()
                .map(|b| {
                  let b = *b;
                  view! {
                    <button
                      type="button"
                      on:click=move |_| band_filter.set(if band_filter.get() == Some(b) { None } else { Some(b) })
                      class=move || if band_filter.get() == Some(b) { CHIP_ON } else { CHIP_OFF }
                    >
                      {b}
                    </button>
                  }
                })
                .collect_view()}
            </div>
            <div class="flex flex-wrap items-center gap-1.5">
              <span class="mr-1 text-xs text-muted-foreground">"模式"</span>
              <button
                type="button"
                on:click=move |_| mode_filter.set(None)
                class=move || if mode_filter.get().is_none() { CHIP_ON } else { CHIP_OFF }
              >
                "全部"
              </button>
              {MODES
                .iter()
                .map(|m| {
                  let m = *m;
                  view! {
                    <button
                      type="button"
                      on:click=move |_| mode_filter.set(if mode_filter.get() == Some(m) { None } else { Some(m) })
                      class=move || if mode_filter.get() == Some(m) { CHIP_ON } else { CHIP_OFF }
                    >
                      {m}
                    </button>
                  }
                })
                .collect_view()}
            </div>
            {move || {
              has_log.get().then(|| {
                view! {
                  <div class="flex flex-wrap items-center gap-1.5">
                    <span class="mr-1 text-xs text-muted-foreground">"日志"</span>
                    <button
                      type="button"
                      on:click=move |_| needed_only.update(|v| *v = !*v)
                      class=move || if needed_only.get() { CHIP_ON } else { CHIP_OFF }
                      title="只显示日志中未通联的实体，或该实体尚未通联的波段"
                    >
                      {move || format!("只看需要的（{}）", needed_count.get())}
                    </button>
                  </div>
                }
              })
            }}
            <div class="flex flex-wrap items-center gap-1.5">
              <span class="mr-1 text-xs text-muted-foreground">"提醒"</span>
              <button
                type="button"
                on:click=move |_| show_alerts.update(|v| *v = !*v)
                class=move || if alerts.with(AlertSettings::enabled) { CHIP_ON } else { CHIP_OFF }
              >
                {move || {
                  let a = alerts.get();
                  if a.enabled() {
                    let mut parts = Vec::new();
                    if a.new_dxcc {
                      parts.push("新 DXCC".to_owned());
                    }
                    if a.new_band {
                      parts.push("新波段".to_owned());
                    }
                    if !a.calls.is_empty() {
                      parts.push(format!("关注 {} 个", a.calls.len()));
                    }
                    format!("已开启：{}", parts.join(" · "))
                  } else {
                    "设置通知提醒".to_owned()
                  }
                }}
              </button>
            </div>
            {move || {
              show_alerts.get().then(|| {
                let toggle = move |label: &'static str, get: fn(&AlertSettings) -> bool, set: fn(&mut AlertSettings, bool)| {
                  view! {
                    <label class="flex items-center gap-2 text-sm">
                      <input
                        type="checkbox"
                        class="size-4 accent-primary"
                        prop:checked=move || alerts.with(get)
                        on:change=move |e| {
                          let on = event_target_checked(&e);
                          update_alerts(&|a| set(a, on));
                        }
                      />
                      {label}
                    </label>
                  }
                };
                view! {
                  <div class="space-y-3 rounded-lg border bg-muted/30 p-3">
                    <div class="flex flex-wrap gap-x-5 gap-y-2">
                      {toggle("出现新 DXCC 时通知", |a| a.new_dxcc, |a, v| a.new_dxcc = v)}
                      {toggle("出现已联实体的新波段时通知", |a| a.new_band, |a, v| a.new_band = v)}
                    </div>
                    <label class="flex flex-col gap-1.5 text-sm">
                      <span class="text-xs text-muted-foreground">"关注呼号（空格或逗号分隔，* 为通配符，如 VP8* 3Y0J */P）"</span>
                      <input
                        type="text"
                        class=input_class("uppercase")
                        placeholder="VP8* 3Y0J"
                        prop:value=move || watch_input.get()
                        on:input=move |e| watch_input.set(event_target_value(&e))
                        on:change=move |_| {
                          let calls: Vec<String> = watch_input
                            .get_untracked()
                            .split([' ', ',', '，'])
                            .map(|s| s.trim().to_uppercase())
                            .filter(|s| !s.is_empty())
                            .collect();
                          update_alerts(&|a| a.calls.clone_from(&calls));
                        }
                      />
                    </label>
                    <p class="text-xs text-muted-foreground">
                      "开启后页面每分钟自动刷新，命中时发送浏览器通知（需允许通知权限；同一呼号同一波段只提醒一次）。关闭页面后不再提醒。"
                    </p>
                  </div>
                }
              })
            }}
          </div>
          <div class="p-2">
            {move || {
              if loading.get() {
                return view! {
                  <p class="px-3 py-8 text-center text-sm text-muted-foreground">"正在获取实时热点…"</p>
                }
                .into_any();
              }
              if failed.get() || spots.get().is_empty() {
                return view! {
                  <p class="px-3 py-8 text-center text-sm text-muted-foreground">
                    "实时热点暂不可用（可能因网络受限），稍后重试。"
                  </p>
                }
                .into_any();
              }
              let list = spots.get();
              let band = band_filter.get();
              let mode = mode_filter.get();
              let only_needed = needed_only.get();
              let need_list = needs.get();
              let filtered: Vec<(Spot, Need)> = list
                .into_iter()
                .zip(need_list)
                .filter(|(s, n)| {
                  band.is_none_or(|b| band_of(s.freq_khz) == b)
                    && mode.is_none_or(|m| mode_of(&s.comment) == m)
                    && (!only_needed || matches!(n, Need::NewDxcc | Need::NewBand))
                })
                .collect();
              if filtered.is_empty() {
                return view! {
                  <p class="px-3 py-8 text-center text-sm text-muted-foreground">"当前筛选下暂无报告，试试其他波段或模式。"</p>
                }
                .into_any();
              }
              view! {
                <div class="divide-y">
                  {filtered
                    .iter()
                    .map(|(s, need)| {
                      let freq = s.freq_khz;
                      let dx = s.dx.clone();
                      let country = if s.country.is_empty() {
                        lookup(&s.dx).map(|e| e.name.to_owned()).unwrap_or_default()
                      } else {
                        s.country.clone()
                      };
                      let badge = match need {
                        Need::NewDxcc => Some(("新 DXCC", "border-emerald-500/40 bg-emerald-500/15 text-emerald-700 dark:text-emerald-300")),
                        Need::NewBand => Some(("新波段", "border-sky-500/40 bg-sky-500/15 text-sky-700 dark:text-sky-300")),
                        Need::Worked => Some(("已联", "text-muted-foreground")),
                        Need::None => None,
                      };
                      let watched = alerts.with_untracked(|a| {
                        matches!(a.reason(&s.dx, Need::None), Some(AlertReason::Watched(_)))
                      });
                      let comment = s.comment.clone();
                      let time = s.time.clone();
                      let spotter = s.spotter.clone();
                      let dx_btn = dx.clone();
                      let comment_btn = comment.clone();
                      view! {
                        <div class="flex flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2.5">
                          <button
                            type="button"
                            class="w-28 shrink-0 text-left"
                            title="点击记入日志"
                            on:click=move |_| add_to_log(dx_btn.clone(), freq, comment_btn.clone())
                          >
                            <div class="font-mono text-sm font-semibold text-primary">{dx.clone()}</div>
                            {if country.is_empty() {
                              view! { <div></div> }.into_any()
                            } else {
                              view! {
                                <div class="truncate text-xs text-muted-foreground">{country.clone()}</div>
                              }
                              .into_any()
                            }}
                          </button>
                          <span class="w-24 shrink-0 font-mono text-sm tabular-nums">{fmt_freq(freq)}</span>
                          <span
                            class=format!("shrink-0 rounded border px-1.5 py-0.5 text-xs {}", band_color(band_of(freq)))
                          >
                            {band_of(freq)}
                          </span>
                          <span class="shrink-0 rounded border px-1.5 py-0.5 text-xs text-muted-foreground">
                            {mode_of(&comment)}
                          </span>
                          {badge.map(|(text, class)| view! {
                            <span class=format!("shrink-0 rounded border px-1.5 py-0.5 text-xs font-medium {class}")>{text}</span>
                          })}
                          {watched.then(|| view! {
                            <span class="shrink-0 rounded border border-amber-500/40 bg-amber-500/15 px-1.5 py-0.5 text-xs font-medium text-amber-700 dark:text-amber-300">"关注"</span>
                          })}
                          <span class="min-w-0 flex-1 truncate text-sm text-muted-foreground">
                            {if comment.is_empty() { "—".to_owned() } else { comment.clone() }}
                          </span>
                          <span class="shrink-0 text-xs text-muted-foreground">{time.clone()}</span>
                          <span class="shrink-0 text-xs text-muted-foreground" title="报告者">
                            {spotter.clone()}
                          </span>
                        </div>
                      }
                    })
                    .collect_view()}
                </div>
              }
              .into_any()
            }}
          </div>
        </section>

        <p class="text-xs text-muted-foreground">
          "数据来自 DXWatch 全球 DX Cluster，反映当前正在被报告的电台、频率与时间，供追 DX / 守听参考。"
        </p>
      </div>
    </div>
  }
}

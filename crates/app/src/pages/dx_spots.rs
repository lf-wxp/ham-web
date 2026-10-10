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

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::ui::{Button, Checkbox, ChipToggle, Input, Size, Variant};
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

/// 波段 → chip 着色样式（深色模式改用更亮的文字色以保证对比度）。
fn band_color(band: &str) -> &'static str {
  match band {
    "160m" => "border-red-500/30 bg-red-500/10 text-red-700 dark:text-red-300",
    "80m" => "border-orange-500/30 bg-orange-500/10 text-orange-700 dark:text-orange-300",
    "40m" => "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300",
    "30m" => "border-yellow-500/30 bg-yellow-500/10 text-yellow-700 dark:text-yellow-300",
    "20m" => "border-green-500/30 bg-green-500/10 text-green-700 dark:text-green-300",
    "17m" => "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
    "15m" => "border-teal-500/30 bg-teal-500/10 text-teal-700 dark:text-teal-300",
    "12m" => "border-sky-500/30 bg-sky-500/10 text-sky-700 dark:text-sky-300",
    "10m" => "border-blue-500/30 bg-blue-500/10 text-blue-700 dark:text-blue-300",
    "6m" => "border-violet-500/30 bg-violet-500/10 text-violet-700 dark:text-violet-300",
    "2m" => "border-purple-500/30 bg-purple-500/10 text-purple-700 dark:text-purple-300",
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
  set_title("shell.dx-spots");

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
          AlertReason::Watched(p) => tf("radio.watching", &[&(p).to_string()]),
          AlertReason::NewDxcc => t("radio.new-dxcc"),
          AlertReason::NewBand => t("radio.new-band"),
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
      1..=3 => hits
        .iter()
        .for_each(|h| notify(&tf("radio.dx-spot", &[&(h).to_string()]))),
      n => notify(&tf("radio.dx-spot-and-more", &[&hits[0], &n.to_string()])),
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
    crate::util::alert(&tf(
      "radio.logged-mhz",
      &[&(dx).to_string(), &(s).to_string(), (mode)],
    ));
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.dx-spots")
        subtitle=move || t("radio.dx-cluster-global-real")
        actions=ViewFn::from(move || {
          view! {
            <Button
              variant=Variant::Outline
              size=Size::Sm
              loading=loading
              on_click=Callback::new(move |_| load())
            >
              {move || if loading.get() { t("radio.refreshing") } else { t("exam.refresh") }}
            </Button>
          }
        })
      />

      <PageContainer>
        <section class="rounded-xl border bg-card">
          <h2 class="flex items-center justify-between border-b px-4 py-3 text-sm font-semibold">
            {move || t("radio.live-dx-reports")}
            <span class="text-xs font-normal text-muted-foreground">
              {move || tf("radio.entry-2", &[&spots.get().len().to_string()])}
            </span>
          </h2>
          <div class="space-y-2 p-3">
            <div class="flex flex-wrap items-center gap-1.5">
              <span class="mr-1 text-xs text-muted-foreground">{move || t("radio.band")}</span>
              <ChipToggle
                active=Signal::derive(move || band_filter.get().is_none())
                on_change=Callback::new(move |_| band_filter.set(None))
              >
                {move || t("exam.all")}
              </ChipToggle>
              {BANDS
                .iter()
                .map(|b| {
                  let b = *b;
                  view! {
                    <ChipToggle
                      active=Signal::derive(move || band_filter.get() == Some(b))
                      on_change=Callback::new(move |on: bool| {
                        band_filter.set(if on { Some(b) } else { None })
                      })
                    >
                      {b}
                    </ChipToggle>
                  }
                })
                .collect_view()}
            </div>
            <div class="flex flex-wrap items-center gap-1.5">
              <span class="mr-1 text-xs text-muted-foreground">{move || t("log.mode")}</span>
              <ChipToggle
                active=Signal::derive(move || mode_filter.get().is_none())
                on_change=Callback::new(move |_| mode_filter.set(None))
              >
                {move || t("exam.all")}
              </ChipToggle>
              {MODES
                .iter()
                .map(|m| {
                  let m = *m;
                  view! {
                    <ChipToggle
                      active=Signal::derive(move || mode_filter.get() == Some(m))
                      on_change=Callback::new(move |on: bool| {
                        mode_filter.set(if on { Some(m) } else { None })
                      })
                    >
                      {m}
                    </ChipToggle>
                  }
                })
                .collect_view()}
            </div>
            {move || {
              has_log.get().then(|| {
                view! {
                  <div class="flex flex-wrap items-center gap-1.5">
                    <span class="mr-1 text-xs text-muted-foreground">{move || t("radio.log")}</span>
                    <ChipToggle
                      active=needed_only
                      on_change=Callback::new(move |on: bool| needed_only.set(on))
                      title=Signal::derive(move || t("radio.show-only-entities-not"))
                    >
                      {move || tf("radio.needed-only", &[&needed_count.get().to_string()])}
                    </ChipToggle>
                  </div>
                }
              })
            }}
            <div class="flex flex-wrap items-center gap-1.5">
              <span class="mr-1 text-xs text-muted-foreground">{move || t("radio.alerts")}</span>
              <ChipToggle
                active=Signal::derive(move || alerts.with(AlertSettings::enabled))
                on_change=Callback::new(move |_| show_alerts.update(|v| *v = !*v))
              >
                {move || {
                  let a = alerts.get();
                  if a.enabled() {
                    let mut parts = Vec::new();
                    if a.new_dxcc {
                      parts.push(t("radio.new-dxcc"));
                    }
                    if a.new_band {
                      parts.push(t("radio.new-band"));
                    }
                    if !a.calls.is_empty() {
                      parts.push(tf("radio.watched-2", &[&a.calls.len().to_string()]));
                    }
                    tf("radio.on", &[&(parts.join(" · ")).to_string()])
                  } else {
                    t("radio.set-up-alerts")
                  }
                }}
              </ChipToggle>
            </div>
            {move || {
              show_alerts.get().then(|| {
                let toggle = move |label: &'static str, get: fn(&AlertSettings) -> bool, set: fn(&mut AlertSettings, bool)| {
                  view! {
                    <label class="flex items-center gap-2 text-sm">
                      <Checkbox
                        checked=Signal::derive(move || alerts.with(get))
                        on_change=Callback::new(move |on: bool| {
                          update_alerts(&|a| set(a, on));
                        })
                      />
                      {move || t(label)}
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
                      <span class="text-xs text-muted-foreground">{move || t("radio.watched-callsigns-space-or")}</span>
                      <Input
                        value=watch_input
                        on_change=Callback::new(move |v: String| {
                          watch_input.set(v);
                          // 原先是 `on:change`（失焦才解析），`Input` 只有输入即触发的一条通道；
                          // 解析本身是幂等的，每次按键重算一遍不影响结果。
                          let calls: Vec<String> = watch_input
                            .get_untracked()
                            .split([' ', ',', '，'])
                            .map(|s| s.trim().to_uppercase())
                            .filter(|s| !s.is_empty())
                            .collect();
                          update_alerts(&|a| a.calls.clone_from(&calls));
                        })
                        placeholder="VP8* 3Y0J"
                        class="uppercase"
                      />
                    </label>
                    <p class="text-xs text-muted-foreground">
                      {move || t("radio.once-enabled-the-page")}
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
                  <p class="px-3 py-8 text-center text-sm text-muted-foreground">{move || t("radio.fetching-live-spots")}</p>
                }
                .into_any();
              }
              if failed.get() || spots.get().is_empty() {
                return view! {
                  <p class="px-3 py-8 text-center text-sm text-muted-foreground">
                    {move || t("radio.live-spots-are-unavailable")}
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
                  <p class="px-3 py-8 text-center text-sm text-muted-foreground">{move || t("radio.no-reports-for-the")}</p>
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
                        Need::NewDxcc => Some((t("radio.new-dxcc"), "border-emerald-500/40 bg-emerald-500/15 text-emerald-700 dark:text-emerald-300")),
                        Need::NewBand => Some((t("radio.new-band"), "border-sky-500/40 bg-sky-500/15 text-sky-700 dark:text-sky-300")),
                        Need::Worked => Some((t("radio.worked-2"), "text-muted-foreground")),
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
                            title=move || t("radio.click-to-log")
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
                            <span class="shrink-0 rounded border border-amber-500/40 bg-amber-500/15 px-1.5 py-0.5 text-xs font-medium text-amber-700 dark:text-amber-300">{move || t("radio.watched")}</span>
                          })}
                          <span class="min-w-0 flex-1 truncate text-sm text-muted-foreground">
                            {if comment.is_empty() { "—".to_owned() } else { comment.clone() }}
                          </span>
                          <span class="shrink-0 text-xs text-muted-foreground">{time.clone()}</span>
                          <span class="shrink-0 text-xs text-muted-foreground" title=move || t("radio.spotter")>
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
          {move || t("radio.data-comes-from-the")}
        </p>
      </PageContainer>
    </div>
  }
}

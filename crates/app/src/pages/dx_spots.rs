//! DX 实时热点：全球 DX Cluster 实时通联报告，支持按波段 / 模式筛选。

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::icons::{Icon, IconKind};
use crate::pages::log::quick_add;
use crate::ui::{Size, Variant, button_class};
use crate::util::set_title;

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
const CHIP_ON: &str = "rounded-full border bg-primary px-2.5 py-1 text-xs text-primary-foreground";
const CHIP_OFF: &str = "rounded-full border px-2.5 py-1 text-xs hover:bg-accent";

/// 频率 kHz → 业余波段。
fn band_of(khz: u32) -> &'static str {
  match khz {
    1800..=2000 => "160m",
    3500..=4000 => "80m",
    7000..=7300 => "40m",
    10100..=10150 => "30m",
    14000..=14350 => "20m",
    18068..=18168 => "17m",
    21000..=21450 => "15m",
    24890..=24990 => "12m",
    28000..=29700 => "10m",
    50000..=54000 => "6m",
    144000..=148000 => "2m",
    _ => "其他",
  }
}

/// 从备注推断模式。
fn mode_of(comment: &str) -> &'static str {
  let c = comment.to_uppercase();
  if c.contains("FT8") {
    return "FT8";
  }
  if c.contains("FT4") {
    return "FT4";
  }
  if c.contains("RTTY") {
    return "RTTY";
  }
  if c.contains("CW") {
    return "CW";
  }
  if c.contains("SSB") {
    return "SSB";
  }
  if c.contains("PSK") {
    return "PSK";
  }
  "其他"
}

/// 波段 → chip 着色样式。
fn band_color(band: &str) -> &'static str {
  match band {
    "160m" => "border-red-500/30 bg-red-500/10 text-red-500",
    "80m" => "border-orange-500/30 bg-orange-500/10 text-orange-500",
    "40m" => "border-amber-500/30 bg-amber-500/10 text-amber-600",
    "30m" => "border-yellow-500/30 bg-yellow-500/10 text-yellow-600",
    "20m" => "border-green-500/30 bg-green-500/10 text-green-600",
    "17m" => "border-emerald-500/30 bg-emerald-500/10 text-emerald-600",
    "15m" => "border-teal-500/30 bg-teal-500/10 text-teal-600",
    "12m" => "border-sky-500/30 bg-sky-500/10 text-sky-600",
    "10m" => "border-blue-500/30 bg-blue-500/10 text-blue-600",
    "6m" => "border-violet-500/30 bg-violet-500/10 text-violet-600",
    "2m" => "border-purple-500/30 bg-purple-500/10 text-purple-600",
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

  let spots = RwSignal::new(Vec::<Spot>::new());
  let loading = RwSignal::new(true);
  let failed = RwSignal::new(false);
  let band_filter = RwSignal::new(None::<&'static str>);
  let mode_filter = RwSignal::new(None::<&'static str>);

  let load = move || {
    loading.set(true);
    failed.set(false);
    spawn_local(async move {
      match data::fetch_external_json::<Vec<Spot>>("/api/spots").await {
        Ok(s) => spots.set(s),
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };

  load();

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
    quick_add(&dx, &s, mode);
    crate::util::alert(&format!("已加入日志：{dx}（{s} MHz {mode}）"));
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"DX 实时热点"</div>
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
              let filtered: Vec<Spot> = list
                .into_iter()
                .filter(|s| {
                  band.is_none_or(|b| band_of(s.freq_khz) == b)
                    && mode.is_none_or(|m| mode_of(&s.comment) == m)
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
                    .map(|s| {
                      let freq = s.freq_khz;
                      let dx = s.dx.clone();
                      let country = s.country.clone();
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

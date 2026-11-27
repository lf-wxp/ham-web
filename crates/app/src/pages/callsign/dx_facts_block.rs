//! 这条 DX 值不值得叫：方位与距离、这一小时的可靠度、路径与晨昏圈、日志里的新实体 / 新波段、
//! 已保存的 NEC 设计在该方位的增益——拼成一块，只在 `/callsign` 的结果区出现一次。
//!
//! 只**摆事实、不合成总分**：每一项独立给出，缺它需要的输入就留空并说明缺什么，不猜。
//! 事实的汇总在核心 `ham_web_core::dx_facts`（有单测）；NEC 增益用核心的 `antenna_coverage::gain_toward`，
//! 与 `/nec` 的方向表是同一个数。

use std::time::Duration;

use ham_web_core::antenna_coverage::{GainMode, PatternSlices, gain_toward};
use ham_web_core::dx_facts::facts;
use ham_web_core::nec::solve_with_lines;
use ham_web_core::nec_designs::{DESIGNS_KEY, DesignBook};
use ham_web_core::watch_windows::When;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::ui::{Button, SelectOption, SimpleSelect, Size, Variant, card_title_class};
use crate::util::{sleep, unique_id};

/// `/api/solar` 里本块要用的一项。
#[derive(Deserialize, Clone)]
struct SolarApi {
  sunspots: Option<i32>,
}

/// 此刻的 UTC 日期与小时。
fn now_utc() -> When {
  let d = js_sys::Date::new_0();
  When {
    year: d.get_utc_full_year(),
    month: d.get_utc_month() + 1,
    day: d.get_utc_date(),
    utc_hour: f64::from(d.get_utc_hours()) + f64::from(d.get_utc_minutes()) / 60.0,
  }
}

/// 一次 NEC 增益计算的结果。
#[derive(Clone, PartialEq)]
enum NecOutcome {
  /// 设计名、频率（MHz）、方位、增益（dBi）、相对峰值的损失（dB）。
  Gain {
    name: String,
    freq_mhz: f64,
    bearing: f64,
    gain: f64,
    loss: f64,
  },
  /// 这份设计解不出来。
  Unsolvable,
}

#[component]
pub(super) fn DxFactsBlock(
  /// 当前查询的呼号（已大写）。
  call: Signal<String>,
  /// 本台网格（可能为空）。
  home_grid: Signal<String>,
) -> impl IntoView {
  let store = use_log_store();
  let when = RwSignal::new(now_utc());
  let solar = RwSignal::new(None::<SolarApi>);
  if let Ok(handle) = set_interval_with_handle(move || when.set(now_utc()), Duration::from_secs(30))
  {
    on_cleanup(move || handle.clear());
  }
  let refresh = move || {
    spawn_local(async move {
      if let Ok(api) = data::fetch_external_json::<SolarApi>("/api/solar").await {
        solar.try_set(Some(api));
      }
    });
  };
  refresh();
  if let Ok(handle) = set_interval_with_handle(refresh, Duration::from_secs(60)) {
    on_cleanup(move || handle.clear());
  }

  let ssn = Memo::new(move |_| {
    solar
      .with(|s| s.as_ref().and_then(|s| s.sunspots))
      .map(f64::from)
  });
  let found = Memo::new(move |_| {
    store.logbook.with(|lb| {
      facts(
        &call.get(),
        &home_grid.get(),
        &lb.entries,
        &when.get(),
        ssn.get(),
      )
    })
  });

  // 已保存的 NEC 设计：挂载时读一次（`/nec` 在另一个路由，保存之后回到这里会重新挂载）。
  let book = crate::kv::load_snapshot(DESIGNS_KEY)
    .and_then(|json| DesignBook::parse(&json))
    .unwrap_or_default();
  let options: Vec<SelectOption> = book
    .designs
    .iter()
    .map(|d| SelectOption::new(d.id.to_string(), d.name.clone()))
    .collect();
  let picked = RwSignal::new(
    book
      .designs
      .first()
      .map(|d| d.id.to_string())
      .unwrap_or_default(),
  );
  let book = StoredValue::new(book);
  let outcome = RwSignal::new(None::<NecOutcome>);
  let busy = RwSignal::new(false);
  let select_id = unique_id("dx-nec-design");
  // 换呼号或换设计：旧的增益不再对得上。
  Effect::new(move |_| {
    call.track();
    picked.track();
    outcome.set(None);
  });

  let compute = Callback::new(move |()| {
    let Some(bearing) = found.with_untracked(|f| f.path.map(|p| p.bearing_deg)) else {
      return;
    };
    let id: u64 = picked.get_untracked().parse().unwrap_or(0);
    busy.set(true);
    outcome.set(None);
    spawn_local(async move {
      // 让「计算中…」先画出来：求解是同步的，会占住主线程一小会儿。
      sleep(30).await;
      let result = book.with_value(|b| {
        let d = b.get(id)?;
        let (input, lines) = d.model.to_input();
        let freq_mhz = input.freq_hz / 1_000_000.0;
        let solved = solve_with_lines(&input, &lines)?;
        let (gain, loss) = gain_toward(
          &PatternSlices::from(&solved),
          bearing,
          GainMode::BestElevation,
        )?;
        Some(NecOutcome::Gain {
          name: d.name.clone(),
          freq_mhz,
          bearing,
          gain,
          loss,
        })
      });
      outcome.try_set(Some(result.unwrap_or(NecOutcome::Unsolvable)));
      busy.try_set(false);
    });
  });

  view! {
    <div class="space-y-3 border-t px-4 py-4" data-testid="dx-facts">
      <h3 class=card_title_class("")>{move || t("radio.dxf-title")}</h3>
      <p class="text-xs text-muted-foreground">{move || t("radio.dxf-intro")}</p>
      {move || {
        let f = found.get();
        let Some(entity) = f.entity else {
          return view! { <p class="text-sm text-muted-foreground" data-testid="dxf-no-entity">{t("radio.dxf-no-entity")}</p> }
            .into_any();
        };
        let blank_home = move || {
          view! { <span class="text-muted-foreground">{t("radio.fill-home-grid")}</span> }
        };
        view! {
          <dl class="space-y-2 text-sm">
            <div class="flex flex-wrap gap-x-3" data-testid="dxf-bearing">
              <dt class="w-32 shrink-0 text-xs text-muted-foreground">{t("radio.dxf-bearing")}</dt>
              <dd class="min-w-0">
                {match f.path {
                  Some(p) => {
                    view! {
                      <span class="font-mono tabular-nums">
                        {tf("radio.dxf-bearing-value", &[&format!("{:.0}", p.bearing_deg), &format!("{:.0}", p.distance_km)])}
                      </span>
                      <span class="ml-2 text-xs text-muted-foreground">{tf("radio.dxf-coarse", &[entity.name_en])}</span>
                    }
                      .into_any()
                  }
                  None => blank_home().into_any(),
                }}
              </dd>
            </div>

            <div class="flex flex-wrap gap-x-3" data-testid="dxf-reliability">
              <dt class="w-32 shrink-0 text-xs text-muted-foreground">{t("radio.dxf-reliability")}</dt>
              <dd class="min-w-0 space-y-1">
                {match (&f.reliability, f.path) {
                  (Some(rows), _) => {
                    let w = when.get();
                    let stamp = format!(
                      "UTC {:02}:00 · {:04}-{:02}-{:02} · SSN {}",
                      w.utc_hour.floor() as u32,
                      w.year,
                      w.month,
                      w.day,
                      ssn.get().map_or_else(|| "—".to_owned(), |s| format!("{s:.0}"))
                    );
                    view! {
                      <div class="text-xs text-muted-foreground">{stamp}</div>
                      <div class="flex flex-wrap gap-1.5">
                        {rows
                          .iter()
                          .map(|b| {
                            let pct = (b.reliability * 100.0).round() as u32;
                            view! {
                              <span
                                class="border px-1.5 py-0.5 font-mono text-xs tabular-nums"
                                data-band=b.band
                                data-reliability=pct.to_string()
                              >
                                {format!("{} {pct}%", b.band)}
                              </span>
                            }
                          })
                          .collect_view()}
                      </div>
                    }
                      .into_any()
                  }
                  (None, None) => blank_home().into_any(),
                  (None, Some(_)) => {
                    view! { <span class="text-muted-foreground">{t("radio.dxf-no-ssn")}</span> }.into_any()
                  }
                }}
              </dd>
            </div>

            <div class="flex flex-wrap gap-x-3" data-testid="dxf-terminator">
              <dt class="w-32 shrink-0 text-xs text-muted-foreground">{t("radio.dxf-terminator")}</dt>
              <dd class="min-w-0">
                {match f.terminator {
                  Some(tm) => {
                    let share = format!("{:.0}", tm.share_near * 100.0);
                    let min_km = format!("{:.0}", tm.min_km);
                    view! {
                      <span data-along=tm.along.to_string()>
                        {if tm.along {
                          tf("radio.dxf-terminator-along", &[&share, &min_km])
                        } else {
                          tf("radio.dxf-terminator-not", &[&share, &min_km])
                        }}
                      </span>
                    }
                      .into_any()
                  }
                  None => blank_home().into_any(),
                }}
              </dd>
            </div>

            <div class="flex flex-wrap gap-x-3" data-testid="dxf-standing">
              <dt class="w-32 shrink-0 text-xs text-muted-foreground">{t("radio.dxf-standing")}</dt>
              <dd class="min-w-0 space-y-0.5">
                {match &f.standing {
                  None => {
                    view! { <span class="text-muted-foreground">{t("radio.dxf-empty-log")}</span> }.into_any()
                  }
                  Some(s) => {
                    let new_bands = s.new_bands.join(" ");
                    let worked_bands = s.worked_bands.join(" ");
                    view! {
                      <div data-new-entity=s.new_entity.to_string()>
                        {if s.new_entity { t("radio.dxf-new-entity") } else { t("radio.dxf-worked-entity") }}
                      </div>
                      {(!s.new_bands.is_empty())
                        .then(|| {
                          view! {
                            <div data-testid="dxf-new-bands">{tf("radio.dxf-new-bands", &[&new_bands])}</div>
                          }
                        })}
                      {(!s.worked_bands.is_empty())
                        .then(|| {
                          view! {
                            <div data-testid="dxf-worked-bands">{tf("radio.dxf-worked-bands", &[&worked_bands])}</div>
                          }
                        })}
                    }
                      .into_any()
                  }
                }}
              </dd>
            </div>

            <div class="flex flex-wrap gap-x-3" data-testid="dxf-nec">
              <dt class="w-32 shrink-0 text-xs text-muted-foreground">{t("radio.dxf-nec")}</dt>
              <dd class="min-w-0 space-y-1.5">
                {if options.is_empty() {
                  view! {
                    <span class="text-muted-foreground">
                      {t("radio.dxf-no-design")}
                      <a href="/nec" class="ml-1 text-primary underline underline-offset-2">{t("radio.dxf-open-nec")}</a>
                    </span>
                  }
                    .into_any()
                } else if f.path.is_none() {
                  blank_home().into_any()
                } else {
                  view! {
                    <div class="flex flex-wrap items-center gap-2">
                      <SimpleSelect
                        id=select_id.clone()
                        value=picked
                        on_change=Callback::new(move |v: String| picked.set(v))
                        options=options.clone()
                        aria_label=Signal::derive(move || t("radio.dxf-nec"))
                      />
                      // 按钮文案很长（西语「Calcular la ganancia en esa dirección」）：基类是定高 + 不换行，
                      // 手机宽度下它自己就比容器宽，所以两套皮肤都允许换行。
                      <Button
                        variant=Variant::Outline
                        size=Size::Sm
                        class="md-btn-wrap h-auto max-w-full whitespace-normal py-1.5 text-left"
                        disabled=Signal::derive(move || busy.get())
                        on_click=compute
                      >
                        {move || if busy.get() { t("radio.dxf-computing") } else { t("radio.dxf-compute") }}
                      </Button>
                    </div>
                    {move || {
                      outcome
                        .get()
                        .map(|o| match o {
                          NecOutcome::Gain { name, freq_mhz, bearing, gain, loss } => {
                            view! {
                              <div data-testid="dxf-nec-result" data-gain=format!("{gain:.2}") data-loss=format!("{loss:.2}")>
                                {tf(
                                  "radio.dxf-nec-result",
                                  &[
                                    &name,
                                    &format!("{freq_mhz:.3}"),
                                    &format!("{bearing:.0}"),
                                    &format!("{gain:.1}"),
                                    &format!("{loss:.1}"),
                                  ],
                                )}
                              </div>
                            }
                              .into_any()
                          }
                          NecOutcome::Unsolvable => {
                            view! { <div data-testid="dxf-nec-result" class="text-hp-text">{t("radio.dxf-unsolvable")}</div> }
                              .into_any()
                          }
                        })
                    }}
                  }
                    .into_any()
                }}
              </dd>
            </div>
          </dl>
        }
          .into_any()
      }}
      <p class="text-xs text-muted-foreground">{move || t("radio.dxf-note")}</p>
    </div>
  }
}

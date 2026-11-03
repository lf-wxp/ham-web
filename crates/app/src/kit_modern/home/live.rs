//! 首页「实时传播」：一次 `/api/solar` 拿到 SFI / A / K、传播等级与各波段条件。
//!
//! 为什么放在首页而不是只留 `/solar`：传播条件是业余无线电最有「现在是活的」感的读数，
//! 而首页此前没有实时数据（像素版有传播卡，现代版一直缺）。只接**一个**接口，
//! 并且复用 `fetch_external_json` 的 5 分钟离线缓存 —— 首页不该为了一块读数打三次外部请求。
//!
//! 时段（白天 / 夜间）按本地小时判断：HamQSL 给的是 day / night 两份条件表，
//! 首页只显示当前那份，另一份留在 `/solar`。

use ham_web_core::solar::{
  BandCondition, PropagationLevel, condition_label_key, condition_tone, live_level, select_bands,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::data;
use crate::data::core_text_tracked;
use crate::i18n::{t, tf};

use super::live_band::LiveBand;
use super::live_metric::LiveMetric;

/// 服务端 `/api/solar` 的返回结构（只用首页要的字段）。
#[derive(Deserialize, Clone)]
struct SolarApi {
  #[serde(default)]
  updated: Option<String>,
  solar_flux: Option<i32>,
  a_index: Option<i32>,
  k_index: Option<i32>,
  sunspots: Option<i32>,
  #[serde(default)]
  conditions: Vec<BandCondition>,
}

/// 当前时段（6:00–18:00 算白天）。
///
/// 只决定「首页显示哪一张条件表」，不参与任何计算：真正的传播预测在 `/solar`，
/// 那里两张表都列出来。
fn current_period() -> &'static str {
  if (6..18).contains(&js_sys::Date::new_0().get_hours()) {
    "day"
  } else {
    "night"
  }
}

#[component]
pub(super) fn Live() -> impl IntoView {
  let sfi = RwSignal::new(None::<i32>);
  let a = RwSignal::new(None::<i32>);
  let k = RwSignal::new(None::<i32>);
  let ssn = RwSignal::new(None::<i32>);
  let updated = RwSignal::new(String::new());
  let conditions = RwSignal::new(Vec::<BandCondition>::new());
  let loading = RwSignal::new(true);
  let failed = RwSignal::new(false);
  let period = RwSignal::new(current_period());
  // 波段表由「全量条件 + 当前时段」派生：换时段只换这张表，不重挂组件、不重新请求。
  let bands = Memo::new(move |_| select_bands(&conditions.get(), period.get()));

  let load = move || {
    loading.set(true);
    failed.set(false);
    spawn_local(async move {
      match data::fetch_external_json::<SolarApi>("/api/solar").await {
        Ok(api) => {
          sfi.set(api.solar_flux);
          a.set(api.a_index);
          k.set(api.k_index);
          ssn.set(api.sunspots);
          updated.set(api.updated.unwrap_or_default());
          conditions.set(api.conditions);
        }
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };
  load();

  // 首页常在标签页里挂一整天：跨过 18:00 要换表。每分钟重算一次本地小时，
  // 变了才写信号（写同一个值也会惊动订阅方重算，没必要）。
  if let Ok(handle) = leptos::leptos_dom::helpers::set_interval_with_handle(
    move || {
      let now = current_period();
      if period.get_untracked() != now {
        period.set(now);
      }
    },
    std::time::Duration::from_secs(60),
  ) {
    on_cleanup(move || handle.clear());
  }

  // 等级只由 K / SSN / SFI 决定（与 `/solar`、像素版传播卡同一套规则）；
  // 决定结论的指数缺失时显示「—」而不是编一个等级（规则与单测在 core 的 `live_level`）。
  let level = move || {
    let Some(lvl) = live_level(k.get(), ssn.get(), sfi.get()) else {
      return view! {
        <div class="md-live-level" data-tone="unknown">
          <span class="md-led" aria-hidden="true"></span>
          <span class="md-live-level-text">"—"</span>
        </div>
      }
      .into_any();
    };
    let tone = match lvl {
      PropagationLevel::Excellent | PropagationLevel::Good => "good",
      PropagationLevel::Fair => "warn",
      PropagationLevel::Poor => "muted",
    };
    view! {
      <div class="md-live-level" data-tone=tone>
        <span class="md-led" aria-hidden="true"></span>
        <span class="md-live-level-text">{core_text_tracked(lvl.label())}</span>
      </div>
    }
    .into_any()
  };

  view! {
    <section aria-labelledby="home-live-title">
      <div class="md-sect-head">
        <h2 id="home-live-title" class="md-h2">{move || t("home.live-propagation")}</h2>
        <span class="md-sect-note">
          {move || if period.get() == "day" { t("radio.day") } else { t("radio.night") }}
          " · HamQSL"
        </span>
      </div>

      {move || {
        failed
          .get()
          .then(|| {
            view! {
              <div class="md-live-failed">
                <span>{move || t("home.live-unavailable")}</span>
                <div class="md-live-failed-actions">
                  <button class="md-btn md-btn-quiet md-btn-sm" on:click=move |_| load()>
                    {move || t("exam.retry")}
                  </button>
                  <a class="md-btn md-btn-quiet md-btn-sm" href="/solar">
                    {move || t("home.details")}
                  </a>
                </div>
              </div>
            }
          })
      }}

      {move || {
        (!failed.get())
          .then(|| {
            view! {
              // `aria-busy`：读数从「—」变成真值时读屏能知道这一次更新已经结束。
              <div class="md-live" aria-busy=move || loading.get().to_string()>
                <div class="md-live-metrics">
                  <LiveMetric label="home.solar-flux" value=Signal::from(sfi) />
                  <LiveMetric label="radio.a-index" value=Signal::from(a) />
                  <LiveMetric label="home.k-index" value=Signal::from(k) />
                  {level}
                </div>

                {move || {
                  let rows = bands.get();
                  (!rows.is_empty())
                    .then(|| {
                      view! {
                        <div
                          class="md-live-belt"
                          role="group"
                          aria-label=Signal::derive(|| t("radio.band-conditions"))
                        >
                          {rows
                            .into_iter()
                            .map(|b| {
                              let tone = condition_tone(&b.condition);
                              let label = condition_label_key(&b.condition);
                              view! { <LiveBand band=b.band tone=tone label=label /> }
                            })
                            .collect_view()}
                        </div>
                      }
                    })
                }}

                <div class="md-live-source">
                  {move || {
                    let u = updated.get();
                    if u.is_empty() {
                      "HamQSL".to_owned()
                    } else {
                      format!("HamQSL · {}", tf("home.live-updated", &[&u]))
                    }
                  }}
                </div>
              </div>
            }
          })
      }}
    </section>
  }
}

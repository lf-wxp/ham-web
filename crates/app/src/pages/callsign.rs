//! 呼号查询：本地离线解析国家 / 地区、DXCC 实体与分区、稀有度，并可选在线补全姓名、QTH 与网格。

use ham_web_core::award_progress::CONTINENTS;
use ham_web_core::callsign::parse_callsign;
use ham_web_core::dxcc::lookup;
use ham_web_core::logbook::StationInfo;
use ham_web_core::most_wanted::wanted_prefix;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;

use crate::components::common::{BulletSection, KnowledgePage};
use crate::data;
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::pages::map::ZoneMap;
use crate::ui::{Button, Chip, ChipGroup, Input, Size, Variant};
use crate::util::{set_title, storage};

/// 示例呼号，便于一键试查。
const EXAMPLES: &[&str] = &["BG4XYZ", "JA1ABC", "K1ZZ/QRP", "P5ABC", "BV9PAA"];

/// 使用提示。
const USAGE_TIPS: &[&str] = &[
  "本地解析完全离线：不联网也能查国家 / 地区、DXCC 实体、CQ / ITU 分区与稀有度。",
  "「在线查询」再补全姓名、QTH 与网格，数据来自 Callook（美加）与 HamQTH（国际），纯静态托管时不可用。",
  "在地址后加上 `?call=BA1XX` 可分享查询结果，打开页面即自动发起一次在线查询。",
  "解析后可对照「DXCC 稀有度榜单」判断追台价值，或到「通联日志」录入该呼号。",
];

/// `/api/callsign` 返回的操作员公开资料。
#[derive(serde::Deserialize, Clone)]
struct OnlineInfo {
  #[serde(default)]
  name: String,
  #[serde(default)]
  grid: String,
  #[serde(default)]
  qth: String,
  #[serde(default)]
  country: String,
  #[serde(default)]
  source: String,
}

#[component]
pub fn CallsignPage() -> impl IntoView {
  set_title("tools.callsign-lookup");

  let query = use_query_map();
  let input = RwSignal::new(query.with_untracked(|q| q.get("call")).unwrap_or_default());
  let online = RwSignal::new(None::<OnlineInfo>);
  let loading = RwSignal::new(false);
  let failed = RwSignal::new(false);
  // 自己的日志与本台网格：查询结果里要回答「我打过吗、在哪个方向、多远」。
  let store = use_log_store();
  // 网格用信号存并在查询时重读：站信息页改完之后，本页不该继续用挂载时的旧值算方位。
  let home_grid = RwSignal::new(
    storage::get_json::<StationInfo>("station-info")
      .map(|s| s.gridsquare)
      .unwrap_or_default(),
  );
  // 分区地图：默认 CQ，高亮当前呼号所属实体。
  let is_cq = RwSignal::new(true);
  let zone_dxcc = Signal::derive(move || {
    let call = input.get().trim().to_ascii_uppercase();
    if call.is_empty() {
      return None;
    }
    lookup(&call).map(|e| e.dxcc)
  });

  let run_online = move || {
    let call = input.get().trim().to_ascii_uppercase();
    if call.len() < 3 {
      crate::ui::push_error(t("radio.enter-a-callsign-of"));
      return;
    }
    home_grid.set(
      storage::get_json::<StationInfo>("station-info")
        .map(|s| s.gridsquare)
        .unwrap_or_default(),
    );
    loading.set(true);
    failed.set(false);
    online.set(None);
    spawn_local(async move {
      let url = format!(
        "/api/callsign?callsign={}",
        js_sys::encode_uri_component(&call)
      );
      match data::fetch_external_json::<OnlineInfo>(&url).await {
        Ok(info) => online.set(Some(info)),
        Err(_) => failed.set(true),
      }
      loading.set(false);
    });
  };

  // 带 `?call=` 打开时自动查询一次在线资料。
  Effect::new(move |_| {
    if !input.get_untracked().trim().is_empty() {
      run_online();
    }
  });

  // 本地解析结果：`(标签, 值)`。
  let rows = move || -> Vec<(&'static str, String)> {
    let call = input.get().trim().to_ascii_uppercase();
    if call.is_empty() {
      return Vec::new();
    }
    let parts = parse_callsign(&call);
    let entity = lookup(&call);
    let dash = "—".to_owned();

    // 通联历史与方位都交给 core（`callsign_insight`），页面只管排版。
    let found = store
      .logbook
      .with(|lb| ham_web_core::callsign_insight::insight(&call, &home_grid.get(), &lb.entries));
    let history = if found.worked == 0 {
      t("radio.never-worked").to_string()
    } else {
      match &found.last {
        Some(last) => tf(
          "radio.worked-times-last",
          &[
            &found.worked.to_string(),
            &format!("{} {} {}", last.date, last.time, last.band),
          ],
        ),
        None => tf("radio.worked-times", &[&found.worked.to_string()]),
      }
    };
    // 没有本台网格时只答「打过几次」，方位与距离如实留空。
    let (bearing, distance) = match (found.azimuth_deg, found.distance_km) {
      (Some(azimuth), Some(km)) => (format!("{azimuth:.0}°"), format!("{km:.0} km")),
      _ => (t("radio.fill-home-grid").to_string(), dash.clone()),
    };

    // 标签返回**语义 key**，渲染时再 `t()`：这几行原来直接写中文，英文界面下整列都是中文。
    vec![
      (
        "radio.country-region",
        parts
          .entity
          .map(t)
          .unwrap_or_else(|| t("radio.unrecognised")),
      ),
      (
        "log.dxcc-entities",
        entity.map_or_else(
          || t("radio.unrecognised"),
          |e| format!("#{} {}", e.dxcc, e.name_en),
        ),
      ),
      (
        "radio.continent",
        entity
          .and_then(|e| CONTINENTS.iter().find(|(c, _)| *c == e.continent))
          .map_or_else(|| dash.clone(), |(_, name)| t(name)),
      ),
      (
        "radio.cq-itu-zones",
        entity.map_or_else(|| dash.clone(), |e| format!("CQ {} / ITU {}", e.cq, e.itu)),
      ),
      (
        "radio.station-class",
        parts
          .station_type
          .map(t)
          .unwrap_or_else(|| t("radio.non-chinese-callsign")),
      ),
      (
        "radio.zone-label",
        match (parts.area, parts.area_regions) {
          (Some(a), Some(r)) => tf("common.zone-with-regions", &[a, r]),
          _ => dash.clone(),
        },
      ),
      (
        "radio.suffix",
        if parts.suffix.is_empty() {
          dash.clone()
        } else {
          parts.suffix.clone()
        },
      ),
      (
        "radio.slash-suffix",
        parts.slash.map(t).unwrap_or_else(|| dash.clone()),
      ),
      (
        "radio.rarity",
        wanted_prefix(&call).map_or_else(
          || t("radio.regular-entity"),
          |p| tf("radio.most-wanted-entity", &[p]),
        ),
      ),
      ("radio.my-log", history),
      ("radio.azimuth-from-home", bearing),
      ("radio.distance-from-home", distance),
    ]
  };

  view! {
    <KnowledgePage title=t("tools.callsign-lookup") subtitle=t("radio.offline-parsing-dxcc-entity")>
      <section class="pxl-card border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.callsign-lookup")}</h2>
        <p class="px-4 pt-3 text-xs text-muted-foreground">
          {move || t("radio.enter-a-callsign-to")}
        </p>

        <div class="flex flex-wrap items-center gap-3 p-4">
          <Input
            value=input
            on_change=Callback::new(move |v: String| input.set(v.to_uppercase()))
            placeholder=Signal::derive(move || t("radio.e-g-bg4xyz-ja1abc-p5abc"))
            aria_label=Signal::derive(move || t("log.callsign"))
            class="max-w-xs font-mono uppercase"
          />
          <Button
            variant=Variant::Default
            size=Size::Default
            on_click=Callback::new(move |_| run_online())
          >
            {move || if loading.get() { t("log.looking-up") } else { t("radio.online-lookup") }}
          </Button>
          <div class="flex flex-wrap items-center gap-1.5">
            {EXAMPLES
              .iter()
              .map(|&ex| {
                view! {
                  <button
                    type="button"
                    class="border px-2.5 py-1 font-mono text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                    on:click=move |_| {
                      input.set(ex.to_owned());
                      run_online();
                    }
                  >
                    {ex}
                  </button>
                }
              })
              .collect_view()}
          </div>
        </div>

        {move || {
          let list = rows();
          if list.is_empty() {
            return view! {
              <p class="border-t px-4 py-4 text-sm text-muted-foreground">
                {move || t("radio.enter-a-callsign-to-begin")}
              </p>
            }
            .into_any();
          }
          view! {
            <div class="grid gap-1 border-t p-4 sm:grid-cols-2">
              {list
                .into_iter()
                .map(|(label, value)| {
                  view! {
                    <div class="flex items-baseline gap-2 px-3 py-2">
                      <span class="w-28 shrink-0 text-xs text-muted-foreground">
                        {move || t(label)}
                      </span>
                      <span class="text-sm font-medium">{value}</span>
                    </div>
                  }
                })
                .collect_view()}
            </div>
          }
          .into_any()
        }}

        {move || {
          if failed.get() {
            return view! {
              <p class="border-t px-4 py-3 text-xs text-muted-foreground">
                {move || t("radio.online-data-is-unavailable")}
              </p>
            }
            .into_any();
          }
          online
            .get()
            .map(|info| {
              let fields: Vec<(&'static str, String)> = vec![
                ("姓名", info.name.clone()),
                ("QTH", info.qth.clone()),
                ("网格", info.grid.clone()),
                ("国家", info.country.clone()),
                ("数据来源", info.source.clone()),
              ]
                .into_iter()
                .filter(|(_, v)| !v.trim().is_empty())
                .collect();
              view! {
                <div class="space-y-2 border-t px-4 py-4">
                  <div class="text-xs font-semibold text-muted-foreground">{move || t("radio.online-data")}</div>
                  <div class="grid gap-1 sm:grid-cols-2">
                    {fields
                      .into_iter()
                      .map(|(label, value)| {
                        view! {
                          <div class="flex items-baseline gap-2">
                            <span class="w-20 shrink-0 text-xs text-muted-foreground">{label}</span>
                            <span class="text-sm font-medium">{value}</span>
                          </div>
                        }
                      })
                      .collect_view()}
                  </div>
                  {(info.source == "DXCC")
                    .then(|| {
                      view! {
                        <p class="text-xs text-muted-foreground">
                          {move || t("radio.no-public-operator-data")}
                        </p>
                      }
                    })}
                </div>
              }
              .into_any()
            })
            .into_any()
        }}

        <p class="border-t px-4 py-3 text-xs text-muted-foreground">
          {move || t("radio.offline-parsing-uses-the")}
        </p>
      </section>

      <section class="pxl-card border bg-card">
        <div class="flex flex-wrap items-center gap-3 border-b px-4 py-3">
          <h2 class="mr-auto text-sm font-semibold">{move || t("radio.zone-map-cq-itu")}</h2>
          <ChipGroup
            value=Signal::derive(move || (if is_cq.get() { "cq" } else { "itu" }).to_string())
            on_change=Callback::new(move |v: String| is_cq.set(v == "cq"))
            aria_label=Signal::derive(move || t("radio.zone-map-cq-itu"))
          >
            {
              let cq = view! { {move || t("common.cq-zone")} }.into_any();
              let itu = view! { {move || t("common.itu-zone")} }.into_any();
              view! {
                <Chip value="cq".to_string()>{cq}</Chip>
                <Chip value="itu".to_string()>{itu}</Chip>
              }
            }
          </ChipGroup>
        </div>
        <div class="p-4">
          <ZoneMap is_cq=is_cq highlight=zone_dxcc />
        </div>
        <p class="border-t px-4 py-3 text-xs text-muted-foreground">
          {move || t("radio.coloured-by-each-dxcc")}
          <a
            href=move || {
              let call = input.get().trim().to_ascii_uppercase();
              if call.is_empty() {
                "/zone-map".to_owned()
              } else {
                format!("/zone-map?q={}", js_sys::encode_uri_component(&call))
              }
            }
            class="ml-1 font-medium text-primary underline underline-offset-2"
          >
            {move || t("radio.open-the-full-zone")}
          </a>
        </p>
      </section>

      <BulletSection title="怎么用" items=USAGE_TIPS ui=true />
    </KnowledgePage>
  }
}

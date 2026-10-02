//! SOTA / POTA 地图：在世界地图上标出本地日志已激活的山峰 / 公园。

use std::collections::BTreeSet;

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::pages::map::{MapView, project};
use crate::util::set_title;

/// SOTA 山峰详情（/api/sota）。
#[derive(Deserialize, Clone)]
struct SummitInfo {
  #[serde(default)]
  reference: String,
  #[serde(default)]
  name: String,
  latitude: f64,
  longitude: f64,
}

/// POTA 公园详情（/api/pota）。
#[derive(Deserialize, Clone)]
struct ParkInfo {
  #[serde(default)]
  reference: String,
  #[serde(default)]
  name: String,
  latitude: f64,
  longitude: f64,
}

/// 地图上的一个标记点。
#[derive(Clone)]
struct Marker {
  lat: f64,
  lon: f64,
  label: String,
}

#[component]
pub fn PortableMapPage() -> impl IntoView {
  set_title(&t("SOTA / POTA 地图"));

  let store = use_log_store();
  let markers = RwSignal::new(Vec::<Marker>::new());
  let loading = RwSignal::new(true);

  // 从日志提取去重后的 SOTA / POTA 编号。
  let (sota_refs, pota_refs) = {
    let entries = store.logbook.get_untracked().entries;
    let mut sota = BTreeSet::new();
    let mut pota = BTreeSet::new();
    for e in &entries {
      if !e.sota_ref.trim().is_empty() {
        sota.insert(e.sota_ref.trim().to_ascii_uppercase());
      }
      if !e.pota_ref.trim().is_empty() {
        pota.insert(e.pota_ref.trim().to_ascii_uppercase());
      }
    }
    (sota, pota)
  };

  spawn_local(async move {
    let mut out: Vec<Marker> = Vec::new();
    for r in &sota_refs {
      if let Ok(s) = data::fetch_external_json::<SummitInfo>(&format!("/api/sota?ref={r}")).await {
        out.push(Marker {
          lat: s.latitude,
          lon: s.longitude,
          label: format!("{} · {}", s.reference, s.name),
        });
      }
    }
    for r in &pota_refs {
      if let Ok(p) = data::fetch_external_json::<ParkInfo>(&format!("/api/pota?ref={r}")).await {
        out.push(Marker {
          lat: p.latitude,
          lon: p.longitude,
          label: format!("{} · {}", p.reference, p.name),
        });
      }
    }
    markers.set(out);
    loading.set(false);
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=Signal::derive(move || t("SOTA / POTA 地图"))
        subtitle=Signal::derive(move || t("已激活的山峰与公园分布"))
        actions=ViewFn::from(move || {
          view! {
            <span class="rounded-full border px-3 py-1 text-xs text-muted-foreground">
              {move || tf("已激活 {} 个", &[&markers.get().len().to_string()])}
            </span>
          }
        })
      />
      <PageContainer>
        <section class="rounded-xl border bg-card p-4">
          {move || {
            if loading.get() {
              view! {
                <div class="py-12 text-center text-sm text-muted-foreground">{move || t("加载中…")}</div>
              }
              .into_any()
            } else if markers.get().is_empty() {
              view! {
                <div class="py-12 text-center text-sm text-muted-foreground">
                  {move || t("暂无 SOTA / POTA 激活记录，请在通联日志中添加带 SOTA / POTA 编号的记录。")}
                </div>
              }
              .into_any()
            } else {
              view! {
                <MapView>
                  {markers
                    .get()
                    .iter()
                    .map(|m| {
                      let (x, y) = project(m.lon, m.lat);
                      view! {
                        <circle
                          cx=x.to_string()
                          cy=y.to_string()
                          r="3"
                          class="fill-red-500 stroke-red-500/60"
                          stroke-width="1"
                          vector-effect="non-scaling-stroke"
                        >
                          <title>{m.label.clone()}</title>
                        </circle>
                      }
                    })
                    .collect_view()}
                </MapView>
              }
              .into_any()
            }
          }}
          <p class="mt-2 text-xs text-muted-foreground">
            {move || t("红点为你已激活的 SOTA 山峰 / POTA 公园；位置来自 SOTA / POTA API 查询，需通过后端（dev-full / serve）访问。")}
          </p>
        </section>
      </PageContainer>
    </div>
  }
}

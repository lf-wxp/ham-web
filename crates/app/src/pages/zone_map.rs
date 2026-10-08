//! CQ / ITU 分区地图：按分区着色世界地图，可定位呼号 / 实体并查看分区构成。

use ham_web_core::dxcc;
use ham_web_core::zone::{
  CQ_ZONE_MAX, ITU_ZONE_MAX, ZONE_CONCEPTS, ZONE_TIPS, cq_zone_counts, entities_in_cq,
  entities_in_itu, itu_zone_counts, zone_color,
};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::{t, tf, tp};
use crate::pages::map::ZoneMap;
use crate::ui::{Button, Input, Size, Variant};
use crate::util::{alert, set_title};

#[component]
pub fn ZoneMapPage() -> impl IntoView {
  set_title("radio.cq-itu-zone-map");

  let url_query = use_query_map();
  let is_cq = RwSignal::new(true);
  let query = RwSignal::new(url_query.with_untracked(|q| q.get("q")).unwrap_or_default());
  let hit = RwSignal::new(None::<u16>);
  let selected = RwSignal::new(None::<u8>);
  // 仅由「定位」驱动视图居中，避免每次点击区域都跳动。
  let focus_target = RwSignal::new(None::<(f64, f64)>);

  // 制式切换或命中实体变化时，同步选中分区。
  Effect::new(move |_| {
    let cq = is_cq.get();
    if let Some(id) = hit.get()
      && let Some(e) = dxcc::entity_by_dxcc(id)
    {
      selected.set(Some(if cq { e.cq } else { e.itu }));
    }
  });

  let resolve = move || {
    let q = query.get().trim().to_owned();
    if q.is_empty() {
      hit.set(None);
      selected.set(None);
      return;
    }
    match dxcc::lookup(&q).or_else(|| dxcc::entity_by_name(&q)) {
      Some(e) => {
        hit.set(Some(e.dxcc));
        focus_target.set(Some((e.lat, e.lon)));
      }
      None => {
        hit.set(None);
        alert(&t("radio.unrecognised-callsign-or-entity"));
      }
    }
  };

  // 带 ?q= 打开时自动定位一次（从呼号查询页跳转过来的联动）。
  if !query.get_untracked().trim().is_empty() {
    resolve();
  }

  let highlight = Signal::derive(move || hit.get());
  let max_zone = Signal::derive(move || {
    if is_cq.get() {
      CQ_ZONE_MAX
    } else {
      ITU_ZONE_MAX
    }
  });
  let counts = Memo::new(move |_| {
    if is_cq.get() {
      cq_zone_counts()
    } else {
      itu_zone_counts()
    }
  });

  let on_region_click = Callback::new(move |key: String| {
    if let Ok(id) = key.parse::<u16>() {
      hit.set(Some(id));
    }
  });

  view! {
    <KnowledgePage title=t("radio.cq-itu-zone-map") subtitle=t("radio.colour-by-zone-locate")>
      <section class="rounded-xl border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.zone-map")}</h2>
        <div class="flex flex-wrap items-center gap-3 p-4">
          <div class="flex rounded-lg border p-0.5">
            <button
              type="button"
              class=move || {
                if is_cq.get() {
                  "rounded-md bg-primary px-3 py-1 text-xs font-medium text-primary-foreground"
                } else {
                  "rounded-md px-3 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
                }
              }
              on:click=move |_| is_cq.set(true)
            >
              {move || t("radio.cq-zones-40")}
            </button>
            <button
              type="button"
              class=move || {
                if is_cq.get() {
                  "rounded-md px-3 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
                } else {
                  "rounded-md bg-primary px-3 py-1 text-xs font-medium text-primary-foreground"
                }
              }
              on:click=move |_| is_cq.set(false)
            >
              {move || t("radio.itu-zones-90")}
            </button>
          </div>
          <Input
            value=query
            on_change=Callback::new(move |v: String| query.set(v))
            on_enter=Callback::new(move |()| resolve())
            placeholder=Signal::derive(move || t("radio.callsign-or-entity-name-2"))
            aria_label=Signal::derive(move || t("radio.callsign-or-entity-name"))
            class="max-w-xs"
          />
          <Button
            variant=Variant::Default
            size=Size::Default
            on_click=Callback::new(move |_| resolve())
          >
            {move || t("log.locate")}
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Default
            on_click=Callback::new(move |_| {
                        query.set(String::new());
                        hit.set(None);
                        selected.set(None);
                        focus_target.set(None);
                      })
          >
            {move || t("exam.clear")}
          </Button>
          {move || {
            hit.get()
              .and_then(dxcc::entity_by_dxcc)
              .map(|e| {
                view! {
                  <span class="rounded-full border border-amber-500 bg-amber-500/10 px-3 py-1 text-xs">
                    {tf("radio.located", &[e.name])}
                  </span>
                }
              })
          }}
        </div>
        <div class="px-4 pb-4">
          <ZoneMap
            is_cq=is_cq
            highlight=highlight
            on_region_click=on_region_click
            focus=focus_target
          />
        </div>
        <p class="border-t px-4 py-3 text-xs text-muted-foreground">
          {move || t("radio.coloured-by-each-dxcc-2")}
        </p>
      </section>

      <section class="rounded-xl border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">
          {move || tf("radio.legend-zones-the-number", &[&max_zone.get().to_string()])}
        </h2>
        <div class="flex flex-wrap gap-1.5 p-4">
          {move || {
            let max = max_zone.get();
            counts
              .get()
              .into_iter()
              .map(|(z, n)| {
                let color = zone_color(z, max);
                view! {
                  <button
                    type="button"
                    class=move || {
                      if selected.get() == Some(z) {
                        "flex items-center gap-1.5 rounded-full border border-primary px-2 py-0.5 text-xs"
                      } else {
                        "flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-xs transition-colors hover:bg-accent"
                      }
                    }
                    on:click=move |_| selected.set(Some(z))
                  >
                    <span class="inline-block h-3 w-3 rounded-sm" style=format!("background-color: {color}")></span>
                    <span class="tabular-nums">{format!("{z} ({n})")}</span>
                  </button>
                }
              })
              .collect_view()
          }}
        </div>
      </section>

      <section class="rounded-xl border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("radio.zone-composition")}</h2>
        {move || {
          let Some(z) = selected.get() else {
            return view! {
              <p class="px-4 py-4 text-sm text-muted-foreground">
                {move || t("radio.click-an-area-of")}
              </p>
            }
            .into_any();
          };
          let cq = is_cq.get();
          let list = if cq {
            entities_in_cq(z)
          } else {
            entities_in_itu(z)
          };
          let label = if cq { t("common.cq") } else { t("common.itu") };
          let n = list.len() as u32;
          view! {
            <div class="border-t">
              <div class="flex flex-wrap items-baseline gap-x-3 px-4 py-3">
                <span class="text-sm font-semibold">
                  {tf("radio.zone", &[&label, &z.to_string()])}
                </span>
                <span class="text-xs text-muted-foreground">
                  {tp("radio.dxcc-entities", n, &[&n.to_string()])}
                </span>
              </div>
              {if list.is_empty() {
                view! {
                  <p class="px-4 pb-4 text-sm text-muted-foreground">
                    {move || t("radio.no-dxcc-entities-registered")}
                  </p>
                }
                .into_any()
              } else {
                view! {
                  <div class="flex flex-wrap gap-1.5 px-4 pb-4">
                    {list
                      .into_iter()
                      .map(|e| {
                        let id = e.dxcc;
                        let active = hit.get() == Some(id);
                        view! {
                          <button
                            type="button"
                            class=if active {
                              "rounded-full border border-amber-500 bg-amber-500/10 px-2.5 py-1 text-xs"
                            } else {
                              "rounded-full border px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                            }
                            on:click=move |_| hit.set(Some(id))
                          >
                            {e.name}
                          </button>
                        }
                      })
                      .collect_view()}
                  </div>
                }
                .into_any()
              }}
            </div>
          }
          .into_any()
        }}
      </section>

      <ConceptsSection title="核心概念" items=ZONE_CONCEPTS />
      <BulletSection title="使用要点" items=ZONE_TIPS />
    </KnowledgePage>
  }
}

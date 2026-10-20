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
use crate::ui::{Button, Chip, ChipGroup, Input, Size, Variant};
use crate::util::set_title;

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
        crate::ui::push_error(t("radio.unrecognised-callsign-or-entity"));
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
      <section class="pxl-card border bg-card">
        <h2 class="pxl-section-title">{move || t("radio.zone-map")}</h2>
        <div class="flex flex-wrap items-center gap-3 p-4">
          <ChipGroup
            value=Signal::derive(move || (if is_cq.get() { "cq" } else { "itu" }).to_string())
            on_change=Callback::new(move |v: String| is_cq.set(v == "cq"))
            aria_label=Signal::derive(move || t("radio.zone-map-cq-itu"))
          >
            {
              let cq = view! { {move || t("radio.cq-zones-40")} }.into_any();
              let itu = view! { {move || t("radio.itu-zones-90")} }.into_any();
              view! {
                <Chip value="cq".to_string()>{cq}</Chip>
                <Chip value="itu".to_string()>{itu}</Chip>
              }
            }
          </ChipGroup>
          <Input
            value=query
            on_change=Callback::new(move |v: String| query.set(v))
            on_enter=Callback::new(move |()| resolve())
            placeholder=Signal::derive(move || t("radio.callsign-or-entity-placeholder"))
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
                  <span class="border border-amber-500 bg-amber-500/10 px-3 py-1 text-xs">
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
          {move || t("radio.coloured-by-primary-zone")}
        </p>
      </section>

      <section class="pxl-card border bg-card">
        <h2 class="pxl-section-title">
          {move || tf("radio.legend-zones-the-number", &[&max_zone.get().to_string()])}
        </h2>
        <ChipGroup
          value=Signal::derive(move || selected.get().map(|z| z.to_string()).unwrap_or_default())
          on_change=Callback::new(move |v: String| selected.set(v.parse::<u8>().ok()))
          class="gap-1.5 p-4"
        >
          {move || {
            let max = max_zone.get();
            counts
              .get()
              .into_iter()
              .map(|(z, n)| {
                let color = zone_color(z, max);
                let label = view! {
                  <span class="inline-block h-3 w-3" style=format!("background-color: {color}")></span>
                  <span class="tabular-nums">{format!("{z} ({n})")}</span>
                }
                  .into_any();
                view! { <Chip value=z.to_string()>{label}</Chip> }
              })
              .collect_view()
          }}
        </ChipGroup>
      </section>

      <section class="pxl-card border bg-card">
        <h2 class="pxl-section-title">{move || t("radio.zone-composition")}</h2>
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
                  <ChipGroup
                    value=Signal::derive(move || hit.get().map(|i| i.to_string()).unwrap_or_default())
                    on_change=Callback::new(move |v: String| hit.set(v.parse::<u16>().ok()))
                    class="gap-1.5 px-4 pb-4"
                  >
                    {list
                      .into_iter()
                      .map(|e| {
                        let id = e.dxcc;
                        let label = view! { {e.name} }.into_any();
                        view! { <Chip value=id.to_string()>{label}</Chip> }
                      })
                      .collect_view()}
                  </ChipGroup>
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

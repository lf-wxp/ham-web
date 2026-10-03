//! CQ / ITU 分区地图：按分区着色世界地图，可定位呼号 / 实体并查看分区构成。

use ham_web_core::dxcc;
use ham_web_core::zone::{
  CQ_ZONE_MAX, ITU_ZONE_MAX, ZONE_CONCEPTS, ZONE_TIPS, cq_zone_counts, entities_in_cq,
  entities_in_itu, itu_zone_counts, zone_color,
};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::components::common::{BulletSection, ConceptsSection, KnowledgePage};
use crate::i18n::{t, tf};
use crate::pages::map::ZoneMap;
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{alert, set_title};

#[component]
pub fn ZoneMapPage() -> impl IntoView {
  set_title(&t("CQ / ITU 分区地图"));

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
        alert(&t("未识别的呼号或实体名，请检查拼写。"));
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
    <KnowledgePage title=t("CQ / ITU 分区地图") subtitle=t("按分区着色 · 呼号定位 · 分区构成")>
      <section class="rounded-xl border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("分区地图")}</h2>
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
              {move || t("CQ 分区（40）")}
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
              {move || t("ITU 分区（90）")}
            </button>
          </div>
          <input
            type="text"
            placeholder=move || t("呼号或实体名（如 BG4XYZ、日本）")
            aria-label=move || t("呼号或实体名")
            prop:value=move || query.get()
            on:input=move |e| query.set(event_target_value(&e))
            on:keydown=move |e: web_sys::KeyboardEvent| {
              if e.key() == "Enter" {
                resolve();
              }
            }
            class=input_class("max-w-xs")
          />
          <button
            type="button"
            class=button_class(Variant::Default, Size::Default, "")
            on:click=move |_| resolve()
          >
            {move || t("定位")}
          </button>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Default, "")
            on:click=move |_| {
              query.set(String::new());
              hit.set(None);
              selected.set(None);
              focus_target.set(None);
            }
          >
            {move || t("清除")}
          </button>
          {move || {
            hit.get()
              .and_then(dxcc::entity_by_dxcc)
              .map(|e| {
                view! {
                  <span class="rounded-full border border-amber-500 bg-amber-500/10 px-3 py-1 text-xs">
                    {tf("已定位：{}", &[e.name])}
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
          {move || t("按每个 DXCC 实体的主分区着色：俄罗斯、美国、中国等横跨多个分区的大国会被整体归入一个分区，因此本图用于「分区大致在哪、含哪些实体」的速查，并非精确的分区边界。")}
        </p>
      </section>

      <section class="rounded-xl border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">
          {move || tf("图例（共 {} 个分区，括号内为所含 DXCC 实体数）", &[&max_zone.get().to_string()])}
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
        <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("分区构成")}</h2>
        {move || {
          let Some(z) = selected.get() else {
            return view! {
              <p class="px-4 py-4 text-sm text-muted-foreground">
                {move || t("点击地图区域或图例，或输入呼号定位，查看该分区包含的 DXCC 实体。")}
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
          let label = if cq { t("CQ") } else { t("ITU") };
          let count = list.len().to_string();
          view! {
            <div class="border-t">
              <div class="flex flex-wrap items-baseline gap-x-3 px-4 py-3">
                <span class="text-sm font-semibold">
                  {tf("{} {} 区", &[&label, &z.to_string()])}
                </span>
                <span class="text-xs text-muted-foreground">
                  {tf("含 {} 个 DXCC 实体", &[&count])}
                </span>
              </div>
              {if list.is_empty() {
                view! {
                  <p class="px-4 pb-4 text-sm text-muted-foreground">
                    {move || t("该分区暂无登记的 DXCC 实体。")}
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

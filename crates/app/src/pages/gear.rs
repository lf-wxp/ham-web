//! 设备评测与选购（阶段一）：精选机型库、按用途推荐与多机参数对比。

use ham_web_core::gear::{GEAR_CATEGORIES, GEAR_PICKS, GEAR_TIPS, Gear, in_category};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, TableSection};
use crate::i18n::{t, tf};
use crate::util::{alert, set_title};

/// 对比表最多同时对比的机型数。
const MAX_COMPARE: usize = 4;

/// 推荐表表头。
const PICK_HEADERS: &[&str] = &["用途", "推荐机型", "理由"];

/// 某类别默认选中的机型（前 3 台）。
fn default_selection(category: &str) -> Vec<String> {
  in_category(category)
    .iter()
    .take(3)
    .map(|g| g.id.to_owned())
    .collect()
}

/// 对比表的行：`(属性, 每台机型对应的取值)`。
fn comparison_rows(models: &[&Gear]) -> Vec<(&'static str, Vec<String>)> {
  if models.is_empty() {
    return Vec::new();
  }
  let collect = |f: fn(&Gear) -> String| models.iter().map(|g| f(g)).collect::<Vec<_>>();
  vec![
    ("品牌", collect(|g| g.brand.to_owned())),
    ("价格档", collect(|g| g.tier.to_owned())),
    ("频段", collect(|g| g.bands.to_owned())),
    ("功率", collect(|g| g.power.to_owned())),
    ("模式", collect(|g| g.modes.to_owned())),
    ("特点", collect(|g| g.highlight.to_owned())),
    ("点评", collect(|g| g.note.to_owned())),
  ]
}

#[component]
pub fn GearPage() -> impl IntoView {
  set_title("knowledge.gear-reviews-buying-guide");

  let category = RwSignal::new(GEAR_CATEGORIES[0].0.to_owned());
  let selected = RwSignal::new(default_selection(GEAR_CATEGORIES[0].0));

  let switch_category = move |key: &str| {
    selected.set(default_selection(key));
    category.set(key.to_owned());
  };

  let toggle = move |id: &str| {
    let mut too_many = false;
    selected.update(|v| {
      if let Some(pos) = v.iter().position(|x| x == id) {
        v.remove(pos);
      } else if v.len() >= MAX_COMPARE {
        too_many = true;
      } else {
        v.push(id.to_owned());
      }
    });
    if too_many {
      let limit = MAX_COMPARE.to_string();
      alert(&tf("knowledge.you-can-compare-at", &[&limit]));
    }
  };

  view! {
    <KnowledgePage title=t("knowledge.gear-reviews-buying-guide") subtitle=t("knowledge.featured-radios-spec-comparison")>
      <TableSection
        title="按用途推荐"
        headers=PICK_HEADERS
        rows=GEAR_PICKS
        min_width=720
      />

      <section class="rounded-xl border bg-card">
        <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("knowledge.spec-comparison")}</h2>
        <p class="px-4 pt-3 text-xs text-muted-foreground">
          {move || t("knowledge.pick-a-category-and")}
        </p>

        <div class="flex flex-wrap gap-1.5 p-4 pb-2">
          {GEAR_CATEGORIES
            .iter()
            .map(|&(key, label)| {
              view! {
                <button
                  type="button"
                  class=move || {
                    if category.get() == key {
                      "rounded-full border border-primary bg-primary/10 px-3 py-1 text-xs font-medium text-primary"
                    } else {
                      "rounded-full border px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                    }
                  }
                  on:click=move |_| switch_category(key)
                >
                  {label}
                </button>
              }
            })
            .collect_view()}
        </div>

        <div class="flex flex-wrap gap-1.5 px-4 pb-4">
          {move || {
            let cat = category.get();
            in_category(&cat)
              .into_iter()
              .map(|g| {
                let id = g.id;
                view! {
                  <button
                    type="button"
                    class=move || {
                      if selected.get().iter().any(|x| x == id) {
                        "rounded-full border border-primary bg-primary/10 px-3 py-1 text-xs font-medium text-primary"
                      } else {
                        "rounded-full border px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                      }
                    }
                    on:click=move |_| toggle(id)
                  >
                    {g.model}
                  </button>
                }
              })
              .collect_view()
          }}
        </div>

        {move || {
          let sel = selected.get();
          let models: Vec<&Gear> = in_category(&category.get())
            .into_iter()
            .filter(|g| sel.iter().any(|id| id == g.id))
            .collect();
          if models.is_empty() {
            return view! {
              <p class="border-t px-4 py-4 text-sm text-muted-foreground">
                {move || t("knowledge.select-at-least-one")}
              </p>
            }
            .into_any();
          }
          let rows = comparison_rows(&models);
          view! {
            <div class="overflow-x-auto border-t">
              <table class="w-full border-collapse text-sm" style="min-width: 640px">
                <thead class="bg-muted/60 text-xs">
                  <tr>
                    <th class="border px-3 py-2 text-left">{move || t("knowledge.specs")}</th>
                    {models
                      .iter()
                      .map(|g| {
                        view! {
                          <th class="border px-3 py-2 text-left">
                            <div class="font-semibold">{g.brand}</div>
                            <div class="font-normal text-muted-foreground">{g.model}</div>
                          </th>
                        }
                      })
                      .collect_view()}
                  </tr>
                </thead>
                <tbody>
                  {rows
                    .into_iter()
                    .map(|(label, values)| {
                      view! {
                        <tr class="border-t transition-colors hover:bg-muted/40">
                          <td class="border px-3 py-2 text-left align-top font-medium whitespace-nowrap">
                            {move || t(label)}
                          </td>
                          {values
                            .into_iter()
                            .map(|value| {
                              view! {
                                <td class="border px-3 py-2 text-left align-top text-muted-foreground">
                                  {value}
                                </td>
                              }
                            })
                            .collect_view()}
                        </tr>
                      }
                    })
                    .collect_view()}
                </tbody>
              </table>
            </div>
          }
          .into_any()
        }}

        <p class="border-t px-4 py-3 text-xs text-muted-foreground">
          {move || t("knowledge.this-table-compiles-objective")}
        </p>
      </section>

      <BulletSection title="选购要点" items=GEAR_TIPS />
    </KnowledgePage>
  }
}

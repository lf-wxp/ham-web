//! DIY 实战项目：结构化卡片 + 筛选 + 制作进度（本地存储）。

use ham_web_core::diy_projects::{
  COST_CHECKED_ON, Cost, DIY_PROJECTS, DIY_SAFETY, DIY_STEPS, Difficulty, DiyBand, DiyFilters,
  DiyProgressBook, Instrument,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, KnowledgePage, StepsSection};
use crate::data;
use crate::i18n::{t, tf};
use crate::ui::{Checkbox, NativeSelect, SelectOption};
use crate::util::{set_title, unique_id};

/// 制作进度的存储 key（只存本机）。
const PROGRESS_KEY: &str = "diy-progress";

/// 难度 → 界面文案（字面量 key 写在调用点）。
fn difficulty_label(d: Difficulty) -> String {
  match d {
    Difficulty::Beginner => t("diy.difficulty-beginner"),
    Difficulty::Advanced => t("diy.difficulty-advanced"),
  }
}

/// 成本档位 → 界面文案。
fn cost_label(c: Cost) -> String {
  match c {
    Cost::Low => t("diy.cost-low"),
    Cost::Mid => t("diy.cost-mid"),
  }
}

/// 波段 → 界面文案。
fn band_label(b: DiyBand) -> String {
  match b {
    DiyBand::Hf => t("diy.band-hf"),
    DiyBand::VhfUhf => t("diy.band-vhf-uhf"),
    DiyBand::Other => t("diy.band-other"),
  }
}

/// 仪表 → 界面文案。
fn instrument_label(i: Instrument) -> String {
  match i {
    Instrument::SwrMeter => t("diy.instrument-swr"),
    Instrument::Vna => t("diy.instrument-vna"),
    Instrument::Multimeter => t("diy.instrument-multimeter"),
  }
}

/// 知识库正文译文：先订阅加载状态（切语言后要重算），再查词典。
fn text(zh: &'static str) -> String {
  data::track_knowledge();
  data::kt(zh)
}

/// 筛选下拉的公共选项：`全部 + 各档位`。
///
/// 标签用信号：选项列表只在组件构建时生成一次，传现成的字符串会让下拉里的文案停在
/// 首次渲染的语言上。
fn select_options(items: Vec<(&'static str, Signal<String>)>) -> Vec<SelectOption> {
  let mut out = vec![SelectOption::new(
    "all",
    Signal::derive(move || t("exam.all")),
  )];
  out.extend(
    items
      .into_iter()
      .map(|(key, label)| SelectOption::new(key, label)),
  );
  out
}

#[component]
pub fn DiyProjectsPage() -> impl IntoView {
  set_title("knowledge.hands-on-diy-projects");
  let filters = RwSignal::new(DiyFilters::default());
  // 制作进度：只存本机。
  let progress = RwSignal::new(
    crate::util::storage::get_json::<DiyProgressBook>(PROGRESS_KEY).unwrap_or_default(),
  );
  let save_progress = Callback::new(move |book: DiyProgressBook| {
    // 一个勾都没有就删掉 key，不在存储里留空壳。
    if book.is_empty() {
      crate::util::storage::remove(PROGRESS_KEY);
    } else {
      crate::util::storage::set_json(PROGRESS_KEY, &book);
    }
  });

  let visible = Memo::new(move |_| {
    DIY_PROJECTS
      .iter()
      .filter(|p| filters.get().matches(p))
      .collect::<Vec<_>>()
  });

  view! {
    <KnowledgePage title=t("knowledge.hands-on-diy-project") subtitle=t("knowledge.hands-on-projects-from")>
      // ── 筛选：难度 / 成本 / 波段 / 所需仪表 ────────────────────
      <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <NativeSelect
          value=Signal::derive(move || {
            filters
              .with(|f| match f.difficulty {
                None => "all".to_owned(),
                Some(Difficulty::Beginner) => "beginner".to_owned(),
                Some(Difficulty::Advanced) => "advanced".to_owned(),
              })
          })
          on_change=Callback::new(move |v: String| {
            filters.update(|f| {
              f.difficulty = match v.as_str() {
                "beginner" => Some(Difficulty::Beginner),
                "advanced" => Some(Difficulty::Advanced),
                _ => None,
              };
            });
          })
          options=select_options(
            vec![
              ("beginner", Signal::derive(move || difficulty_label(Difficulty::Beginner))),
              ("advanced", Signal::derive(move || difficulty_label(Difficulty::Advanced))),
            ],
          )
          // 「难度」词典里已有（exam.difficulty），同词只能复用一个。
          aria_label=Signal::derive(move || t("exam.difficulty"))
          class="w-full"
        />
        <NativeSelect
          value=Signal::derive(move || {
            filters
              .with(|f| match f.cost {
                None => "all".to_owned(),
                Some(Cost::Low) => "low".to_owned(),
                Some(Cost::Mid) => "mid".to_owned(),
              })
          })
          on_change=Callback::new(move |v: String| {
            filters.update(|f| {
              f.cost = match v.as_str() {
                "low" => Some(Cost::Low),
                "mid" => Some(Cost::Mid),
                _ => None,
              };
            });
          })
          options=select_options(
            vec![
              ("low", Signal::derive(move || cost_label(Cost::Low))),
              ("mid", Signal::derive(move || cost_label(Cost::Mid))),
            ],
          )
          aria_label=Signal::derive(move || t("diy.cost"))
          class="w-full"
        />
        <NativeSelect
          value=Signal::derive(move || {
            filters
              .with(|f| match f.band {
                None => "all".to_owned(),
                Some(DiyBand::Hf) => "hf".to_owned(),
                Some(DiyBand::VhfUhf) => "vhf-uhf".to_owned(),
                Some(DiyBand::Other) => "other".to_owned(),
              })
          })
          on_change=Callback::new(move |v: String| {
            filters.update(|f| {
              f.band = match v.as_str() {
                "hf" => Some(DiyBand::Hf),
                "vhf-uhf" => Some(DiyBand::VhfUhf),
                "other" => Some(DiyBand::Other),
                _ => None,
              };
            });
          })
          options=select_options(
            vec![
              ("hf", Signal::derive(move || band_label(DiyBand::Hf))),
              ("vhf-uhf", Signal::derive(move || band_label(DiyBand::VhfUhf))),
              ("other", Signal::derive(move || band_label(DiyBand::Other))),
            ],
          )
          aria_label=Signal::derive(move || t("knowledge.band"))
          class="w-full"
        />
        <NativeSelect
          value=Signal::derive(move || {
            filters
              .with(|f| match f.instrument {
                None => "all".to_owned(),
                Some(Instrument::SwrMeter) => "swr".to_owned(),
                Some(Instrument::Vna) => "vna".to_owned(),
                Some(Instrument::Multimeter) => "multimeter".to_owned(),
              })
          })
          on_change=Callback::new(move |v: String| {
            filters.update(|f| {
              f.instrument = match v.as_str() {
                "swr" => Some(Instrument::SwrMeter),
                "vna" => Some(Instrument::Vna),
                "multimeter" => Some(Instrument::Multimeter),
                _ => None,
              };
            });
          })
          options=select_options(
            vec![
              ("swr", Signal::derive(move || instrument_label(Instrument::SwrMeter))),
              ("vna", Signal::derive(move || instrument_label(Instrument::Vna))),
              ("multimeter", Signal::derive(move || instrument_label(Instrument::Multimeter))),
            ],
          )
          aria_label=Signal::derive(move || t("diy.instruments"))
          class="w-full"
        />
      </div>

      <p class="text-xs text-muted-foreground">
        {move || tf("diy.cost-note", &[COST_CHECKED_ON])}
        " "
        {move || t("diy.progress-local")}
      </p>

      // ── 项目卡片 ──────────────────────────────────────────
      <div class="grid gap-4 md:grid-cols-2">
        {move || {
          let projects = visible.get();
          projects
            .into_iter()
            .map(|p| {
              let name = p.name;
              let done_id = unique_id("diy-done");
              let bought_ids: Vec<(usize, String)> = p
                .bom
                .iter()
                .enumerate()
                .map(|(i, (_, _))| (i, unique_id(&format!("diy-buy-{i}"))))
                .collect();
              view! {
                <article class="rounded-xl border bg-card">
                  <header class="flex flex-wrap items-baseline gap-2 border-b px-4 py-3">
                    <h2 class="text-sm font-semibold">{move || text(name)}</h2>
                    <span class="text-xs text-muted-foreground">
                      {move || difficulty_label(p.difficulty)}
                      " · "
                      {move || cost_label(p.cost)}
                      " · "
                      {move || tf("diy.hours", &[text(p.hours).as_str()])}
                    </span>
                  </header>

                  <div class="space-y-3 px-4 py-3">
                    <p class="text-sm text-muted-foreground">{move || text(p.notes)}</p>

                    <div class="flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
                      <span>{move || t("knowledge.band")}</span>
                      <span class="rounded-full border px-2 py-0.5">{move || band_label(p.band)}</span>
                      {p.instruments.iter().map(|i| {
                        let i = *i;
                        view! {
                          <span class="rounded-full border px-2 py-0.5" title=move || t("diy.instruments")>
                            {move || instrument_label(i)}
                          </span>
                        }
                      }).collect_view()}
                    </div>

                    // BOM：每行带「已采购」勾选（只存本机）。
                    <div>
                      <h3 class="text-xs font-semibold text-primary">{move || t("diy.bom")}</h3>
                      <ul class="mt-1 divide-y">
                        {p
                          .bom
                          .iter()
                          .enumerate()
                          .map(|(i, (item, how))| {
                            let item = *item;
                            let how = *how;
                            let bought_id = bought_ids
                              .iter()
                              .find(|(idx, _)| *idx == i)
                              .map(|(_, id)| id.clone())
                              .unwrap_or_default();
                            view! {
                              <li class="flex items-center gap-2 py-1 text-sm">
                                <Checkbox
                                  id=bought_id
                                  checked=Signal::derive(move || {
                                    progress.with(|b| b.is_bought(name, item))
                                  })
                                  on_change=Callback::new(move |v: bool| {
                                    progress.update(|b| {
                                      if b.is_bought(name, item) != v {
                                        b.toggle_bought(name, item);
                                      }
                                    });
                                    save_progress.run(progress.get_untracked());
                                  })
                                  aria_label=Signal::derive(move || {
                                    format!("{}：{}", t("diy.bom-bought"), text(item))
                                  })
                                />
                                <span class="font-medium">{move || text(item)}</span>
                                <span class="text-muted-foreground">{move || text(how)}</span>
                              </li>
                            }
                          })
                          .collect_view()}
                      </ul>
                    </div>

                    // 资料链接（没有公认出处的项目这一栏留空）。
                    {(!p.links.is_empty())
                      .then(|| {
                        view! {
                          <div>
                            <h3 class="text-xs font-semibold text-primary">{move || t("diy.links")}</h3>
                            <ul class="mt-1 space-y-0.5 text-sm">
                              {p.links.iter().copied().map(|(label, url)| {
                                let external = url.starts_with("https://");
                                view! {
                                  <li>
                                    <a
                                      href=url
                                      target=external.then_some("_blank")
                                      rel=external.then_some("noreferrer")
                                      class="text-primary underline-offset-2 hover:underline"
                                    >
                                      {move || text(label)}
                                    </a>
                                  </li>
                                }
                              }).collect_view()}
                            </ul>
                          </div>
                        }
                      })}

                    // 测试步骤。
                    <div>
                      <h3 class="text-xs font-semibold text-primary">{move || t("diy.tests")}</h3>
                      <ol class="mt-1 list-decimal space-y-0.5 pl-5 text-sm text-muted-foreground">
                        {p.tests.iter().map(|step| {
                          let step = *step;
                          view! { <li>{move || text(step)}</li> }
                        }).collect_view()}
                      </ol>
                    </div>

                    // 常见问题。
                    <div>
                      <h3 class="text-xs font-semibold text-primary">{move || t("diy.pitfalls")}</h3>
                      <ul class="mt-1 space-y-0.5 text-sm text-muted-foreground">
                        {p.pitfalls.iter().map(|tip| {
                          let tip = *tip;
                          view! {
                            <li class="flex gap-2">
                              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                              <span>{move || text(tip)}</span>
                            </li>
                          }
                        }).collect_view()}
                      </ul>
                    </div>

                    <label class="flex items-center gap-2 border-t pt-2 text-sm">
                      <Checkbox
                        id=done_id
                        checked=Signal::derive(move || progress.with(|b| b.is_done(name)))
                        on_change=Callback::new(move |v: bool| {
                          progress.update(|b| {
                            if b.is_done(name) != v {
                              b.toggle_done(name);
                            }
                          });
                          save_progress.run(progress.get_untracked());
                        })
                        aria_label=Signal::derive(move || t("diy.done"))
                      />
                      <span class="text-muted-foreground">{move || t("diy.done")}</span>
                    </label>
                  </div>
                </article>
              }
            })
            .collect_view()
        }}
      </div>

      <StepsSection title="通用制作流程" items=DIY_STEPS />
      <BulletSection title="制作安全提醒" items=DIY_SAFETY />
    </KnowledgePage>
  }
}

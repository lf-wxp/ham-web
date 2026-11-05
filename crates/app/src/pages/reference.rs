//! 考试速查页：操作证权限、分区号、RST 信号报告、发射类别标识、
//! 50Ω 电平换算、功率与 dBm 对照、单位前缀、通联英语短句。

use ham_web_core::reference::{
  CALL_AREAS, EMISSION_DESIGNATION, EMISSION_TYPES, KNOWLEDGE_SOURCES, LEVEL_CONVERSIONS,
  LICENSE_CLASSES, PHRASES, POWER_DBM_TABLE, RST_SCALES, UNIT_PREFIXES, call_area_color,
};
use leptos::prelude::*;

use crate::components::call_area_map::CallAreaMap;
use crate::components::common::{PageContainer, PageHeader, TableSection};
use crate::data;
use crate::data::kt_tracked;
use crate::i18n::{t, tf};
use crate::ui::{Chip, ChipGroup, Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

#[component]
pub fn ReferencePage() -> impl IntoView {
  set_title("shell.exam-reference-title");
  // 分区号与省名是知识库正文（`crates/core` 的数据表），译文按模块维护；本页不是
  // `KnowledgePage`，译文要自己挂一次，否则英文 / 西语界面里这一节仍是中文。
  data::use_knowledge_i18n();
  // 地图与色块共用同一个选区：`None` 表示「不分分区，全部同色」。
  let selected = RwSignal::new(None::<String>);
  // 点已选中的分区（或「全部」传空串）→ 取消选择。
  let toggle_area = Callback::new(move |area: String| {
    selected.update(|cur| {
      *cur = if area.is_empty() || cur.as_deref() == Some(area.as_str()) {
        None
      } else {
        Some(area.clone())
      };
    });
  });
  if crate::kit::is_modern_at_mount() {
    return view! {
      <div class="md-root">
        <div class="md-page md-article-page">
          <crate::kit_modern::ModernPageHead
            title=Signal::derive(move || t("shell.exam-reference-title"))
            subtitle=Signal::derive(move || t("exam.licence-privileges-zones-rst"))
          />

          <div class="md-article">
            <section class="md-panel">
              <h2 class="md-panel-title">{move || t("exam.operator-licence-classes-and")}</h2>
              <div class="md-table-scroll">
                <table class="md-table">
                  <thead>
                    <tr>
                      <th>{move || t("exam.class-column")}</th>
                      <th>{move || t("exam.frequency-range")}</th>
                      <th>{move || t("exam.power-limit")}</th>
                      <th>{move || t("radio.notes")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {LICENSE_CLASSES
                      .iter()
                      .map(|c| {
                        view! {
                          <tr>
                            <th scope="row" class="md-mono">{move || tf("learning.class", &[c.class])}</th>
                            <td>{move || kt_tracked(c.freq)}</td>
                            <td>{move || kt_tracked(c.power)}</td>
                            <td class="md-muted">{move || kt_tracked(c.note)}</td>
                          </tr>
                        }
                      })
                      .collect_view()}
                  </tbody>
                </table>
              </div>
            </section>

            <section class="md-panel">
              <h2 class="md-panel-title">{move || t("exam.amateur-radio-zone-numbers")}</h2>
              <p class="md-muted md-sm">{move || t("exam.call-area-map-hint")}</p>
              <div class="mt-3">
                <CallAreaMap selected=selected on_select=toggle_area />
              </div>
              <div class="md-chip-row mt-3">
                {CALL_AREAS
                  .iter()
                  .map(|a| {
                    let digit = a.digit.to_string();
                    view! {
                      <button
                        type="button"
                        class="md-chip"
                        aria-pressed=move || (selected.get().as_deref() == Some(a.digit)).to_string()
                        on:click=move |_| toggle_area.run(digit.clone())
                      >
                        <span class="md-area-dot" style=format!("background: {}", call_area_color(a.digit))></span>
                        <span class="md-mono">{a.digit}</span>
                      </button>
                    }
                  })
                  .collect_view()}
                <button
                  type="button"
                  class="md-chip"
                  aria-pressed=move || selected.get().is_none().to_string()
                  on:click=move |_| toggle_area.run(String::new())
                >
                  {move || t("exam.all")}
                </button>
              </div>
              <div class="md-area-grid">
                {move || {
                  let sel = selected.get();
                  CALL_AREAS
                    .iter()
                    .map(|a| {
                      let active = sel.as_deref() == Some(a.digit);
                      view! {
                        <div class="md-area-cell" data-active=active.to_string()>
                          <span class="md-area-digit">{a.digit}</span>
                          <span class="md-muted md-sm">{move || data::kt_tracked(a.regions)}</span>
                        </div>
                      }
                    })
                    .collect_view()
                }}
              </div>
            </section>

            <section class="md-panel">
              <h2 class="md-panel-title">{move || t("shell.rst-report")}</h2>
              <div class="md-rst-grid">
                {RST_SCALES
                  .iter()
                  .map(|s| {
                    view! {
                      <div class="md-rst-card">
                        <div class="md-rst-card-head">
                          <span class="md-mono md-rst-key">{s.key}</span>
                          <span>{move || kt_tracked(s.name)}</span>
                        </div>
                        <p class="md-muted md-sm">{move || kt_tracked(s.note)}</p>
                        <dl class="md-rst-levels">
                          {s
                            .levels
                            .iter()
                            .map(|&(num, desc)| {
                              view! {
                                <div>
                                  <dt class="md-mono">{num}</dt>
                                  <dd class="md-muted">{move || kt_tracked(desc)}</dd>
                                </div>
                              }
                            })
                            .collect_view()}
                        </dl>
                      </div>
                    }
                  })
                  .collect_view()}
              </div>
            </section>

            <section class="md-panel">
              <h2 class="md-panel-title">{move || t("exam.emission-designators")}</h2>
              <div class="md-table-scroll">
                <table class="md-table">
                  <thead>
                    <tr>
                      <th>{move || t("exam.designator")}</th>
                      <th>{move || t("exam.name")}</th>
                      <th>{move || t("radio.notes")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {EMISSION_TYPES
                      .iter()
                      .map(|e| {
                        view! {
                          <tr>
                            <th scope="row" class="md-mono">{e.code}</th>
                            <td>{move || kt_tracked(e.name)}</td>
                            <td class="md-muted">{move || kt_tracked(e.desc)}</td>
                          </tr>
                        }
                      })
                      .collect_view()}
                  </tbody>
                </table>
              </div>
            </section>

            <TableSection
              title="发射类别标识的构成"
              headers=&["组成", "含义", "示例"]
              rows=EMISSION_DESIGNATION
              class="min-w-[720px]"
            />

            <TableSection
              title="50Ω 电平换算（dBm · dBμV · 电动势）"
              headers=&["换算", "公式", "说明"]
              rows=LEVEL_CONVERSIONS
              class="min-w-[720px]"
            />

            <TableSection
              title="功率与 dBm 对照（含操作证功率上限）"
              headers=&["功率", "dBm", "说明"]
              rows=POWER_DBM_TABLE
              class="min-w-[720px]"
            />

            <TableSection
              title="单位前缀与量级"
              headers=&["前缀", "倍数", "示例"]
              rows=UNIT_PREFIXES
              class="min-w-[720px]"
            />

            <section class="md-panel">
              <h2 class="md-panel-title">{move || t("exam.contact-english-phrases")}</h2>
              <dl class="md-phrases">
                {PHRASES
                  .iter()
                  .map(|p| {
                    view! {
                      <div class="md-phrase">
                        <dt class="md-muted md-sm">{move || kt_tracked(p.usage)}</dt>
                        <dd>
                          <div class="md-mono">{p.en}</div>
                          <div class="md-muted md-sm">{move || kt_tracked(p.zh)}</div>
                        </dd>
                      </div>
                    }
                  })
                  .collect_view()}
              </dl>
            </section>

            <TableSection
              title="知识来源与时效"
              headers=&["领域", "来源", "说明"]
              rows=KNOWLEDGE_SOURCES
              class="min-w-[720px]"
            />
          </div>
        </div>
      </div>
    }
    .into_any();
  }

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader title=move || t("shell.exam-reference-title") subtitle=move || t("exam.licence-privileges-zones-rst") />
      <PageContainer>
        // 操作证类别与权限
        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("exam.operator-licence-classes-and")}</h2>
          <Table
            aria_label=Signal::derive(move || t("exam.operator-licence-classes-and"))
            class="min-w-[560px]"
          >
            <TableHead>
              <TableRow>
                <TableHeader>{move || t("exam.class-column")}</TableHeader>
                <TableHeader>{move || t("exam.frequency-range")}</TableHeader>
                <TableHeader>{move || t("exam.power-limit")}</TableHeader>
                <TableHeader>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {LICENSE_CLASSES
                .iter()
                .map(|c| {
                  view! {
                    <TableRow>
                      <TableCell class="align-top font-mono">
                        {move || tf("learning.class", &[c.class])}
                      </TableCell>
                      <TableCell class="align-top">{move || kt_tracked(c.freq)}</TableCell>
                      <TableCell class="align-top">{move || kt_tracked(c.power)}</TableCell>
                      <TableCell class="align-top text-muted-foreground">{move || kt_tracked(c.note)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        // 分区号：地图 + 色块图例 + 分区号表
        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("exam.amateur-radio-zone-numbers")}</h2>
          <p class="px-4 pt-3 text-xs leading-5 text-muted-foreground">
            {move || t("exam.call-area-map-hint")}
          </p>
          <div class="px-4 py-3">
            <CallAreaMap selected=selected on_select=toggle_area />
          </div>
          <ChipGroup
            value=Signal::derive(move || selected.get().unwrap_or_default())
            on_change=toggle_area
            aria_label=Signal::derive(move || t("exam.call-area-map"))
            class="gap-1.5 border-t px-4 py-3"
          >
            {CALL_AREAS
              .iter()
              .map(|a| {
                let color = call_area_color(a.digit);
                let label = view! {
                  <span
                    class="inline-block h-3 w-3"
                    style=format!("background-color: {color}")
                  ></span>
                  <span class="font-mono">{a.digit}</span>
                }
                  .into_any();
                view! { <Chip value=a.digit.to_string()>{label}</Chip> }
              })
              .collect_view()}
            // 空串 = 清空选区（与 `toggle_area` 的约定一致），默认就停在「全部」。
            <Chip value=String::new()>{move || t("exam.all")}</Chip>
          </ChipGroup>
          <div class="grid grid-cols-2 gap-2 border-t p-4 sm:grid-cols-3 lg:grid-cols-5">
            {move || {
              let sel = selected.get();
              CALL_AREAS
                .iter()
                .map(|a| {
                  let is_sel = sel.as_deref() == Some(a.digit);
                  let cell = if is_sel {
                    "flex items-start gap-2 border border-gold bg-inset p-3"
                  } else {
                    "flex items-start gap-2 border bg-inset p-3"
                  };
                  let digit_class = if is_sel {
                    "font-mono text-lg text-gold-text"
                  } else {
                    "font-mono text-lg text-primary"
                  };
                  view! {
                    <div class=cell>
                      <span class=digit_class>{a.digit}</span>
                      <span class="text-xs leading-5 text-muted-foreground">
                        {move || data::kt_tracked(a.regions)}
                      </span>
                    </div>
                  }
                })
                .collect_view()
            }}
          </div>
        </section>

        // RST 信号报告
        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("shell.rst-report")}</h2>
          <div class="grid grid-cols-1 gap-3 p-4 md:grid-cols-3">
            {RST_SCALES
              .iter()
              .map(|s| {
                view! {
                  <div class="border">
                    <div class="border-b bg-inset px-3 py-2">
                      <span class="font-mono text-primary">{s.key}</span>
                      <span class="ml-2 text-sm">{move || kt_tracked(s.name)}</span>
                      <div class="text-xs text-muted-foreground">{move || kt_tracked(s.note)}</div>
                    </div>
                    <dl class="divide-y text-sm">
                      {s
                        .levels
                        .iter()
                        .map(|&(num, desc)| {
                          view! {
                            <div class="flex gap-2 px-3 py-1.5">
                              <dt class="font-mono tabular-nums">{num}</dt>
                              <dd class="text-muted-foreground">{move || kt_tracked(desc)}</dd>
                            </div>
                          }
                        })
                        .collect_view()}
                    </dl>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        // 发射类别
        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("exam.emission-designators")}</h2>
          <Table
            aria_label=Signal::derive(move || t("exam.emission-designators"))
            class="min-w-[480px]"
          >
            <TableHead>
              <TableRow>
                <TableHeader>{move || t("exam.designator")}</TableHeader>
                <TableHeader>{move || t("exam.name")}</TableHeader>
                <TableHeader>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {EMISSION_TYPES
                .iter()
                .map(|e| {
                  view! {
                    <TableRow>
                      <TableCell class="align-top font-mono">{e.code}</TableCell>
                      <TableCell class="align-top">{move || kt_tracked(e.name)}</TableCell>
                      <TableCell class="align-top text-muted-foreground">{move || kt_tracked(e.desc)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        // 发射类别的构成
        <TableSection
          title="发射类别标识的构成"
          headers=&["组成", "含义", "示例"]
          rows=EMISSION_DESIGNATION
          class="min-w-[720px]"
        />

        // 电平换算 / 功率对照 / 单位前缀
        <TableSection
          title="50Ω 电平换算（dBm · dBμV · 电动势）"
          headers=&["换算", "公式", "说明"]
          rows=LEVEL_CONVERSIONS
          class="min-w-[720px]"
        />
        <TableSection
          title="功率与 dBm 对照（含操作证功率上限）"
          headers=&["功率", "dBm", "说明"]
          rows=POWER_DBM_TABLE
          class="min-w-[720px]"
        />
        <TableSection
          title="单位前缀与量级"
          headers=&["前缀", "倍数", "示例"]
          rows=UNIT_PREFIXES
          class="min-w-[720px]"
        />

        // 通联英语
        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("exam.contact-english-phrases")}</h2>
          <dl class="divide-y">
            {PHRASES
              .iter()
              .map(|p| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[7rem_1fr]">
                    <dt class="text-xs text-muted-foreground">{move || kt_tracked(p.usage)}</dt>
                    <dd>
                      <div class="font-mono text-sm text-foreground">{p.en}</div>
                      <div class="mt-0.5 text-sm text-muted-foreground">{move || kt_tracked(p.zh)}</div>
                    </dd>
                  </div>
                }
              })
              .collect_view()}
          </dl>
        </section>
        // 知识来源与时效
        <TableSection
          title="知识来源与时效"
          headers=&["领域", "来源", "说明"]
          rows=KNOWLEDGE_SOURCES
          class="min-w-[720px]"
        />
      </PageContainer>
    </div>
  }
  .into_any()
}

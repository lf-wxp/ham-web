//! 公式速查：高频必背公式；有对应计算器的点击跳转到 `/tools` 代入计算，没有的只展示。

use ham_web_core::formulas::{FORMULA_GROUPS, tex_of};
use ham_web_core::unit_conversions::UNIT_CONVERSION_GROUPS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn FormulasPage() -> impl IntoView {
  set_title("exam.formula-reference");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader title=move || t("exam.formula-reference") subtitle=move || t("exam.must-know-formulas-tap") />
      <PageContainer>
        {FORMULA_GROUPS
          .iter()
          .map(|g| {
            view! {
              <section class=crate::kit::card_class()>
                <h2 class=crate::kit::card_title_class()>{move || kt_tracked(g.name)}</h2>
                <div class="grid gap-2 p-4 sm:grid-cols-2">
                  {g.items
                    .iter()
                    .map(|f| {
                      let body = view! {
                        <div class="text-sm">{move || kt_tracked(f.name)}</div>
                        // KaTeX 可用时渲染 data-tex，否则保留 Unicode 表达式。
                        <div class="mt-1 font-mono text-sm text-primary" data-tex=tex_of(f.name)>
                          {move || kt_tracked(f.expr)}
                        </div>
                        <div class="mt-0.5 text-xs text-muted-foreground">{move || kt_tracked(f.desc)}</div>
                        {f.tool.map(|_| {
                          view! {
                            <div class="mt-1 text-xs text-primary opacity-0 transition-opacity group-hover:opacity-100">
                              {move || t("knowledge.calculate")}
                            </div>
                          }
                        })}
                      };
                      // 没有对应计算器的公式只展示：不做成链接，也不出「计算」提示（点进去算不出这条公式）。
                      match f.tool {
                        Some(tool) => {
                          view! {
                            <a
                              href=format!("/tools#{tool}")
                              class="group border bg-inset p-3 transition-colors hover:bg-accent/60"
                            >
                              {body}
                            </a>
                          }
                            .into_any()
                        }
                        None => view! { <div class="border bg-inset p-3">{body}</div> }.into_any(),
                      }
                    })
                    .collect_view()}
                </div>
              </section>
            }
          })
          .collect_view()}

        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("exam.units-title")}</h2>
          <div class="space-y-4 p-4">
            {UNIT_CONVERSION_GROUPS
              .iter()
              .map(|g| {
                view! {
                  <div>
                    <h3 class="mb-2 text-xs text-muted-foreground">{move || crate::data::core_text_tracked(g.name)}</h3>
                    <div class="grid gap-2 sm:grid-cols-2">
                      {g.items
                        .iter()
                        .map(|c| {
                          let href = format!("/tools#{}", c.tool);
                          view! {
                            <a
                              href=href
                              class="group border bg-inset p-3 transition-colors hover:bg-accent/60"
                            >
                              <div class="text-sm">{move || crate::data::core_text_tracked(c.name)}</div>
                              <div class="mt-1 font-mono text-sm text-primary">{move || kt_tracked(c.expr)}</div>
                              <div class="mt-0.5 text-xs text-muted-foreground">{move || crate::data::core_text_tracked(c.example)}</div>
                              <div class="mt-1 text-xs text-primary opacity-0 transition-opacity group-hover:opacity-100">
                                {move || t("knowledge.calculate")}
                              </div>
                            </a>
                          }
                        })
                        .collect_view()}
                    </div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </PageContainer>
    </div>
  }
}

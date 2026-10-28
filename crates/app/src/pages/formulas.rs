//! 公式速查：高频必背公式，每条关联到 `/tools` 页对应计算器，点击跳转代入计算。

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
              <section class="pxl-card border bg-card">
                <h2 class="pxl-section-title">{move || kt_tracked(g.name)}</h2>
                <div class="grid gap-2 p-4 sm:grid-cols-2">
                  {g.items
                    .iter()
                    .map(|f| {
                      let href = format!("/tools#{}", f.tool);
                      view! {
                        <a
                          href=href
                          class="group border bg-inset p-3 transition-colors hover:bg-accent/60"
                        >
                          <div class="text-sm">{move || kt_tracked(f.name)}</div>
                          // KaTeX 可用时渲染 data-tex，否则保留 Unicode 表达式。
                          <div class="mt-1 font-mono text-sm text-primary" data-tex=tex_of(f.name)>
                            {move || kt_tracked(f.expr)}
                          </div>
                          <div class="mt-0.5 text-xs text-muted-foreground">{move || kt_tracked(f.desc)}</div>
                          <div class="mt-1 text-xs text-primary opacity-0 transition-opacity group-hover:opacity-100">
                            {move || t("knowledge.calculate")}
                          </div>
                        </a>
                      }
                    })
                    .collect_view()}
                </div>
              </section>
            }
          })
          .collect_view()}

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("exam.units-title")}</h2>
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

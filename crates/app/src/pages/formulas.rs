//! 公式速查：高频必背公式，每条关联到 `/tools` 页对应计算器，点击跳转代入计算。

use ham_web_core::formulas::FORMULA_GROUPS;
use ham_web_core::unit_conversions::UNIT_CONVERSION_GROUPS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn FormulasPage() -> impl IntoView {
  set_title("exam.formula-reference");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader title=t("exam.formula-reference") subtitle=t("exam.must-know-formulas-tap") />
      <PageContainer>
        {FORMULA_GROUPS
          .iter()
          .map(|g| {
            view! {
              <section class="rounded-xl border bg-card">
                <h2 class="border-b px-4 py-3 text-sm font-semibold">{g.name}</h2>
                <div class="grid gap-2 p-4 sm:grid-cols-2">
                  {g.items
                    .iter()
                    .map(|f| {
                      let href = format!("/tools#{}", f.tool);
                      view! {
                        <a
                          href=href
                          class="group rounded-lg border bg-muted/30 p-3 transition-colors hover:bg-accent/60"
                        >
                          <div class="text-sm font-medium">{f.name}</div>
                          <div class="mt-1 font-mono text-sm font-semibold text-primary">{f.expr}</div>
                          <div class="mt-0.5 text-xs text-muted-foreground">{f.desc}</div>
                          <div class="mt-1 text-[11px] text-primary opacity-0 transition-opacity group-hover:opacity-100">
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

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("exam.units-title")}</h2>
          <div class="space-y-4 p-4">
            {UNIT_CONVERSION_GROUPS
              .iter()
              .map(|g| {
                view! {
                  <div>
                    <h3 class="mb-2 text-xs font-semibold text-muted-foreground">{move || t(g.name)}</h3>
                    <div class="grid gap-2 sm:grid-cols-2">
                      {g.items
                        .iter()
                        .map(|c| {
                          let href = format!("/tools#{}", c.tool);
                          view! {
                            <a
                              href=href
                              class="group rounded-lg border bg-muted/30 p-3 transition-colors hover:bg-accent/60"
                            >
                              <div class="text-sm font-medium">{move || t(c.name)}</div>
                              <div class="mt-1 font-mono text-sm font-semibold text-primary">{c.expr}</div>
                              <div class="mt-0.5 text-xs text-muted-foreground">{move || t(c.example)}</div>
                              <div class="mt-1 text-[11px] text-primary opacity-0 transition-opacity group-hover:opacity-100">
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

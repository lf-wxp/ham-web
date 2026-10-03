//! 公式速查：高频必背公式，每条关联到 `/tools` 页对应计算器，点击跳转代入计算。

use ham_web_core::formulas::FORMULA_GROUPS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn FormulasPage() -> impl IntoView {
  set_title(&t("公式速查"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader title=t("公式速查") subtitle=t("高频必背公式 · 点击跳转对应计算器") />
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
                            {move || t("计算 →")}
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
      </PageContainer>
    </div>
  }
}

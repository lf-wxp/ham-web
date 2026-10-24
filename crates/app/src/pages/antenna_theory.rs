//! 天线理论深化：近远场、Q 与带宽、G/T、互耦 —— 公式卡片。

use ham_web_core::antenna_theory::THEORY_SECTIONS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn AntennaTheoryPage() -> impl IntoView {
  set_title("knowledge.antenna-theory");
  // 本页自带页头、没套 KnowledgePage 外壳，正文译文得自己确保在拉。
  Effect::new(move |_| data::ensure_knowledge_i18n());
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("knowledge.antenna-theory")
        subtitle=move || t("knowledge.antenna-theory-subtitle")
      />
      <PageContainer>
        {THEORY_SECTIONS
          .iter()
          .map(|(title, formulas)| {
            view! {
              <section class="pxl-card border bg-card">
                <h2 class="pxl-section-title">
                  {move || data::kt_tracked(title)}
                </h2>
                <div class="grid gap-3 p-4 md:grid-cols-2">
                  {formulas
                    .iter()
                    .map(|f| {
                      view! {
                        <div class="border px-3 py-2">
                          <div class="text-xs text-muted-foreground">
                            {move || data::kt_tracked(f.name)}
                          </div>
                          // 公式源带在 data-tex 上：KaTeX 可用时由前端渲染，不可用就显示
                          // 下面的 Unicode 表达式（同一公式的两种写法）。
                          // tex / expr 是数学表示，不随界面语言变 —— 不走知识库翻译。
                          <div class="formula mt-1 font-mono text-base" data-tex=f.tex>
                            {f.expr}
                          </div>
                          <div class="mt-1 text-xs text-muted-foreground">
                            {move || data::kt_tracked(f.desc)}
                          </div>
                        </div>
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

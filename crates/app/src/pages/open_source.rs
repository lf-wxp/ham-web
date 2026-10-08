//! 开源项目与 DIY 教程索引。

use ham_web_core::open_source::{DIY_GUIDES, OSS_CATEGORIES, OSS_PROJECTS};
use leptos::prelude::*;

use crate::components::common::{KnowledgePage, SectionCard};
use crate::data::{self, kt};
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::util::set_title;

#[component]
pub fn OpenSourcePage() -> impl IntoView {
  set_title("knowledge.open-source-projects-and");
  view! {
    <KnowledgePage title=t("knowledge.open-source-projects-and") subtitle=t("knowledge.community-open-source-software")>
      <section class="rounded-xl border bg-card">
        <div class="border-b px-4 py-3">
          <h2 class="text-sm font-semibold">{move || t("knowledge.featured-open-source-projects")}</h2>
          <p class="mt-0.5 text-xs text-muted-foreground">
            {move || t("knowledge.grouped-by-purpose-click")}
          </p>
        </div>
        <div class="space-y-5 p-4">
          {OSS_CATEGORIES
            .iter()
            .map(|cat| {
              let items: Vec<_> = OSS_PROJECTS.iter().filter(|p| p.category == *cat).collect();
              view! {
                <div>
                  <h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-primary">
                    {move || t(cat)}
                  </h3>
                  <div class="grid gap-2 sm:grid-cols-2">
                    {items
                      .into_iter()
                      .map(|p| {
                        view! {
                          <a
                            href=p.url
                            target="_blank"
                            rel="noopener noreferrer"
                            class="group flex items-start gap-2 rounded-lg border px-3 py-2 transition-colors hover:bg-accent"
                          >
                            <Icon
                              kind=IconKind::ExternalLink
                              class="mt-0.5 size-3.5 shrink-0 text-muted-foreground"
                            />
                            <span class="min-w-0">
                              <span class="block text-sm font-medium">
                                {move || {
                                  data::track_knowledge();
                                  kt(p.name)
                                }}
                              </span>
                              <span class="mt-0.5 block text-xs text-muted-foreground">
                                {move || {
                                  data::track_knowledge();
                                  kt(p.desc)
                                }}
                              </span>
                            </span>
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

      <SectionCard title=t("knowledge.diy-and-build-tutorials")>
        <div class="divide-y">
          {DIY_GUIDES
            .iter()
            .map(|(href, name, desc)| {
              view! {
                <a
                  href=*href
                  class="flex flex-col gap-0.5 px-4 py-3 transition-colors hover:bg-muted/40 sm:flex-row sm:items-baseline sm:gap-3"
                >
                  <span class="shrink-0 text-sm font-medium text-primary">
                    {move || {
                      data::track_knowledge();
                      kt(name)
                    }}
                  </span>
                  <span class="text-sm text-muted-foreground">
                    {move || {
                      data::track_knowledge();
                      kt(desc)
                    }}
                  </span>
                </a>
              }
            })
            .collect_view()}
        </div>
      </SectionCard>
    </KnowledgePage>
  }
}

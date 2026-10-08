//! 火腿社区：国内外论坛与问答的外链聚合页。

use ham_web_core::community::{COMMUNITY_CATEGORIES, COMMUNITY_LINKS};
use leptos::prelude::*;

use crate::components::common::{KnowledgePage, SectionCard};
use crate::data::{self, kt};
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::util::set_title;

#[component]
pub fn CommunityPage() -> impl IntoView {
  set_title("knowledge.ham-community");
  view! {
    <KnowledgePage title=t("knowledge.ham-community") subtitle=t("knowledge.ham-radio-forums-q")>
      <SectionCard title=t("knowledge.about-this-community")>
        <p class="text-sm leading-relaxed text-muted-foreground">
          {move || {
            t(
              "knowledge.this-site-currently-focuses",
            )
          }}
        </p>
      </SectionCard>

      <section class="rounded-xl border bg-card">
        <div class="border-b px-4 py-3">
          <h2 class="text-sm font-semibold">{move || t("knowledge.community-index")}</h2>
          <p class="mt-0.5 text-xs text-muted-foreground">
            {move || t("knowledge.organized-by-category-click")}
          </p>
        </div>
        <div class="space-y-5 p-4">
          {COMMUNITY_CATEGORIES
            .iter()
            .map(|cat| {
              let items: Vec<_> = COMMUNITY_LINKS.iter().filter(|l| l.category == *cat).collect();
              view! {
                <div>
                  <h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-primary">
                    {move || t(cat)}
                  </h3>
                  <div class="grid gap-2 sm:grid-cols-2">
                    {items
                      .into_iter()
                      .map(|l| {
                        view! {
                          <a
                            href=l.url
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
                                  kt(l.name)
                                }}
                              </span>
                              <span class="mt-0.5 block text-xs text-muted-foreground">
                                {move || {
                                  data::track_knowledge();
                                  kt(l.desc)
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
    </KnowledgePage>
  }
}

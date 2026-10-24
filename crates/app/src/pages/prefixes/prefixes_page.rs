use ham_web_core::prefixes::PREFIX_GROUPS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data;
use crate::util::set_title;

use super::call_area_quiz::CallAreaQuiz;
use crate::i18n::{t, tf};

#[component]
pub fn PrefixesPage() -> impl IntoView {
  set_title("shell.callsign-prefixes");
  // 本页自带页头（不是 `KnowledgePage`），译文要自己挂一次，否则大洲与前缀表在英文界面仍是中文。
  Effect::new(move |_| data::ensure_knowledge_i18n());
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.callsign-prefixes")
        subtitle=move || t("radio.itu-allocations-and-call-area-quiz")
      />

      <PageContainer>
        <section class="pxl-card border bg-card p-4 text-sm text-muted-foreground">
          {move || t("radio.callsign-structure-lives-on")}
          <a
            href="/callsign"
            class="ml-1 text-primary underline underline-offset-2"
          >
            {move || t("tools.callsign-knowledge")}
          </a>
        </section>

        <CallAreaQuiz />

        {PREFIX_GROUPS
          .iter()
          .map(|g| {
            let region = g.region;
            view! {
              <section class="pxl-card border bg-card">
                <h2 class="pxl-section-title">
                  {move || data::kt_tracked(region)}
                  <span class="ml-2 text-xs font-normal text-muted-foreground">
                    {move || tf("log.entry", &[&g.prefixes.len().to_string()])}
                  </span>
                </h2>
                <div class="grid grid-cols-1 gap-2 p-4 sm:grid-cols-2 lg:grid-cols-3">
                  {g
                    .prefixes
                    .iter()
                    .map(|p| {
                      let entity = p.entity;
                      view! {
                        <div class="flex items-baseline gap-2 px-2 py-1.5">
                          <span class="w-32 shrink-0 font-mono text-sm text-primary">{p.prefix}</span>
                          <span class="text-sm text-muted-foreground">{move || data::kt_tracked(entity)}</span>
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

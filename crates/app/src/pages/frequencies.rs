//! 常用频率速查：遇险应急、国际信标、呼叫、APRS 与数字模式频率。

use ham_web_core::frequencies::FREQ_GROUPS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::t;
use crate::util::set_title;

#[component]
pub fn FrequenciesPage() -> impl IntoView {
  set_title("shell.common-frequencies");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.common-frequencies")
        subtitle=move || t("knowledge.distress-and-emergency-beacons")
      />

      <PageContainer>
        {FREQ_GROUPS
          .iter()
          .map(|g| {
            view! {
              <section class="pxl-card border bg-card">
                <h2 class="pxl-section-title">{g.category}</h2>
                <div class="grid grid-cols-1 gap-2 p-4 sm:grid-cols-2">
                  {g
                    .freqs
                    .iter()
                    .map(|&(freq, usage)| {
                      view! {
                        <div class="flex items-baseline gap-2 px-2 py-1.5">
                          <span class="w-36 shrink-0 font-mono text-sm text-primary">{freq}</span>
                          <span class="text-sm text-muted-foreground">{usage}</span>
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

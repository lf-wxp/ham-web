use ham_web_core::prefixes::PREFIX_GROUPS;
use leptos::prelude::*;

use crate::util::set_title;

use super::call_area_quiz::CallAreaQuiz;
use super::callsign_analyzer::CallsignAnalyzer;
use crate::i18n::{t, tf};

#[component]
pub fn PrefixesPage() -> impl IntoView {
  set_title(&t("呼号前缀"));
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("呼号前缀")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("ITU 分配 · 结构解析 · 按大洲分组")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <CallsignAnalyzer />

        <CallAreaQuiz />

        {PREFIX_GROUPS
          .iter()
          .map(|g| {
            view! {
              <section class="rounded-xl border bg-card">
                <h2 class="border-b px-4 py-3 text-sm font-semibold">
                  {g.region}
                  <span class="ml-2 text-xs font-normal text-muted-foreground">
                    {move || tf("{} 个", &[&g.prefixes.len().to_string()])}
                  </span>
                </h2>
                <div class="grid grid-cols-1 gap-2 p-4 sm:grid-cols-2 lg:grid-cols-3">
                  {g
                    .prefixes
                    .iter()
                    .map(|p| {
                      view! {
                        <div class="flex items-baseline gap-2 rounded-lg px-2 py-1.5">
                          <span class="w-32 shrink-0 font-mono text-sm font-semibold text-primary">{p.prefix}</span>
                          <span class="text-sm text-muted-foreground">{p.entity}</span>
                        </div>
                      }
                    })
                    .collect_view()}
                </div>
              </section>
            }
          })
          .collect_view()}
      </div>
    </div>
  }
}

use ham_web_core::phonetic::PHONETIC;
use leptos::prelude::*;

use crate::ui::Stat;
use crate::util::set_title;

use super::phonetic_card::PhoneticCard;

#[component]
pub fn PhoneticPage() -> impl IntoView {
  set_title("字母解释法");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"字母解释法"</h1>
            <div class="text-xs text-muted-foreground">"ITU 语音字母表 · 话音通联拼读呼号"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-4">
          <Stat label="字母" value=PHONETIC.len() />
          <div class="rounded-xl border bg-card p-4 sm:col-span-3">
            <p class="text-sm leading-6 text-muted-foreground">
              "用固定单词代表每个字母，话音通联拼读呼号时使用，避免近音字母（如 B、D、E、P、T）听错。例如呼号 BG4XXX 应读作 Bravo Golf Four X-ray X-ray X-ray。"
            </p>
            <p class="mt-2 text-xs text-muted-foreground">"读音提示中大写部分为重读音节。"</p>
          </div>
        </div>

        <section>
          <h2 class="mb-3 text-sm font-semibold">"语音字母表"</h2>
          <div class="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6">
            {PHONETIC.iter().map(|e| view! { <PhoneticCard entry=e /> }).collect_view()}
          </div>
        </section>
      </div>
    </div>
  }
}

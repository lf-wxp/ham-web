use ham_web_core::morse::{DIGITS, LETTERS, PUNCTUATION, TIMING};
use leptos::prelude::*;

use crate::ui::Stat;
use crate::util::set_title;

use super::morse_card::MorseCard;
use super::morse_trainer::MorseTrainer;
use super::send_trainer::SendTrainer;

#[component]
pub fn MorsePage() -> impl IntoView {
  set_title("莫尔斯电码");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"莫尔斯电码"</div>
            <div class="text-xs text-muted-foreground">"国际摩尔斯电码（ITU）· 字母 · 数字 · 标点 · 点击试听"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label="字母" value=LETTERS.len() />
          <Stat label="数字" value=DIGITS.len() />
          <Stat label="标点符号" value=PUNCTUATION.len() />
          <Stat label="字符总计" value=LETTERS.len() + DIGITS.len() + PUNCTUATION.len() />
        </div>

        <MorseTrainer />

        <SendTrainer />

        <div class="flex items-center gap-2 text-xs text-muted-foreground">
          <span class="font-mono text-base leading-none text-primary">"•"</span>
          "点"
          <span class="mx-2 text-border">"·"</span>
          <span class="font-mono text-base leading-none text-primary">"—"</span>
          "划"
          <span class="mx-2 text-border">"·"</span>
          "点击任意卡片即可试听"
        </div>

        <section>
          <h2 class="mb-3 text-sm font-semibold">
            "字母表"
            <span class="ml-2 text-xs font-normal text-muted-foreground">"含 ITU 语音字母"</span>
          </h2>
          <div class="grid grid-cols-3 gap-2 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8">
            {LETTERS.iter().map(|c| view! { <MorseCard entry=c /> }).collect_view()}
          </div>
        </section>

        <section>
          <h2 class="mb-3 text-sm font-semibold">"数字"</h2>
          <div class="grid grid-cols-5 gap-2 sm:grid-cols-10">
            {DIGITS.iter().map(|c| view! { <MorseCard entry=c /> }).collect_view()}
          </div>
        </section>

        <section>
          <h2 class="mb-3 text-sm font-semibold">"常用标点符号"</h2>
          <div class="grid grid-cols-4 gap-2 sm:grid-cols-6 lg:grid-cols-9">
            {PUNCTUATION.iter().map(|c| view! { <MorseCard entry=c /> }).collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"信号时值标准"</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">"以一个「点」时间为基准（CW 拍发节奏）："</p>
          <div class="grid grid-cols-2 gap-3 p-4 sm:grid-cols-5">
            {TIMING
              .iter()
              .map(|&(k, v)| {
                view! {
                  <div class="rounded-lg border bg-muted/40 p-3 text-center">
                    <div class="text-xl font-semibold tabular-nums">{v}</div>
                    <div class="mt-1 text-xs text-muted-foreground">{k}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </div>
    </div>
  }
}

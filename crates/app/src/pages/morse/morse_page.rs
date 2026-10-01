use ham_web_core::koch::farnsworth;
use ham_web_core::morse::{DIGITS, LETTERS, PUNCTUATION, TIMING};
use leptos::prelude::*;

use crate::morse_audio::play_morse_timed_with;
use crate::morse_settings::{provide_morse_settings, use_morse_settings};
use crate::ui::Stat;
use crate::util::set_title;

use super::abbrev_quiz::AbbrevQuiz;
use super::callsign_runner::CallsignRunner;
use super::cw_decoder::CwDecoder;
use super::koch_trainer::KochTrainer;
use super::morse_card::MorseCard;
use super::morse_display;
use super::morse_trainer::MorseTrainer;
use super::send_trainer::SendTrainer;
use super::signal_bars::SignalBars;

/// CW 专用符号（合并码，整体拍发、字符间无间隔）。
const PROSIGNS: &[(&str, &str, &str)] = &[
  ("AR", ".-.-.", "报文结束"),
  ("SK", "...-.-", "通联结束"),
  ("KN", "-.--.", "仅邀指定台"),
  ("BT", "-...-", "分隔/暂停"),
  ("AS", ".-...", "请稍候"),
  ("HH", "........", "更正（误发）"),
];

#[component]
pub fn MorsePage() -> impl IntoView {
  set_title("莫尔斯电码");
  provide_morse_settings();
  let settings = use_morse_settings();
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"莫尔斯电码"</h1>
            <div class="text-xs text-muted-foreground">"国际摩尔斯电码（ITU）· 字母 · 数字 · 标点 · 点击试听"</div>
          </div>
          <SignalBars />
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label="字母" value=LETTERS.len() />
          <Stat label="数字" value=DIGITS.len() />
          <Stat label="标点符号" value=PUNCTUATION.len() />
          <Stat label="字符总计" value=LETTERS.len() + DIGITS.len() + PUNCTUATION.len() />
        </div>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-x-6 gap-y-3 px-4 py-3">
            <div class="flex items-center gap-2">
              <span class="whitespace-nowrap text-xs text-muted-foreground">"音调"</span>
              <input
                type="range"
                min="300"
                max="1200"
                step="10"
                aria-label="播放音调"
                prop:value=move || settings.settings.get().tone_hz.to_string()
                aria-valuetext=move || format!("{} Hz", settings.settings.get().tone_hz)
                on:input=move |e| {
                  if let Ok(v) = event_target_value(&e).parse::<u32>() {
                    settings.set_tone(v);
                  }
                }
                class="h-1.5 w-32 accent-primary"
              />
              <span class="w-14 text-xs tabular-nums text-muted-foreground">{move || format!("{} Hz", settings.settings.get().tone_hz)}</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="whitespace-nowrap text-xs text-muted-foreground">"音量"</span>
              <input
                type="range"
                min="0"
                max="100"
                step="1"
                aria-label="播放音量"
                prop:value=move || (settings.settings.get().volume * 100.0).round().to_string()
                aria-valuetext=move || format!("{:.0}%", settings.settings.get().volume * 100.0)
                on:input=move |e| {
                  if let Ok(v) = event_target_value(&e).parse::<f32>() {
                    settings.set_volume(v / 100.0);
                  }
                }
                class="h-1.5 w-32 accent-primary"
              />
              <span class="w-14 text-xs tabular-nums text-muted-foreground">{move || format!("{:.0}%", settings.settings.get().volume * 100.0)}</span>
            </div>
            <button
              type="button"
              on:click=move |_| {
                play_morse_timed_with("CQ", farnsworth(20.0, 20.0), settings.tone_hz(), settings.volume());
              }
              class="inline-flex items-center gap-1.5 rounded-full border bg-card px-3 py-1 text-xs transition-all duration-200 ease-out hover:border-primary/40 hover:bg-accent hover:text-accent-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
            >
              "试听"
            </button>
          </div>
        </section>

        <KochTrainer />

        <CallsignRunner />

        <MorseTrainer />

        <SendTrainer />

        <AbbrevQuiz />

        <CwDecoder />

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
          <div class="grid grid-cols-2 gap-2 min-[400px]:grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8">
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">
            "CW 专用符号（Prosigns）"
            <span class="ml-2 text-xs font-normal text-muted-foreground">"整体拍发，字符间无间隔"</span>
          </h2>
          <div class="grid grid-cols-2 gap-3 p-4 sm:grid-cols-3">
            {PROSIGNS
              .iter()
              .map(|&(label, code, meaning)| {
                view! {
                  <button
                    type="button"
                    on:click=move |_| {
                      play_morse_timed_with(code, farnsworth(20.0, 20.0), settings.tone_hz(), settings.volume());
                    }
                    class="group rounded-lg border bg-muted/40 p-3 text-center transition-all duration-200 ease-out hover:border-primary/40 hover:bg-accent/60 active:scale-[0.97] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
                    title=format!("试听 {label}")
                  >
                    <div class="text-lg font-semibold tabular-nums">{label}</div>
                    <div class="font-mono text-base font-semibold tracking-widest text-primary">
                      {morse_display(code)}
                    </div>
                    <div class="mt-1 text-xs text-muted-foreground">{meaning}</div>
                  </button>
                }
              })
              .collect_view()}
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

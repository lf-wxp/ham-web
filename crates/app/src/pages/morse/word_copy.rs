use ham_web_core::koch::farnsworth;
use ham_web_core::morse::COMMON_WORDS;
use leptos::prelude::*;

use crate::morse_audio::play_morse_timed_with;
use crate::morse_settings::use_morse_settings;
use crate::ui::input_class;

use super::{btn_primary, btn_secondary, encode_words, random_index};
use crate::i18n::{t, tf};

/// 抄收速度（WPM）。
const COPY_WPM: f64 = 18.0;

/// CW 单词抄收：播放一个常用单词/缩语的莫尔斯音，抄收并拼出。
#[component]
pub(super) fn WordCopy() -> impl IntoView {
  let settings = use_morse_settings();
  let word = RwSignal::new(COMMON_WORDS[0]);
  let input = RwSignal::new(String::new());
  let feedback = RwSignal::new(None::<bool>);
  let correct = RwSignal::new(0usize);
  let wrong = RwSignal::new(0usize);

  let next = move || {
    word.set(COMMON_WORDS[random_index(COMMON_WORDS.len())]);
    input.set(String::new());
    feedback.set(None);
  };
  next();

  let play = move || {
    let code = encode_words(word.get());
    play_morse_timed_with(
      &code,
      farnsworth(COPY_WPM, COPY_WPM),
      settings.tone_hz(),
      settings.volume(),
    );
  };

  let check = move || {
    let ok = input.get().trim().eq_ignore_ascii_case(word.get());
    if ok {
      correct.update(|v| *v += 1);
    } else {
      wrong.update(|v| *v += 1);
    }
    feedback.set(Some(ok));
  };

  view! {
    <section class="rounded-xl border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">
        {move || t("单词抄收")}
        <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("听常用缩语 / 单词的莫尔斯音，抄收并拼出")}</span>
      </h2>
      <div class="space-y-4 p-4">
        <div class="flex flex-col items-center gap-3 rounded-xl border bg-muted/30 px-4 py-6">
          <div class="text-xs text-muted-foreground">{move || t("点击播放，抄收后输入对应单词或缩语")}</div>
          <button
            type="button"
            on:click=move |_| play()
            class="rounded-lg bg-primary px-5 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
          >
            {move || t("🔊 播放")}
          </button>
        </div>
        <input
          type="text"
          placeholder=move || t("输入抄收的单词，如 CQ / 73 / RST")
          aria-label=move || t("抄收输入")
          prop:value=move || input.get()
          on:input=move |e| input.set(event_target_value(&e))
          on:keydown=move |e| {
            if e.key() == "Enter" && feedback.get().is_none() {
              check();
            }
          }
          class=format!("{} font-mono uppercase", input_class(""))
        />
        <div class="flex flex-wrap items-center gap-3">
          {move || {
            if feedback.get().is_none() {
              view! {
                <button type="button" on:click=move |_| check() class=btn_primary("")>{move || t("核对")}</button>
              }
              .into_any()
            } else {
              let ok = feedback.get().unwrap_or(false);
              view! {
                <span class=if ok { "text-sm font-medium text-emerald-600 dark:text-emerald-400" } else { "text-sm font-medium text-red-600 dark:text-red-400" }>
                  {if ok { t("正确！") } else { tf("答案是 {}", &[(word.get())]) }}
                </span>
                <button type="button" on:click=move |_| next() class=btn_primary("")>{move || t("下一题")}</button>
              }
              .into_any()
            }
          }}
          <button type="button" on:click=move |_| play() class=btn_secondary("")>{move || t("重播")}</button>
          <span class="ml-auto text-xs text-muted-foreground">
            {move || t("正确 ")} <span class="font-semibold tabular-nums text-foreground">{move || correct.get()}</span>
            {move || t("　错误 ")} <span class="font-semibold tabular-nums text-foreground">{move || wrong.get()}</span>
          </span>
        </div>
      </div>
    </section>
  }
}

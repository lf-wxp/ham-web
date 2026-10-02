//! 呼号抄收训练：播放字母解释法拼读，用户输入呼号，判定式核对。

use ham_web_core::morse::code_of;
use ham_web_core::phonetic::{PHONETIC, word_of};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::morse_audio::play_morse;
use crate::speech::speak_en;
use crate::ui::{Size, Variant, button_class, input_class};
use crate::util::{random, set_title};

/// 单个字符的拼读（字母用 ITU 单词，数字用英文读法，9 读作 Niner）。
fn read_char(c: char) -> String {
  if c.is_ascii_digit() {
    match c {
      '0' => "Zero",
      '1' => "One",
      '2' => "Two",
      '3' => "Three",
      '4' => "Four",
      '5' => "Five",
      '6' => "Six",
      '7' => "Seven",
      '8' => "Eight",
      '9' => "Niner",
      _ => "?",
    }
    .to_owned()
  } else {
    word_of(&c.to_ascii_uppercase().to_string())
      .unwrap_or("?")
      .to_owned()
  }
}

/// 随机生成一个中国业余呼号（如 `BG1ABC`）：B + 字母 + 分区数字 + 2~3 位后缀。
fn gen_callsign(rng: &mut impl FnMut() -> f64) -> String {
  const PREFIX: &[u8] = b"ABCDEFGHI";
  let mut s = String::from("B");
  s.push(PREFIX[(rng() * PREFIX.len() as f64) as usize] as char);
  s.push((b'0' + (rng() * 10.0) as u8) as char);
  let suffix_len = if rng() < 0.5 { 2 } else { 3 };
  for _ in 0..suffix_len {
    s.push((b'A' + (rng() * 26.0) as u8) as char);
  }
  s
}

/// 把呼号转为摩尔斯点划序列（字符间以空格分隔）。
fn to_morse(s: &str) -> String {
  s.chars().filter_map(code_of).collect::<Vec<_>>().join(" ")
}

/// 规范化用户输入：去空格与连字符、转大写。
fn normalize(s: &str) -> String {
  s.chars()
    .filter(|c| !c.is_whitespace() && *c != '-' && *c != '/')
    .map(|c| c.to_ascii_uppercase())
    .collect()
}

#[component]
pub fn CallsignCopyPage() -> impl IntoView {
  set_title(&t("呼号抄收训练"));

  let callsign = RwSignal::new(gen_callsign(&mut random));
  let input = RwSignal::new(String::new());
  let feedback = RwSignal::new(None::<bool>);
  let total = RwSignal::new(0usize);
  let correct = RwSignal::new(0usize);
  let streak = RwSignal::new(0usize);
  // false = 字母解释法（语音拼读），true = 摩尔斯电码
  let cw = RwSignal::new(false);
  let wpm = RwSignal::new(15u32);

  let spelled = move || {
    callsign
      .get()
      .chars()
      .map(read_char)
      .collect::<Vec<_>>()
      .join(", ")
  };

  let play = move || {
    if cw.get() {
      play_morse(&to_morse(&callsign.get()), f64::from(wpm.get()));
    } else {
      speak_en(&spelled());
    }
  };

  let next = move || {
    callsign.set(gen_callsign(&mut random));
    input.set(String::new());
    feedback.set(None);
  };

  let check = move || {
    let ok = normalize(&input.get()) == callsign.get();
    feedback.set(Some(ok));
    total.update(|c| *c += 1);
    if ok {
      correct.update(|c| *c += 1);
      streak.update(|c| *c += 1);
    } else {
      streak.set(0);
    }
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-2xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("呼号抄收训练")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("听字母解释法 / 摩尔斯电码拼读，抄收呼号 · 判定式核对")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-2xl space-y-4 px-4 py-5">
        <section class="rounded-xl border bg-card p-5">
          <div class="mb-3 flex items-center justify-between text-xs text-muted-foreground">
            <span>{move || t("正确 ")} <span class="font-semibold text-emerald-600">{correct.get()}</span> {" / "} {total.get()}</span>
            <span>{move || t("连续 ")} <span class="font-semibold text-foreground">{streak.get()}</span> {move || t(" 题")}</span>
          </div>

          <div class="mb-3 flex flex-wrap items-center gap-2">
            <button
              type="button"
              class=move || {
                if !cw.get() { button_class(Variant::Default, Size::Sm, "") } else { button_class(Variant::Outline, Size::Sm, "") }
              }
              on:click=move |_| cw.set(false)
            >
              {move || t("字母解释法")}
            </button>
            <button
              type="button"
              class=move || {
                if cw.get() { button_class(Variant::Default, Size::Sm, "") } else { button_class(Variant::Outline, Size::Sm, "") }
              }
              on:click=move |_| cw.set(true)
            >
              {move || t("摩尔斯电码")}
            </button>
            {move || cw.get().then(|| view! {
              <label class="flex items-center gap-1.5 text-xs text-muted-foreground">
                {move || t("速度")}
                <input
                  type="number"
                  min="5"
                  max="40"
                  aria-label=move || t("摩尔斯速度 WPM")
                  prop:value=move || wpm.get().to_string()
                  on:input=move |e| {
                    if let Ok(v) = event_target_value(&e).parse::<u32>() {
                      wpm.set(v.clamp(5, 40));
                    }
                  }
                  class="w-16 rounded-lg border bg-background px-2 py-1 text-sm tabular-nums outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
                />
                "WPM"
              </label>
            })}
          </div>

          <button
            type="button"
            class=format!("{} w-full", button_class(Variant::Default, Size::Default, ""))
            on:click=move |_| play()
          >
            {move || if cw.get() { t("🔊 播放摩尔斯") } else { t("🔊 播放拼读") }}
          </button>

          <div class="mt-4">
            <input
              type="text"
              placeholder=move || t("输入抄收到的呼号，如 BG1ABC")
              aria-label=move || t("呼号输入")
              prop:value=move || input.get()
              on:input=move |e| input.set(event_target_value(&e))
              on:keydown=move |e| {
                if e.key() == "Enter" && feedback.get().is_none() {
                  check();
                }
              }
              class=format!("{} font-mono uppercase", input_class(""))
            />
          </div>

          <div class="mt-3 flex items-center gap-2">
            {move || {
              if feedback.get().is_none() {
                view! {
                  <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=move |_| check()>
                    {move || t("核对")}
                  </button>
                }
                .into_any()
              } else {
                let ok = feedback.get().unwrap_or(false);
                view! {
                  <span class=if ok { "text-sm font-medium text-emerald-700 dark:text-emerald-400" } else { "text-sm font-medium text-red-700 dark:text-red-400" }>
                    {if ok { t("正确！") } else { tf("答案是 {}", &[&(callsign.get()).to_string()]) }}
                  </span>
                  <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=move |_| next()>
                    {move || t("下一题")}
                  </button>
                }
                .into_any()
              }
            }}
          </div>
        </section>

        <details class="rounded-xl border bg-card">
          <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">{move || t("字母解释法速查")}</summary>
          <div class="grid grid-cols-2 gap-x-6 gap-y-1 px-4 pb-4 sm:grid-cols-3">
            {PHONETIC
              .iter()
              .map(|e| {
                view! {
                  <div class="flex items-baseline gap-2 text-sm">
                    <span class="w-6 shrink-0 font-mono font-semibold text-primary">{e.letter}</span>
                    <span class="text-muted-foreground">{e.word} " " <span class="text-xs">({e.pronunciation})</span></span>
                  </div>
                }
              })
              .collect_view()}
          </div>
          <p class="px-4 pb-3 text-xs text-muted-foreground">
            {move || t("数字读法：0 Zero · 1 One · … · 9 Niner。字母解释法播放依赖系统 TTS（英文语音包）；摩尔斯电码由 Web Audio 合成，可在上方调节速度。")}
          </p>
        </details>
      </div>
    </div>
  }
}

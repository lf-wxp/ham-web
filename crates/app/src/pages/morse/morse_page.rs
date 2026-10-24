use ham_web_core::koch::farnsworth;
use ham_web_core::morse::{DIGITS, LETTERS, PUNCTUATION, TIMING};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::morse_audio::play_morse_timed_with;
use crate::morse_settings::{provide_morse_settings, use_morse_settings};
use crate::ui::{
  Button, Size, Slider, Stat, Tabs, TabsList, TabsPanel, TabsTrigger, Tooltip, Variant,
  card_title_class,
};
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
use super::word_copy::WordCopy;
use crate::i18n::{t, tf};

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
  set_title("shell.morse-code");
  provide_morse_settings();
  let settings = use_morse_settings();
  // 四个训练器各自成块、用的时候只用一个：堆在一页里要滚三四屏，于是收进 Tabs。
  // 默认落在 Koch 法（新手入口），选中的视图由用户切换，不做持久化。
  let trainer = RwSignal::new("koch".to_owned());
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.morse-code")
        subtitle=move || t("morse.international-morse-code-itu")
        actions=ViewFn::from(move || {
          view! {
            <SignalBars />
          }
        })
      />

      <PageContainer>
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label=t("morse.letters") value=LETTERS.len() />
          <Stat label=t("morse.digits") value=DIGITS.len() />
          <Stat label=t("morse.punctuation") value=PUNCTUATION.len() />
          <Stat label=t("morse.total-characters") value=LETTERS.len() + DIGITS.len() + PUNCTUATION.len() />
        </div>

        <section class="pxl-card border bg-card">
          <div class="flex flex-wrap items-center gap-x-6 gap-y-3 px-4 py-3">
            <div class="flex items-center gap-2">
              <span class="whitespace-nowrap text-xs text-muted-foreground">{move || t("morse.tone")}</span>
              <Slider
                value=Signal::derive(move || f64::from(settings.settings.get().tone_hz))
                on_change=Callback::new(move |v: f64| settings.set_tone(v as u32))
                min=300.0
                max=1200.0
                step=10.0
                aria_label=Signal::derive(move || t("morse.play-tone"))
                aria_valuetext=Signal::derive(move || format!("{} Hz", settings.settings.get().tone_hz))
                class="w-32"
              />
              <span class="w-14 text-xs tabular-nums text-muted-foreground">{move || format!("{} Hz", settings.settings.get().tone_hz)}</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="whitespace-nowrap text-xs text-muted-foreground">{move || t("morse.volume-label")}</span>
              <Slider
                value=Signal::derive(move || (f64::from(settings.settings.get().volume) * 100.0).round())
                on_change=Callback::new(move |v: f64| settings.set_volume(v as f32 / 100.0))
                min=0.0
                max=100.0
                step=1.0
                aria_label=Signal::derive(move || t("morse.volume"))
                aria_valuetext=Signal::derive(move || format!("{:.0}%", settings.settings.get().volume * 100.0))
                class="w-32"
              />
              <span class="w-14 text-xs tabular-nums text-muted-foreground">{move || format!("{:.0}%", settings.settings.get().volume * 100.0)}</span>
            </div>
            <Button
              variant=Variant::Outline
              size=Size::Sm
              on_click=Callback::new(move |_| {
                play_morse_timed_with("CQ", farnsworth(20.0, 20.0), settings.tone_hz(), settings.volume());
              })
            >
              {move || t("exam.preview")}
            </Button>
          </div>
        </section>

        <Tabs
          value=trainer
          on_change=Callback::new(move |v: String| trainer.set(v))
        >
          <TabsList aria_label=Signal::derive(move || t("morse.training-tabs"))>
            <TabsTrigger value="koch">{move || t("morse.koch")}</TabsTrigger>
            <TabsTrigger value="callsign">{move || t("shell.callsign-copy")}</TabsTrigger>
            <TabsTrigger value="copy">{move || t("morse.decoding-practice")}</TabsTrigger>
            <TabsTrigger value="send">{move || t("morse.sending-practice")}</TabsTrigger>
          </TabsList>
          <TabsPanel value="koch"><KochTrainer /></TabsPanel>
          <TabsPanel value="callsign"><CallsignRunner /></TabsPanel>
          <TabsPanel value="copy"><MorseTrainer /></TabsPanel>
          <TabsPanel value="send"><SendTrainer /></TabsPanel>
        </Tabs>

        <AbbrevQuiz />

        <WordCopy />

        <CwDecoder />

        <div class="flex items-center gap-2 text-xs text-muted-foreground">
          <span class="font-mono text-base leading-none text-primary">"•"</span>
          {move || t("morse.dot")}
          <span class="mx-2 text-border">"·"</span>
          <span class="font-mono text-base leading-none text-primary">"—"</span>
          {move || t("morse.dash")}
          <span class="mx-2 text-border">"·"</span>
          {move || t("morse.tap-any-card-to")}
        </div>

        <section>
          <h2 class=card_title_class("mb-3")>
            {move || t("morse.alphabet")}
            <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("morse.includes-itu-phonetic-alphabet")}</span>
          </h2>
          <div class="grid grid-cols-2 gap-2 xs:grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8">
            {LETTERS.iter().map(|c| view! { <MorseCard entry=c /> }).collect_view()}
          </div>
        </section>

        <section>
          <h2 class=card_title_class("mb-3")>{move || t("morse.digits")}</h2>
          <div class="grid grid-cols-5 gap-2 sm:grid-cols-10">
            {DIGITS.iter().map(|c| view! { <MorseCard entry=c /> }).collect_view()}
          </div>
        </section>

        <section>
          <h2 class=card_title_class("mb-3")>{move || t("morse.common-punctuation")}</h2>
          <div class="grid grid-cols-4 gap-2 sm:grid-cols-6 lg:grid-cols-9">
            {PUNCTUATION.iter().map(|c| view! { <MorseCard entry=c /> }).collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">
            {move || t("morse.cw-prosigns")}
            <span class="ml-2 text-xs font-normal text-muted-foreground">{move || t("morse.sent-as-a-whole")}</span>
          </h2>
          <div class="grid grid-cols-2 gap-3 p-4 sm:grid-cols-3">
            {PROSIGNS
              .iter()
              .map(|&(label, code, meaning)| {
                view! {
                  <Tooltip class="min-w-0" label=tf("common.preview", &[(label)])>
                    <button
                      type="button"
                      on:click=move |_| {
                        play_morse_timed_with(code, farnsworth(20.0, 20.0), settings.tone_hz(), settings.volume());
                      }
                      class="group w-full border bg-inset p-3 text-center transition duration-(--motion-normal) ease-out hover:border-primary/40 hover:bg-accent/60"
                    >
                    <div class="text-lg tabular-nums">{label}</div>
                    <div class="font-mono text-base tracking-widest text-primary">
                      {morse_display(code)}
                    </div>
                    <div class="mt-1 text-xs text-muted-foreground">{meaning}</div>
                    </button>
                  </Tooltip>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("morse.signal-timing-standard")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">{move || t("morse.based-on-one-dot")}</p>
          <div class="grid grid-cols-2 gap-3 p-4 sm:grid-cols-5">
            {TIMING
              .iter()
              .map(|&(k, v)| {
                view! {
                  <div class="border bg-inset p-3 text-center">
                    <div class="text-xl tabular-nums">{v}</div>
                    <div class="mt-1 text-xs text-muted-foreground">{k}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </PageContainer>
    </div>
  }
}

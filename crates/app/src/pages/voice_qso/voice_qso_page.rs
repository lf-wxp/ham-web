//! 话音通联练习：对方（语音合成）叫过来，你限时回一整段——双方呼号、报告、名字、QTH、设备、天线、73。
//!
//! 剧本、倒计时与漏项 / 超时的判定在核心 `ham_web_core::voice_qso`（有单测）。这里不做语音识别：
//! 麦克风只用来录下来回放自己的声音，说到了哪几项由用户自评打勾。

use std::time::Duration;

use ham_web_core::voice_qso::{Clock, ITEMS, Item, LIMIT_SECS, Lang, Scenario, clock, review};
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::texts::{item_id, item_label};
use crate::components::common::{PageContainer, PageHeader, SectionCard};
use crate::i18n::{Locale, locale, t, tf};
use crate::mic_recorder::{self, Recording};
use crate::pages::log::use_log_store;
use crate::speech::speak_with;
use crate::ui::{Button, Checkbox, ChipToggle, LedTone, Size, Variant};
use crate::util::set_title;

/// 练习进行到哪一步。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
  /// 还没开始回复。
  Idle,
  /// 正在回复：计时、录音。
  Running,
  /// 已结束：显示评判。
  Ended,
}

/// 麦克风的状态。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mic {
  /// 没在录。
  Off,
  /// 正在录。
  Recording,
  /// 没有麦克风 / 没授权 / 浏览器不支持：只计时。
  Unavailable,
}

#[component]
pub fn VoiceQsoPage() -> impl IntoView {
  set_title("shell.voice-qso");
  let store = use_log_store();
  let lang = RwSignal::new(if locale().get_untracked() == Locale::Zh {
    Lang::Zh
  } else {
    Lang::En
  });
  let new_scenario = move || {
    let me = store.active_station_untracked();
    Scenario::generate(js_sys::Date::now() as u64, &me.callsign)
  };
  let scenario = RwSignal::new(new_scenario());
  let phase = RwSignal::new(Phase::Idle);
  let start_ms = StoredValue::new(0.0f64);
  let elapsed_ms = RwSignal::new(0u64);
  let ticks = RwSignal::new(Vec::<Item>::new());
  let show_text = RwSignal::new(false);
  let mic = RwSignal::new(Mic::Off);
  let audio_url = RwSignal::new(None::<String>);
  let recording = StoredValue::new_local(None::<Recording>);
  // 代数：授权弹窗开着的时候用户可能已经点了结束 / 换了对手，这次录音就不要了。
  let generation = StoredValue::new(0u32);

  let drop_audio = move || {
    if let Some(url) = audio_url.try_get_untracked().flatten() {
      mic_recorder::revoke(&url);
    }
    audio_url.try_set(None);
  };
  on_cleanup(move || {
    generation.try_update_value(|g| *g = g.wrapping_add(1));
    if let Some(rec) = recording.try_update_value(Option::take).flatten() {
      rec.cancel();
    }
    if let Some(url) = audio_url.try_get_untracked().flatten() {
      mic_recorder::revoke(&url);
    }
  });

  // 倒计时：每 200 ms 刷新一次用时；停表时 `phase` 不是 Running，不再动。
  if let Ok(handle) = set_interval_with_handle(
    move || {
      if phase.try_get_untracked() == Some(Phase::Running) {
        let ms = (js_sys::Date::now() - start_ms.get_value()).max(0.0) as u64;
        elapsed_ms.try_set(ms);
      }
    },
    Duration::from_millis(200),
  ) {
    on_cleanup(move || handle.clear());
  }

  let play = move |_| {
    scenario.with_untracked(|s| {
      speak_with(
        &s.partner_speech(lang.get_untracked()),
        lang.get_untracked().speech_tag(),
      )
    });
  };

  let begin = move |_| {
    drop_audio();
    ticks.set(Vec::new());
    elapsed_ms.set(0);
    start_ms.set_value(js_sys::Date::now());
    phase.set(Phase::Running);
    mic.set(Mic::Off);
    generation.update_value(|g| *g = g.wrapping_add(1));
    let mine = generation.get_value();
    spawn_local(async move {
      let outcome = mic_recorder::start().await;
      // 授权弹窗开着的时候用户可能已经结束 / 换了对手 / 离开了页面：这次录音不要了。
      if generation.try_get_value() != Some(mine)
        || phase.try_get_untracked() != Some(Phase::Running)
      {
        if let Ok(rec) = outcome {
          rec.cancel();
        }
        return;
      }
      match outcome {
        Ok(rec) => {
          recording.set_value(Some(rec));
          mic.try_set(Mic::Recording);
        }
        Err(_) => {
          mic.try_set(Mic::Unavailable);
        }
      }
    });
  };

  let finish = move |_| {
    if phase.get_untracked() != Phase::Running {
      return;
    }
    let ms = (js_sys::Date::now() - start_ms.get_value()).max(0.0) as u64;
    elapsed_ms.set(ms);
    phase.set(Phase::Ended);
    show_text.set(true);
    generation.update_value(|g| *g = g.wrapping_add(1));
    let mine = generation.get_value();
    if let Some(rec) = recording.try_update_value(Option::take).flatten() {
      mic.set(Mic::Off);
      spawn_local(async move {
        let url = rec.stop().await;
        match url {
          Some(url) if generation.try_get_value() == Some(mine) => {
            audio_url.try_set(Some(url));
          }
          // 这期间又开始了新一轮 / 离开了页面：地址不再有人用。
          Some(url) => mic_recorder::revoke(&url),
          None => {}
        }
      });
    }
  };

  let new_partner = move |_| {
    generation.update_value(|g| *g = g.wrapping_add(1));
    if let Some(rec) = recording.try_update_value(Option::take).flatten() {
      rec.cancel();
    }
    drop_audio();
    scenario.set(new_scenario());
    phase.set(Phase::Idle);
    mic.set(Mic::Off);
    elapsed_ms.set(0);
    ticks.set(Vec::new());
    show_text.set(false);
  };

  let result = Memo::new(move |_| {
    (phase.get() == Phase::Ended).then(|| ticks.with(|t| review(t, elapsed_ms.get_untracked())))
  });

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.voice-qso")
        subtitle=move || t("radio.voice-subtitle")
      />
      <PageContainer class="space-y-5">
        // 设置条：说明 + 通话语言 + 换对手 + 本台呼号。整页唯一不需要标题栏的块，用平卡。
        <section class=crate::kit::card_pad_class()>
          <p class="text-sm leading-relaxed text-muted-foreground">{move || t("radio.voice-intro")}</p>
          <div class="mt-3 flex flex-wrap items-center gap-2">
            <ChipToggle
              active=Signal::derive(move || lang.get() == Lang::Zh)
              on_change=Callback::new(move |_| lang.set(Lang::Zh))
            >
              {move || t("radio.voice-lang-zh")}
            </ChipToggle>
            <ChipToggle
              active=Signal::derive(move || lang.get() == Lang::En)
              on_change=Callback::new(move |_| lang.set(Lang::En))
            >
              {move || t("radio.voice-lang-en")}
            </ChipToggle>
            <Button
              variant=Variant::Outline
              size=Size::Sm
              class="ml-auto"
              on_click=Callback::new(new_partner)
            >
              {move || t("radio.voice-new")}
            </Button>
          </div>
          <p class="mt-2 text-xs text-muted-foreground" data-testid="voice-me">
            {move || scenario.with(|s| tf("radio.voice-you-are", &[&s.me]))}
          </p>
        </section>

        // 对方来电：先听，抄下要点。次要块，标题栏 + 正文由 `SectionCard` 提供。
        <div data-testid="voice-partner">
          <SectionCard title=Signal::derive(move || t("radio.voice-partner-heading"))>
            <div class="space-y-3 p-4">
              <p class="text-xs text-muted-foreground">{move || t("radio.voice-listen-hint")}</p>
              <div class="flex flex-wrap items-center gap-2">
                <Button size=Size::Sm on_click=Callback::new(play)>
                  {move || t("radio.voice-play")}
                </Button>
                <Button
                  variant=Variant::Outline
                  size=Size::Sm
                  aria_expanded=Signal::derive(move || show_text.get())
                  on_click=Callback::new(move |_| show_text.update(|v| *v = !*v))
                >
                  {move || {
                    if show_text.get() { t("radio.voice-hide-text") } else { t("radio.voice-show-text") }
                  }}
                </Button>
              </div>
              {move || {
                show_text
                  .get()
                  .then(|| {
                    view! {
                      <p
                        class="break-words bg-inset px-3 py-2 text-sm leading-relaxed"
                        data-testid="partner-text"
                        lang=move || lang.get().speech_tag()
                      >
                        {move || scenario.with(|s| s.partner_speech(lang.get()))}
                      </p>
                    }
                  })
              }}
            </div>
          </SectionCard>
        </div>

        // 你的回复：这一页的当前任务块，用窗口级（L2）。标题栏走 `kit::card_title_class()`，
        // 内距与下沿由它自带；正文自己带 `p-4`（与 `SectionCard` 同一套写法）。
        <section class=crate::kit::card_class() data-testid="voice-reply">
          <h2 class=crate::kit::card_title_class()>{move || t("radio.voice-reply-heading")}</h2>
          <div class="space-y-4 p-4">
            <div class="flex flex-wrap items-center gap-x-4 gap-y-3">
              <p class=crate::kit::stat_class() role="timer" data-testid="voice-clock">
                {move || match phase.get() {
                  Phase::Idle => tf("radio.voice-limit", &[&LIMIT_SECS.to_string()]),
                  Phase::Running => match clock(elapsed_ms.get()) {
                    Clock::Left(n) => tf("radio.voice-left", &[&n.to_string()]),
                    Clock::Over(n) => tf("radio.voice-over", &[&n.to_string()]),
                  },
                  Phase::Ended => {
                    tf("radio.voice-used", &[&elapsed_ms.get().div_ceil(1000).to_string()])
                  }
                }}
              </p>
              // 主操作就是「发射键」：像素风按键带灯，回复期间常亮。
              {move || match phase.get() {
                Phase::Running => {
                  view! {
                    <Button
                      led=LedTone::Tx
                      led_on=Signal::derive(|| true)
                      on_click=Callback::new(finish)
                    >
                      {t("radio.voice-end")}
                    </Button>
                  }
                    .into_any()
                }
                Phase::Idle | Phase::Ended => {
                  view! {
                    <Button led=LedTone::Tx on_click=Callback::new(begin)>
                      {t("radio.voice-start")}
                    </Button>
                  }
                    .into_any()
                }
              }}
            </div>
            <p class="text-xs text-muted-foreground" role="status" data-testid="voice-mic">
              {move || match mic.get() {
                Mic::Recording => t("radio.voice-rec-on"),
                Mic::Unavailable => t("radio.voice-rec-off"),
                Mic::Off => String::new(),
              }}
            </p>
            {move || {
              audio_url
                .get()
                .map(|url| {
                  view! {
                    <div class=crate::kit::tile_class("space-y-2")>
                      <h3 class="text-xs text-muted-foreground">{t("radio.voice-playback")}</h3>
                      <audio controls=true src=url data-testid="voice-playback" class="w-full"></audio>
                    </div>
                  }
                })
            }}
            <div class=crate::kit::tile_class("space-y-2")>
              <h3 class="text-xs text-muted-foreground">{move || t("radio.voice-checklist")}</h3>
              <ul class="grid gap-x-4 gap-y-1 sm:grid-cols-2">
                {ITEMS
                  .into_iter()
                  .map(|item| {
                    let on = Signal::derive(move || ticks.with(|t| t.contains(&item)));
                    view! {
                      <li data-voice-item=item_id(item)>
                        <label class="flex items-start gap-2 text-sm">
                          <Checkbox
                            checked=on
                            on_change=Callback::new(move |v: bool| {
                              ticks.update(|t| {
                                t.retain(|i| *i != item);
                                if v {
                                  t.push(item);
                                }
                              })
                            })
                          />
                          <span class="min-w-0 break-words">{move || item_label(item)}</span>
                        </label>
                      </li>
                    }
                  })
                  .collect_view()}
              </ul>
            </div>
          </div>
        </section>

        {move || {
          result
            .get()
            .map(|r| {
              let ok = r.complete();
              let missing: Vec<String> = r.missing.iter().map(|i| item_label(*i)).collect();
              view! {
                <div data-testid="voice-result">
                  <SectionCard title=Signal::derive(move || t("radio.voice-result-heading"))>
                    <div class="space-y-3 p-4 text-sm" role="status">
                      {ok.then(|| view! { <p class="text-win-text">{t("radio.voice-result-ok")}</p> })}
                      {(!r.missing.is_empty())
                        .then(|| {
                          view! {
                            <p class="text-hp-text" data-testid="voice-missing">
                              {tf("radio.voice-result-missing", &[&missing.join(" · ")])}
                            </p>
                          }
                        })}
                      {(r.overtime_secs > 0)
                        .then(|| {
                          view! {
                            <p class="text-hp-text" data-testid="voice-overtime">
                              {tf(
                                "radio.voice-result-over",
                                &[&r.overtime_secs.to_string(), &LIMIT_SECS.to_string()],
                              )}
                            </p>
                          }
                        })}
                      <div class=crate::kit::tile_class("space-y-2")>
                        <h3 class="text-xs text-muted-foreground">{t("radio.voice-model")}</h3>
                        <p
                          class="break-words leading-relaxed"
                          data-testid="voice-model"
                          lang=move || lang.get().speech_tag()
                        >
                          {move || scenario.with(|s| s.model_reply(lang.get()))}
                        </p>
                      </div>
                    </div>
                  </SectionCard>
                </div>
              }
            })
        }}

        <p class="text-xs text-muted-foreground">{move || t("radio.voice-note")}</p>
      </PageContainer>
    </div>
  }
}

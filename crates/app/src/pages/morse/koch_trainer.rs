use std::time::Duration;

use ham_web_core::koch::{GroupResult, KochProgress, MIN_CHARS, PASS_RATE, farnsworth, max_level};
use leptos::html;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::morse_audio::play_morse_timed_with;
use crate::morse_settings::use_morse_settings;
use crate::ui::{
  Button, ButtonKind, ControlSize, Input, Size, Slider, Tooltip, Variant, card_title_class,
};
use crate::util::{random, storage};

use super::encode_words;
use super::morse_key::MorseKey;
use crate::i18n::{t, tf, tp};

const KEY: &str = "morse-koch";
/// 每轮的组数与每组字符数。
const GROUPS: usize = 2;
const GROUP_LEN: usize = 5;

fn save(p: &KochProgress) {
  storage::set_json(KEY, p);
}

/// Koch 法抄收训练：从 2 个字符起步，当前级别正确率达标后自动加入新字符；支持 Farnsworth 间隔，
/// 并统计每个字符的错误率。
#[component]
pub(super) fn KochTrainer() -> impl IntoView {
  let settings = use_morse_settings();
  let progress = RwSignal::new(storage::get_json::<KochProgress>(KEY).unwrap_or_default());
  let new_round = move || progress.with_untracked(|p| p.generate(GROUPS, GROUP_LEN, &mut random));
  let text = RwSignal::new(new_round());
  let answer = RwSignal::new(String::new());
  let result = RwSignal::new(None::<GroupResult>);
  let input_ref = NodeRef::<html::Input>::new();

  let playing = RwSignal::new(false);
  let play_gen = RwSignal::new(0u32);
  let play = move || {
    let (c, e) = progress.with_untracked(|p| (p.char_wpm, p.effective_wpm));
    let seq = play_gen.get_untracked().wrapping_add(1);
    play_gen.set(seq);
    let dur = play_morse_timed_with(
      &encode_words(&text.get_untracked()),
      farnsworth(f64::from(c), f64::from(e)),
      settings.tone_hz(),
      settings.volume(),
    );
    playing.set(true);
    set_timeout(
      move || {
        if play_gen.get_untracked() == seq {
          playing.set(false);
        }
      },
      Duration::from_secs_f64(dur.max(0.12)),
    );
    if let Some(el) = input_ref.get_untracked() {
      let _ = el.focus();
    }
  };
  let next = move || {
    text.set(new_round());
    answer.set(String::new());
    result.set(None);
    play();
  };
  let submit = move || {
    if result.with_untracked(Option::is_some) || answer.with_untracked(|a| a.trim().is_empty()) {
      return;
    }
    let mut r = None;
    progress.update(|p| r = Some(p.record(&text.get_untracked(), &answer.get_untracked())));
    save(&progress.get_untracked());
    result.set(r);
  };
  let set_level = move |delta: isize| {
    progress.update(|p| p.set_level(p.level.saturating_add_signed(delta)));
    save(&progress.get_untracked());
    text.set(new_round());
    answer.set(String::new());
    result.set(None);
  };
  let reset_progress = move || {
    progress.set(KochProgress::default());
    save(&progress.get_untracked());
    text.set(new_round());
    answer.set(String::new());
    result.set(None);
  };
  let speed = move |label: Signal<String>,
                    min: u32,
                    get: fn(&KochProgress) -> u32,
                    set: fn(&mut KochProgress, u32)| {
    view! {
      // `min-w-0`（而不是 `shrink-0`）：这一行要能被收缩，否则窄屏下「标签 + 滑块 + 读数」
      // 会顶出卡片，整页横向滚动。
      <div class="flex min-w-0 items-center gap-2">
        <span class="whitespace-nowrap text-xs text-muted-foreground">{move || label.get()}</span>
        <Slider
          value=Signal::derive(move || f64::from(progress.with(get)))
          on_change=Callback::new(move |v: f64| {
            progress.update(|p| {
              set(p, v.round() as u32);
              p.effective_wpm = p.effective_wpm.min(p.char_wpm);
            });
            save(&progress.get_untracked());
          })
          min=f64::from(min)
          max=35.0
          step=1.0
          aria_label=label
          aria_valuetext=Signal::derive(move || format!("{} WPM", progress.with(get)))
          class="w-28"
        />
        <span class="w-14 text-xs tabular-nums text-muted-foreground">{move || progress.with(get)} " WPM"</span>
      </div>
    }
  };

  view! {
    <section class="pxl-card border bg-card">
      <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
        <h2 class=card_title_class("mr-auto")>
          {move || t("morse.koch-method-copying-practice")}
          <span class="ml-2 text-xs font-normal text-muted-foreground">
            {move || tf("common.level", &[&(progress.with(|p| p.level)).to_string(), &(max_level()).to_string()])}
          </span>
        </h2>
        <Button
          variant=Variant::Outline
          size=Size::Sm
          title=Signal::derive(move || t("morse.remove-one-character"))
          on_click=Callback::new(move |_| set_level(-1))
        >
          {move || t("morse.character-minus")}
        </Button>
        <Button
          variant=Variant::Outline
          size=Size::Sm
          title=Signal::derive(move || t("morse.skip-this-level-and"))
          on_click=Callback::new(move |_| set_level(1))
        >
          {move || t("morse.character")}
        </Button>
        <Button
          variant=Variant::Outline
          size=Size::Sm
          title=Signal::derive(move || t("morse.reset-progress"))
          on_click=Callback::new(move |_| reset_progress())
        >
          {move || t("morse.reset")}
        </Button>
      </div>

      <div class="space-y-4 p-4">
        <div class="flex flex-wrap items-center gap-x-5 gap-y-2">
          {speed(Signal::derive(move || t("morse.character-speed")), 12, |p| p.char_wpm, |p, v| p.char_wpm = v)}
          {speed(Signal::derive(move || t("morse.effective-speed")), 5, |p| p.effective_wpm, |p, v| p.effective_wpm = v)}
        </div>

        <div class="flex flex-wrap items-center gap-1.5">
          <span class="mr-1 text-xs text-muted-foreground">{move || t("morse.learned-characters")}</span>
          {move || {
            progress.with(|p| {
              let newest = p.newest();
              p.chars()
                .into_iter()
                .map(|c| {
                  let err = p.chars.get(&c).and_then(|t| t.error_rate());
                  let class = if Some(c) == newest {
                    "border-primary bg-primary/10 text-primary"
                  } else if err.is_some_and(|e| e > 0.2) {
                    "border-destructive/40 bg-destructive/10 text-hp-text"
                  } else {
                    "text-foreground"
                  };
                  let tip = err.map_or_else(
                    || t("morse.not-practised-yet"),
                    |e| tf("common.error-rate", &[&format!("{:.0}", e * 100.0)]),
                  );
                  view! {
                    <Tooltip label=tip>
                      <span class=format!("border px-1.5 py-0.5 font-mono text-sm {class}")>
                        {c.to_string()}
                      </span>
                    </Tooltip>
                  }
                })
                .collect_view()
            })
          }}
        </div>

        <div class="flex flex-col items-center gap-3 border bg-inset px-4 py-6">
          <MorseKey
            class="size-16"
            active=playing
            aria_label=Signal::derive(move || t("morse.play"))
            on_click=Callback::new(move |()| play())
          >
            <span class="relative">
              {move || {
                if playing.get() {
                  view! { <Icon kind=IconKind::AudioLines class="size-6" /> }.into_any()
                } else {
                  view! { <Icon kind=IconKind::Play class="size-6" /> }.into_any()
                }
              }}
            </span>
          </MorseKey>
          <div class="text-xs text-muted-foreground">
            {move || tp("common.each-round-has-groups", GROUPS, &[&(GROUPS).to_string(), &(GROUP_LEN).to_string()])}
          </div>
          <form
            class="flex flex-wrap items-center justify-center gap-2"
            on:submit=move |e| {
              e.prevent_default();
              if result.with_untracked(Option::is_some) { next() } else { submit() }
            }
          >
            <Input
              node_ref=input_ref
              value=answer
              on_change=Callback::new(move |v: String| answer.set(v))
              size=ControlSize::Lg
              placeholder=Signal::derive(move || t("morse.copied-text"))
              autocomplete="off"
              class="h-10 w-56 text-center font-mono text-lg md:text-lg uppercase tracking-widest"
            />
            <Button
              kind=ButtonKind::Submit
              variant=Variant::Default
              size=Size::Default
              class="h-10"
            >
              {move || if result.get().is_some() { t("morse.next-round") } else { t("learning.submit") }}
            </Button>
          </form>
          {move || {
            result.get().map(|r| {
              let correct = r.correct();
              let total = r.marks.len();
              view! {
                <div class="flex flex-col items-center gap-1.5" aria-live="polite">
                  <div class="flex flex-wrap justify-center gap-1 font-mono text-lg">
                    {r.marks
                      .chunks(GROUP_LEN)
                      .map(|group| view! {
                        <span class="mx-1.5 inline-flex gap-1">
                          {group
                            .iter()
                            .map(|&(c, ok)| view! {
                              <span class={if ok { "text-win-text" } else { "bg-destructive/15 px-0.5 text-hp-text" }}>
                                {c.to_string()}
                              </span>
                            })
                            .collect_view()}
                        </span>
                      })
                      .collect_view()}
                  </div>
                  <div class="text-sm">
                    {tp("common.copied-correct", total as u32, &[&(correct).to_string(), &(total).to_string()])}
                    {r.leveled_up.then(|| view! {
                      <span class="ml-2 text-win-text">
                        {move || tf("common.level-up-new-character", &[&(progress.with(|p| p.newest().map(String::from).unwrap_or_default())).to_string()])}
                      </span>
                    })}
                  </div>
                </div>
              }
            })
          }}
        </div>

        <div class="space-y-1.5">
          {move || {
            progress.with(|p| {
              let t = p.level_tally;
              let rate = p.level_rate().unwrap_or(0.0) * 100.0;
              let pct = (f64::from(t.sent) / f64::from(MIN_CHARS) * 100.0).min(100.0);
              view! {
                <div class="flex items-center justify-between text-xs text-muted-foreground">
                  <span>
                    {tp("common.this-level-characters-accuracy", MIN_CHARS, &[&(t.sent).to_string(), &(MIN_CHARS).to_string(), &format!("{rate:.0}"), &format!("{:.0}", PASS_RATE * 100.0)])}
                  </span>
                </div>
                <div class="h-2 w-full overflow-hidden bg-muted">
                  <div
                    class={if rate >= PASS_RATE * 100.0 { "h-full bg-win/70" } else { "h-full bg-primary" }}
                    style=format!("width: {pct:.1}%")
                  ></div>
                </div>
              }
            })
          }}
        </div>

        {move || {
          progress.with(|p| {
            let mut rows: Vec<(char, f64, u32)> = p
              .chars()
              .into_iter()
              .filter_map(|c| {
                let t = p.chars.get(&c)?;
                Some((c, t.error_rate()?, t.sent))
              })
              .collect();
            rows.sort_by(|a, b| b.1.total_cmp(&a.1));
            (!rows.is_empty()).then(|| view! {
              <div>
                <h3 class="mb-2 text-xs text-muted-foreground">{move || t("morse.character-error-rate-high")}</h3>
                <div class="grid grid-cols-4 gap-x-4 gap-y-1 text-xs sm:grid-cols-6 lg:grid-cols-8">
                  {rows
                    .into_iter()
                    .map(|(c, err, sent)| {
                      let tip = tf("common.total", &[&(sent).to_string()]);
                      view! {
                        <Tooltip label=tip>
                          <div class="flex items-baseline gap-2">
                            <span class="font-mono">{c.to_string()}</span>
                            <span class={if err > 0.2 { "tabular-nums text-hp-text" } else { "tabular-nums text-muted-foreground" }}>
                              {format!("{:.0}%", err * 100.0)}
                            </span>
                          </div>
                        </Tooltip>
                      }
                    })
                    .collect_view()}
                </div>
              </div>
            })
          })
        }}

        <p class="text-xs text-muted-foreground">
          {move || t("morse.koch-method-recognise-characters")}
        </p>
      </div>
    </section>
  }
}

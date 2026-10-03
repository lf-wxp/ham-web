use std::time::Duration;

use ham_web_core::koch::{GroupResult, KochProgress, MIN_CHARS, PASS_RATE, farnsworth, max_level};
use leptos::html;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::morse_audio::play_morse_timed_with;
use crate::morse_settings::use_morse_settings;
use crate::ui::{Button, ButtonKind, ControlSize, Input, Size, Variant};
use crate::util::{random, storage};

use super::{encode_words, pill_class};
use crate::i18n::{t, tf};

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
  let speed = move |label: String,
                    min: u32,
                    get: fn(&KochProgress) -> u32,
                    set: fn(&mut KochProgress, u32)| {
    view! {
      <div class="flex shrink-0 items-center gap-2">
        <span class="whitespace-nowrap text-xs text-muted-foreground">{label.clone()}</span>
        <input
          type="range"
          aria-label=label.clone()
          min=min
          max="35"
          step="1"
          prop:value=move || progress.with(get).to_string()
          aria-valuetext=move || format!("{} WPM", progress.with(get))
          on:input=move |e| {
            if let Ok(v) = event_target_value(&e).parse::<u32>() {
              progress.update(|p| {
                set(p, v);
                p.effective_wpm = p.effective_wpm.min(p.char_wpm);
              });
              save(&progress.get_untracked());
            }
          }
          class="h-1.5 w-28 accent-primary"
        />
        <span class="w-14 text-xs tabular-nums text-muted-foreground">{move || progress.with(get)} " WPM"</span>
      </div>
    }
  };

  view! {
    <section class="rounded-xl border bg-card">
      <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
        <h2 class="mr-auto text-sm font-semibold">
          {move || t("Koch 法抄收训练")}
          <span class="ml-2 text-xs font-normal text-muted-foreground">
            {move || tf("第 {} / {} 级", &[&(progress.with(|p| p.level)).to_string(), &(max_level()).to_string()])}
          </span>
        </h2>
        <button type="button" class=pill_class(false) on:click=move |_| set_level(-1) title=move || t("减少一个字符")>{move || t("− 字符")}</button>
        <button type="button" class=pill_class(false) on:click=move |_| set_level(1) title=move || t("跳过本级，直接加入下一个字符")>{move || t("+ 字符")}</button>
        <button type="button" class=pill_class(false) on:click=move |_| reset_progress() title=move || t("重置学习进度")>{move || t("重置")}</button>
      </div>

      <div class="space-y-4 p-4">
        <div class="flex flex-wrap items-center gap-x-5 gap-y-2">
          {speed(t("字符速度"), 12, |p| p.char_wpm, |p, v| p.char_wpm = v)}
          {speed(t("有效速度"), 5, |p| p.effective_wpm, |p, v| p.effective_wpm = v)}
        </div>

        <div class="flex flex-wrap items-center gap-1.5">
          <span class="mr-1 text-xs text-muted-foreground">{move || t("已学字符")}</span>
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
                    "border-red-500/40 bg-red-500/10 text-red-700 dark:text-red-300"
                  } else {
                    "text-foreground"
                  };
                  view! {
                    <span
                      class=format!("rounded border px-1.5 py-0.5 font-mono text-sm {class}")
                      title=err.map_or_else(|| t("尚未练习"), |e| tf("错误率 {}%", &[&format!("{:.0}", e * 100.0)]))
                    >
                      {c.to_string()}
                    </span>
                  }
                })
                .collect_view()
            })
          }}
        </div>

        <div class="flex flex-col items-center gap-3 rounded-xl border bg-muted/30 px-4 py-6">
          <button
            type="button"
            aria-label=move || t("播放")
            on:click=move |_| play()
            class="relative flex size-16 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg shadow-primary/25 transition-all duration-200 ease-out hover:scale-105 hover:shadow-xl hover:shadow-primary/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:ring-offset-2 focus-visible:ring-offset-background active:scale-95"
          >
            {move || {
              playing.get().then(|| {
                view! {
                  <span class="absolute inset-0 rounded-full bg-primary/40 animate-ping [animation-duration:1.4s]"></span>
                }
              })
            }}
            <span class="relative">
              {move || {
                if playing.get() {
                  view! { <Icon kind=IconKind::AudioLines class="h-7 w-7" /> }.into_any()
                } else {
                  view! { <Icon kind=IconKind::Play class="h-7 w-7" /> }.into_any()
                }
              }}
            </span>
          </button>
          <div class="text-xs text-muted-foreground">
            {tf("每轮 {} 组、每组 {} 个字符，边听边抄，听完再提交", &[&(GROUPS).to_string(), &(GROUP_LEN).to_string()])}
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
              placeholder=Signal::derive(move || t("抄收内容"))
              autocomplete="off"
              class="h-10 w-56 rounded-lg text-center font-mono text-lg md:text-lg font-semibold uppercase tracking-widest"
            />
            <Button
              kind=ButtonKind::Submit
              variant=Variant::Default
              size=Size::Default
              class="rounded-lg h-10"
            >
              {move || if result.get().is_some() { t("下一轮") } else { t("提交") }}
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
                              <span class={if ok { "text-emerald-600 dark:text-emerald-400" } else { "rounded bg-red-500/15 px-0.5 text-red-600 dark:text-red-400" }}>
                                {c.to_string()}
                              </span>
                            })
                            .collect_view()}
                        </span>
                      })
                      .collect_view()}
                  </div>
                  <div class="text-sm">
                    {tf("抄对 {} / {}", &[&(correct).to_string(), &(total).to_string()])}
                    {r.leveled_up.then(|| view! {
                      <span class="ml-2 font-semibold text-emerald-600 dark:text-emerald-400">
                        {move || tf("升级！新字符 {}", &[&(progress.with(|p| p.newest().map(String::from).unwrap_or_default())).to_string()])}
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
                    {tf("本级已抄 {} / {} 字 · 正确率 {}%（需 ≥ {}%）", &[&(t.sent).to_string(), &(MIN_CHARS).to_string(), &format!("{rate:.0}"), &format!("{:.0}", PASS_RATE * 100.0)])}
                  </span>
                </div>
                <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
                  <div
                    class={if rate >= PASS_RATE * 100.0 { "h-full rounded-full bg-emerald-500" } else { "h-full rounded-full bg-primary" }}
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
                <h3 class="mb-2 text-xs font-medium text-muted-foreground">{move || t("字符错误率（高 → 低）")}</h3>
                <div class="grid grid-cols-4 gap-x-4 gap-y-1 text-xs sm:grid-cols-6 lg:grid-cols-8">
                  {rows
                    .into_iter()
                    .map(|(c, err, sent)| view! {
                      <div class="flex items-baseline gap-2" title=tf("共 {} 次", &[&(sent).to_string()])>
                        <span class="font-mono font-semibold">{c.to_string()}</span>
                        <span class={if err > 0.2 { "tabular-nums text-red-600 dark:text-red-400" } else { "tabular-nums text-muted-foreground" }}>
                          {format!("{:.0}%", err * 100.0)}
                        </span>
                      </div>
                    })
                    .collect_view()}
                </div>
              </div>
            })
          })
        }}

        <p class="text-xs text-muted-foreground">
          {move || t("Koch 法：直接用目标字符速度（建议 ≥ 18 WPM）听辨，从 2 个字符开始，本级正确率达到 90% 后自动加入下一个字符，最新字符与易错字符出现得更多。有效速度低于字符速度时启用 Farnsworth 间隔：字符本身不变慢，只拉长字符与组之间的停顿。")}
        </p>
      </div>
    </section>
  }
}

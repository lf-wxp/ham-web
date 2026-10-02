use std::time::Duration;

use ham_web_core::cw_runner::{Exchange, MAX_WPM, MIN_WPM, RunnerStats, adapt_wpm, check};
use ham_web_core::koch::farnsworth;
use leptos::html;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::morse_audio::{play_morse_timed_with, play_pileup};
use crate::morse_settings::use_morse_settings;
use crate::util::{random, storage};

use super::{btn_primary, encode_words, pill_class};

use super::callsign_session::{KEY, Logged, SESSION_QSOS, Session, save};
use crate::i18n::{t, tf};

/// 竞赛模拟抄收：对方发 `呼号 5NN 序号`，抄下呼号与序号后提交，每轮 10 个通联计分。
#[component]
pub(super) fn CallsignRunner() -> impl IntoView {
  let settings = use_morse_settings();
  let stats = RwSignal::new(storage::get_json::<RunnerStats>(KEY).unwrap_or_default());
  let session = RwSignal::new(None::<Session>);
  let call = RwSignal::new(String::new());
  let serial = RwSignal::new(String::new());
  let wpm_delta = RwSignal::new(0i32);
  let revealed = RwSignal::new(None::<Exchange>);
  let call_ref = NodeRef::<html::Input>::new();
  let serial_ref = NodeRef::<html::Input>::new();

  // 等输入框随状态解除 disabled 后再聚焦
  let focus = |r: NodeRef<html::Input>| {
    request_animation_frame(move || {
      if let Some(el) = r.get_untracked() {
        let _ = el.focus();
      }
    });
  };
  let playing = RwSignal::new(false);
  let play_gen = RwSignal::new(0u32);
  let play = move || {
    let Some((decoy, target)) = session.with_untracked(|s| {
      s.as_ref().map(|s| {
        let cut = stats.with_untracked(|st| st.cut_numbers);
        (
          s.decoy.as_ref().map(|d| encode_words(&d.text(cut))),
          encode_words(&s.current.text(cut)),
        )
      })
    }) else {
      return;
    };
    let seq = play_gen.get_untracked().wrapping_add(1);
    play_gen.set(seq);
    let wpm = f64::from(stats.with_untracked(|s| s.wpm));
    let timing = farnsworth(wpm, wpm);
    let dur = match decoy {
      Some(d) => play_pileup(&d, &target, timing, settings.tone_hz(), settings.volume()),
      None => play_morse_timed_with(&target, timing, settings.tone_hz(), settings.volume()),
    };
    playing.set(true);
    set_timeout(
      move || {
        if play_gen.get_untracked() == seq {
          playing.set(false);
        }
      },
      Duration::from_secs_f64(dur.max(0.12)),
    );
  };
  let start = move || {
    let pileup = stats.with_untracked(|st| st.pileup);
    session.set(Some(Session {
      current: Exchange::random(0, &mut random),
      decoy: pileup.then(|| Exchange::random(0, &mut random)),
      replays: 0,
      skipped: 0,
      log: Vec::new(),
    }));
    revealed.set(None);
    call.set(String::new());
    serial.set(String::new());
    focus(call_ref);
    play();
  };
  let replay = move || {
    session.update(|s| {
      if let Some(s) = s {
        s.replays += 1;
      }
    });
    focus(call_ref);
    play();
  };
  // 跳过当前通联（不计分）。
  let skip = move || {
    let pileup = stats.with_untracked(|st| st.pileup);
    let mut advanced = false;
    session.update(|s| {
      if let Some(s) = s
        && !s.finished()
      {
        s.skipped += 1;
        s.current = Exchange::random(s.attempted(), &mut random);
        s.decoy = pileup.then(|| Exchange::random(0, &mut random));
        s.replays = 0;
        advanced = true;
      }
    });
    revealed.set(None);
    call.set(String::new());
    serial.set(String::new());
    if advanced {
      focus(call_ref);
      play();
    }
  };
  // 看答案：显示当前台呼号并跳到下一个（不计分）。
  let reveal = move || {
    let ans = session.with_untracked(|s| s.as_ref().map(|s| s.current.clone()));
    let pileup = stats.with_untracked(|st| st.pileup);
    let mut advanced = false;
    session.update(|s| {
      if let Some(s) = s
        && !s.finished()
      {
        s.skipped += 1;
        s.current = Exchange::random(s.attempted(), &mut random);
        s.decoy = pileup.then(|| Exchange::random(0, &mut random));
        s.replays = 0;
        advanced = true;
      }
    });
    revealed.set(ans);
    call.set(String::new());
    serial.set(String::new());
    if advanced {
      focus(call_ref);
      play();
    }
  };
  // 立即结束本轮（剩余通联记为跳过）。
  let finish = move || {
    let mut score = None;
    session.update(|s| {
      if let Some(s) = s {
        if !s.finished() {
          s.skipped = SESSION_QSOS - s.log.len() as u32;
        }
        score = Some(s.score());
      }
    });
    if let Some(score) = score {
      stats.update(|st| st.finish_session(score));
    }
    save(&stats.get_untracked());
    call.set(String::new());
    serial.set(String::new());
  };
  let submit = move || {
    if call.with_untracked(|c| c.trim().is_empty()) {
      focus(call_ref);
      return;
    }
    let Some(mut s) = session.get_untracked().filter(|s| !s.finished()) else {
      return;
    };
    let before = stats.with_untracked(|st| st.wpm);
    let wpm = before;
    let result = check(&s.current, &call.get_untracked(), &serial.get_untracked());
    stats.update(|st| {
      st.record(&result, wpm);
      if st.adaptive {
        st.wpm = adapt_wpm(st.wpm, &result, s.replays);
      }
    });
    let after = stats.with_untracked(|st| st.wpm);
    wpm_delta.set(i64::from(after) as i32 - i64::from(before) as i32);
    s.log.push(Logged {
      ex: s.current.clone(),
      result,
      call: call.get_untracked().trim().to_ascii_uppercase(),
      serial: serial.get_untracked().trim().to_ascii_uppercase(),
      wpm,
      replays: s.replays,
    });
    let done = s.finished();
    if done {
      let score = s.score();
      stats.update(|st| st.finish_session(score));
    } else {
      let pileup = stats.with_untracked(|st| st.pileup);
      s.current = Exchange::random(s.attempted(), &mut random);
      s.decoy = pileup.then(|| Exchange::random(0, &mut random));
      s.replays = 0;
    }
    save(&stats.get_untracked());
    session.set(Some(s));
    revealed.set(None);
    call.set(String::new());
    serial.set(String::new());
    if !done {
      focus(call_ref);
      set_timeout(play, Duration::from_millis(500));
    }
  };

  let toggle =
    move |label: &'static str, get: fn(&RunnerStats) -> bool, set: fn(&mut RunnerStats, bool)| {
      view! {
        <label class="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
          <input
            type="checkbox"
            class="accent-primary"
            prop:checked=move || stats.with(get)
            on:change=move |e| {
              let on = event_target_checked(&e);
              stats.update(|s| set(s, on));
              save(&stats.get_untracked());
            }
          />
          {label}
        </label>
      }
    };

  let last = move || session.with(|s| s.as_ref().and_then(|s| s.log.last().cloned()));
  let in_progress = move || session.with(|s| s.as_ref().is_some_and(|s| !s.finished()));

  view! {
    <section class="rounded-xl border bg-card">
      <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
        <h2 class="mr-auto text-sm font-semibold">
          {move || t("呼号抄收 · 竞赛模拟")}
          <span class="ml-2 text-xs font-normal text-muted-foreground">
            {move || {
              let s = stats.get();
              tf("最高 {} 分 · 抄对最高 {} WPM", &[&(s.best_score).to_string(), &(s.top_wpm).to_string()])
            }}
          </span>
        </h2>
        <button
          type="button"
          on:click=move |_| {
            stats.set(RunnerStats::default());
            save(&stats.get_untracked());
          }
          class="rounded-md px-2 py-0.5 text-xs text-muted-foreground transition-all duration-200 hover:bg-accent hover:text-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
        >
          {move || t("重置成绩")}
        </button>
      </div>

      <div class="space-y-4 p-4">
        <div class="flex flex-wrap items-center gap-x-5 gap-y-2">
          <div class="flex items-center gap-2">
            <span class="text-xs text-muted-foreground">{move || t("速度")}</span>
            <input
              type="range"
              aria-label=move || t("速度")
              min=MIN_WPM
              max=MAX_WPM
              step="1"
              prop:value=move || stats.with(|s| s.wpm).to_string()
              aria-valuetext=move || format!("{} WPM", stats.with(|s| s.wpm))
              on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse::<u32>() {
                  stats.update(|s| s.wpm = v);
                  save(&stats.get_untracked());
                }
              }
              class="h-1.5 w-32 accent-primary"
            />
            <span class="w-14 text-xs tabular-nums text-muted-foreground">{move || stats.with(|s| s.wpm)} " WPM"</span>
          </div>
          {toggle("自适应速度", |s| s.adaptive, |s, v| s.adaptive = v)}
          {toggle("截短数字（T=0 N=9）", |s| s.cut_numbers, |s, v| s.cut_numbers = v)}
          {toggle("叠听（多一个干扰台）", |s| s.pileup, |s, v| s.pileup = v)}
        </div>

        <div class="flex flex-col items-center gap-3 rounded-xl border bg-muted/30 px-4 py-6">
          {move || {
            if in_progress() {
              let (n, score) = session.with(|s| s.as_ref().map_or((0, 0), |s| (s.attempted() + 1, s.score())));
              view! {
                <div class="flex flex-wrap items-center justify-center gap-x-4 gap-y-1 text-sm">
                  <span class="tabular-nums">{tf("第 {} / {} 个通联", &[&(n).to_string(), &(SESSION_QSOS).to_string()])}</span>
                  <span class="tabular-nums text-muted-foreground">{tf("得分 {}", &[&(score).to_string()])}</span>
                  {move || {
                    let d = wpm_delta.get();
                    (d != 0).then(|| {
                      let cls = if d > 0 {
                        "tabular-nums text-emerald-600 dark:text-emerald-400"
                      } else {
                        "tabular-nums text-red-600 dark:text-red-400"
                      };
                      let sign = if d > 0 { "+" } else { "" };
                      view! {
                        <span class=cls>
                          {tf("速度 {}{} WPM", &[(sign), &(d).to_string()])}
                        </span>
                      }
                    })
                  }}
                </div>
                <div class="flex flex-wrap items-center justify-center gap-2">
                  <button
                    type="button"
                    on:click=move |_| replay()
                    class="inline-flex items-center gap-2 rounded-full border bg-card px-4 py-1.5 text-sm transition-all duration-200 ease-out hover:border-primary/40 hover:bg-accent hover:text-accent-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
                  >
                    {move || {
                      if playing.get() {
                        view! { <Icon kind=IconKind::AudioLines class="h-4 w-4 animate-pulse" /> }.into_any()
                      } else {
                        view! { <Icon kind=IconKind::Play class="h-4 w-4" /> }.into_any()
                      }
                    }}
                    {move || t("重听")}
                  </button>
                  <button
                    type="button"
                    on:click=move |_| skip()
                    class="inline-flex items-center gap-2 rounded-full border bg-card px-4 py-1.5 text-sm transition-all duration-200 ease-out hover:border-primary/40 hover:bg-accent hover:text-accent-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
                  >
                    {move || t("跳过")}
                  </button>
                  <button
                    type="button"
                    on:click=move |_| finish()
                    class="inline-flex items-center gap-2 rounded-full border bg-card px-4 py-1.5 text-sm transition-all duration-200 ease-out hover:border-primary/40 hover:bg-accent hover:text-accent-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
                  >
                    {move || t("结束本轮")}
                  </button>
                  <button
                    type="button"
                    on:click=move |_| reveal()
                    class="inline-flex items-center gap-2 rounded-full border bg-card px-4 py-1.5 text-sm transition-all duration-200 ease-out hover:border-primary/40 hover:bg-accent hover:text-accent-foreground active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
                  >
                    {move || t("看答案")}
                  </button>
                </div>
                {move || {
                  stats.with(|s| s.pileup).then(|| {
                    view! {
                      <div class="text-xs text-amber-700 dark:text-amber-300">{move || t("叠听：干扰台音调较低，请抄音调较高的目标台")}</div>
                    }
                  })
                }}
              }
              .into_any()
            } else {
              view! {
                <button
                  type="button"
                  on:click=move |_| start()
                  class="inline-flex items-center justify-center gap-2 rounded-lg bg-primary px-5 py-2 text-sm font-medium text-primary-foreground shadow-sm shadow-primary/20 transition-all duration-200 ease-out hover:bg-primary/90 hover:shadow-md hover:shadow-primary/25 active:scale-[0.97] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:ring-offset-2 focus-visible:ring-offset-background"
                >
                  {move || if session.with(Option::is_some) { t("再来一轮") } else { t("开始一轮（10 个通联）") }}
                </button>
              }
              .into_any()
            }
          }}
          <form
            class="flex flex-wrap items-end justify-center gap-2"
            on:submit=move |e| {
              e.prevent_default();
              submit();
            }
          >
            <label class="flex flex-col gap-1 text-xs text-muted-foreground">
              {move || t("呼号")}
              <input
                node_ref=call_ref
                prop:value=move || call.get()
                on:input=move |e| call.set(event_target_value(&e))
                on:keydown=move |e| {
                  if e.key() == "Enter" && serial.with_untracked(String::is_empty) {
                    e.prevent_default();
                    focus(serial_ref);
                  }
                }
                prop:disabled=move || !in_progress()
                autocomplete="off"
                autocapitalize="characters"
                spellcheck="false"
                class="h-10 w-40 rounded-lg border bg-background px-3 text-center font-mono text-lg font-semibold uppercase tracking-widest outline-none transition-[border-color,box-shadow] duration-200 focus-visible:border-primary/50 focus-visible:ring-2 focus-visible:ring-ring/60 disabled:cursor-not-allowed disabled:opacity-50"
              />
            </label>
            <label class="flex flex-col gap-1 text-xs text-muted-foreground">
              {move || t("序号")}
              <input
                node_ref=serial_ref
                prop:value=move || serial.get()
                on:input=move |e| serial.set(event_target_value(&e))
                prop:disabled=move || !in_progress()
                autocomplete="off"
                inputmode="numeric"
                class="h-10 w-24 rounded-lg border bg-background px-3 text-center font-mono text-lg font-semibold uppercase tracking-widest outline-none transition-[border-color,box-shadow] duration-200 focus-visible:border-primary/50 focus-visible:ring-2 focus-visible:ring-ring/60 disabled:cursor-not-allowed disabled:opacity-50"
              />
            </label>
            <button
              type="submit"
              prop:disabled=move || !in_progress()
              class=btn_primary("")
            >
              {move || t("记录")}
            </button>
          </form>
          <div aria-live="polite" class="min-h-5 text-sm">
            {move || revealed.get().map(|ex| {
              view! {
                <span class="text-muted-foreground">
                  {move || t("答案 ")} <span class="font-mono font-semibold">{format!("{} 5NN {}", ex.call, ex.serial)}</span>
                </span>
              }
            })}
            {move || last().map(|l| {
              let ok = l.result.call_ok && l.result.serial_ok;
              view! {
                <span class=if ok { "text-emerald-600 dark:text-emerald-400" } else { "text-red-600 dark:text-red-400" }>
                  {if ok { "✓ " } else { "✗ " }}
                  <span class="font-mono">{format!("{} 5NN {}", l.ex.call, l.ex.serial)}</span>
                </span>
                {(!ok).then(|| view! {
                  <span class="ml-2 text-xs text-muted-foreground">
                    {tf("你抄的：{} {}", &[(if l.call.is_empty() { "—" } else { &l.call }), (if l.serial.is_empty() { "—" } else { &l.serial })])}
                  </span>
                })}
              }
            })}
          </div>
          <p class="text-xs text-muted-foreground">{move || t("抄完呼号按回车跳到序号，再按回车记录；呼号对得 2 分，序号也对再加 1 分。")}</p>
        </div>

        {move || {
          session.with(|s| s.as_ref().filter(|s| s.finished()).map(|s| {
            let score = s.score();
            let best = stats.with(|st| st.best_score);
            view! {
              <div class="space-y-2">
                <div class="text-sm font-medium">
                  {tf("本轮得分 {} / {}", &[&(score).to_string(), &(SESSION_QSOS * 3).to_string()])}
                  {(score >= best && score > 0).then(|| view! {
                    <span class="ml-2 text-emerald-600 dark:text-emerald-400">{move || t("新纪录！")}</span>
                  })}
                </div>
                <div class="overflow-x-auto">
                  <table class="w-full text-left text-xs">
                    <thead class="text-muted-foreground">
                      <tr>
                        <th class="py-1 pr-3 font-normal">"#"</th>
                        <th class="py-1 pr-3 font-normal">{move || t("对方")}</th>
                        <th class="py-1 pr-3 font-normal">{move || t("你抄的")}</th>
                        <th class="py-1 pr-3 font-normal">{move || t("速度")}</th>
                        <th class="py-1 pr-3 font-normal">{move || t("重听")}</th>
                        <th class="py-1 font-normal">{move || t("得分")}</th>
                      </tr>
                    </thead>
                    <tbody class="font-mono">
                      {s.log
                        .iter()
                        .enumerate()
                        .map(|(i, l)| view! {
                          <tr class="border-t">
                            <td class="py-1 pr-3 text-muted-foreground">{i + 1}</td>
                            <td class="py-1 pr-3">{format!("{} {}", l.ex.call, l.ex.serial)}</td>
                            <td class=if l.result.call_ok { "py-1 pr-3" } else { "py-1 pr-3 text-red-600 dark:text-red-400" }>
                              {format!("{} {}", l.call, l.serial)}
                            </td>
                            <td class="py-1 pr-3 tabular-nums">{l.wpm}</td>
                            <td class="py-1 pr-3 tabular-nums">{l.replays}</td>
                            <td class="py-1 tabular-nums">{l.result.points}</td>
                          </tr>
                        })
                        .collect_view()}
                    </tbody>
                  </table>
                </div>
              </div>
            }
          }))
        }}

        {move || {
          let s = stats.get();
          (s.qsos > 0).then(|| {
            let rate = f64::from(s.calls_ok) / f64::from(s.qsos) * 100.0;
            let worst = s.worst_chars(8);
            view! {
              <div class="flex flex-wrap items-center gap-x-5 gap-y-1 text-xs text-muted-foreground">
                <span>{tf("累计 {} 轮 · {} 个通联 · 呼号正确率 {}%", &[&(s.sessions).to_string(), &(s.qsos).to_string(), &(rate).to_string()])}</span>
                {(!worst.is_empty()).then(|| view! {
                  <span class="flex flex-wrap items-center gap-1">
                    {move || t("常错字符")}
                    {worst
                      .into_iter()
                      .map(|(c, n)| view! {
                        <span class=pill_class(false) title=tf("抄错 {} 次", &[&(n).to_string()])>
                          <span class="font-mono font-semibold">{c.to_string()}</span>
                          <span class="ml-1 tabular-nums">{n}</span>
                        </span>
                      })
                      .collect_view()}
                  </span>
                })}
              </div>
            }
          })
        }}
      </div>
    </section>
  }
}

//! FT8 通话练习卡片：对方刚发了这句，你下一句该发什么。
//!
//! 剧本、判分与成绩全在核心 `ham_web_core::ft8_practice`（单测里每条报文都与 `ham-web-ft8` 的打包逻辑对拍）；
//! 这里只负责渲染、选择与输入。

use ham_web_core::ft8_practice::{Role, Scenario, Session, Speaker, Verdict, report_text};
use leptos::prelude::*;

use super::texts::{kind_name, mistake_text};
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::ui::{Button, ChipToggle, Field, Input, Size, Variant, card_title_class};

/// 上一次回复的反馈。
#[derive(Clone)]
enum Feedback {
  Right(String),
  Wrong(String),
}

#[component]
pub fn Ft8Practice() -> impl IntoView {
  let store = use_log_store();
  let role = RwSignal::new(Role::AnsweringCq);
  let new_session = move || {
    let me = store.active_station_untracked();
    // 种子取当前毫秒：每一轮的对方呼号、网格、报告都不同。
    let seed = js_sys::Date::now() as u64;
    Session::new(Scenario::generate(
      seed,
      role.get_untracked(),
      &me.callsign,
      &me.gridsquare,
    ))
  };
  let session = RwSignal::new(new_session());
  let feedback = RwSignal::new(None::<Feedback>);
  let input = RwSignal::new(String::new());
  let restart = move || {
    session.set(new_session());
    feedback.set(None);
    input.set(String::new());
  };

  let send = move |reply: String| match session.try_update(|s| s.answer(&reply)) {
    Some(Verdict::Correct { text, .. }) => {
      feedback.set(Some(Feedback::Right(text)));
      input.set(String::new());
    }
    Some(Verdict::Wrong(m)) => feedback.set(Some(Feedback::Wrong(mistake_text(&m)))),
    None => {}
  };

  let finished = Memo::new(move |_| session.with(Session::finished));
  let input_id = crate::util::unique_id("ft8-practice-input");
  let input_for = input_id.clone();

  view! {
    <section class=crate::kit::card_class() data-testid="ft8-practice">
      <div class="flex flex-wrap items-center gap-3 border-b px-4 py-3">
        <h2 class=card_title_class("min-w-0 flex-1")>{move || t("radio.practice-title")}</h2>
        <Button variant=Variant::Outline size=Size::Sm on_click=Callback::new(move |_| restart())>
          {move || t("radio.practice-new")}
        </Button>
      </div>
      <div class="space-y-3 p-4">
        <p class="text-xs text-muted-foreground">{move || t("radio.practice-intro")}</p>
        <div class="flex flex-wrap items-center gap-1.5">
          <ChipToggle
            active=Signal::derive(move || role.get() == Role::CallingCq)
            on_change=Callback::new(move |_| {
              role.set(Role::CallingCq);
              restart();
            })
          >
            {move || t("radio.practice-role-calling")}
          </ChipToggle>
          <ChipToggle
            active=Signal::derive(move || role.get() == Role::AnsweringCq)
            on_change=Callback::new(move |_| {
              role.set(Role::AnsweringCq);
              restart();
            })
          >
            {move || t("radio.practice-role-answering")}
          </ChipToggle>
        </div>

        <p class="text-xs" data-testid="practice-scenario">
          {move || {
            session.with(|s| {
              let sc = s.scenario();
              tf(
                "radio.practice-you-are",
                &[&sc.me, &sc.me_grid, &sc.them, &sc.them_grid],
              )
            })
          }}
          <span class="ml-2 text-muted-foreground">
            {move || {
              session
                .with(|s| tf("radio.practice-your-report", &[&report_text(s.scenario().my_report)]))
            }}
          </span>
        </p>

        <div class="space-y-1.5">
          <h3 class="text-xs text-muted-foreground">{move || t("learning.qsos")}</h3>
          <ol class="space-y-1" data-testid="practice-log">
            {move || {
              session
                .with(|s| s.log().to_vec())
                .into_iter()
                .map(|e| {
                  let who = match e.speaker {
                    Speaker::Me => "me",
                    Speaker::Them => "them",
                  };
                  view! {
                    <li
                      class="flex flex-wrap items-baseline gap-x-3 gap-y-1 bg-inset px-3 py-2 text-xs"
                      data-practice-entry=who
                    >
                      <span class="w-12 shrink-0 text-muted-foreground">
                        {move || match e.speaker {
                          Speaker::Me => t("radio.practice-speaker-me"),
                          Speaker::Them => t("radio.practice-speaker-them"),
                        }}
                      </span>
                      <span class="min-w-0 break-all font-mono">{e.text.clone()}</span>
                    </li>
                  }
                })
                .collect_view()
            }}
          </ol>
        </div>

        // 成绩只在结束后出现；选项与输入框**不卸载**，只是禁用：回车提交最后一步时，
        // 正在处理这次按键的输入框若被卸掉，按键的默认动作会落到别的控件上（实测会让「回应」按钮被触发、整场被重置）。
        {move || {
          finished
            .get()
            .then(|| {
              let score = session.with(Session::score);
              view! {
                <p class="text-sm" role="status" data-testid="practice-score">
                  {tf(
                    "radio.practice-done",
                    &[
                      &score.my_turns.to_string(),
                      &score.first_try.to_string(),
                      &score.mistakes.to_string(),
                    ],
                  )}
                </p>
              }
            })
        }}
        <div class="space-y-3" data-testid="practice-turn">
          <p class="text-sm">
            {move || {
              session
                .with(|s| {
                  s.prompt()
                    .map(|p| {
                      let turn = tf("radio.practice-turn", &[&p.my_turn.to_string(), "3"]);
                      let heard = match p.last_heard {
                        Some(h) => tf("radio.practice-heard", &[&h]),
                        None => t("radio.practice-start-calling"),
                      };
                      format!("{turn} — {heard}")
                    })
                })
            }}
          </p>
          <div class="space-y-1.5">
            <h3 class="text-xs text-muted-foreground">{move || t("radio.practice-choices")}</h3>
            <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
              {move || {
                session
                  .with(|s| s.choices())
                  .into_iter()
                  .map(|c| {
                    let text = c.text.clone();
                    view! {
                      <Button
                        variant=Variant::Outline
                        size=Size::Sm
                        disabled=finished
                        class="h-auto min-w-0 flex-col items-start gap-0.5 py-1.5 text-left"
                        on_click=Callback::new(move |_| send(text.clone()))
                      >
                        <span class="text-xs text-muted-foreground">{move || kind_name(c.kind)}</span>
                        <span class="min-w-0 max-w-full break-all font-mono text-xs">{c.text.clone()}</span>
                      </Button>
                    }
                  })
                  .collect_view()
              }}
            </div>
          </div>
          <Field label=Signal::derive(move || t("radio.practice-own")) r#for=input_for>
            <div class="flex flex-wrap items-center gap-2">
              <Input
                id=input_id
                class="min-w-0 flex-1 uppercase"
                disabled=finished
                placeholder=Signal::derive(move || t("radio.practice-placeholder"))
                value=input
                on_change=Callback::new(move |v: String| input.set(v))
                on_enter=Callback::new(move |()| send(input.get_untracked()))
              />
              <Button
                variant=Variant::Default
                size=Size::Sm
                disabled=finished
                on_click=Callback::new(move |_| send(input.get_untracked()))
              >
                {move || t("tools.kiss-send")}
              </Button>
            </div>
          </Field>
        </div>

        <p class="text-sm" role="status" data-testid="practice-feedback">
          {move || {
            feedback
              .get()
              .map(|f| match f {
                Feedback::Right(text) => {
                  view! { <span class="text-win-text">{tf("radio.practice-correct", &[&text])}</span> }
                    .into_any()
                }
                Feedback::Wrong(msg) => view! { <span class="text-hp-text">{msg}</span> }.into_any(),
              })
          }}
        </p>
      </div>
    </section>
  }
}

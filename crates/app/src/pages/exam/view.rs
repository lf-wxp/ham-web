use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ham_web_core::saved_state::{ExamSavedState, keys};
use ham_web_core::text::format_ms;
use ham_web_core::weak_exam::{self, CategoryDelta};
use ham_web_core::{ExamRule, ExamScore, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;
use send_wrapper::SendWrapper;

use crate::components::common::{
  BottomBar, ExplanationCard, MessageDialog, QuestionProgressHeader,
};
use crate::components::exam::{
  AnswerCardFilter, AnswerCardSheet, ExamResultDialog, ExamResumeDialog, ExamSettingsDialog,
  ExamSubmitConfirmDialog,
};
use crate::components::question_card::QuestionCard;
use crate::data;
use crate::icons::{Icon, IconKind};
use crate::pages::{DEFAULT_TITLE, use_bank_query, use_no_site_footer};
use crate::shortcuts::{DigitDetail, Shortcuts, digit_answer, use_question_shortcuts};
use crate::store;
use crate::ui::{Size, Variant, button_class};
use crate::util::now_ms;
use crate::util::set_title;
use crate::util::storage;

use super::paper::pick_paper;
use super::store::ExamStore;

const PRESS: &str = "active:scale-[0.98] transition-transform";
const PRESS_FULL: &str = "w-full active:scale-[0.98] transition-transform";

#[component]
pub fn ExamPage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  use_no_site_footer();
  let (version, bank) = use_bank_query();
  let query = use_query_map();
  // 薄弱项组卷不保存断点，也不覆盖常规模考的断点
  let weak = Memo::new(move |_| query.with(|q| q.get("mode")).as_deref() == Some("weak"));
  let deltas = RwSignal::new(Vec::<CategoryDelta>::new());
  let store = ExamStore::new();
  let bank_all = RwSignal::new(Arc::<Vec<QuestionItem>>::new(Vec::new()));

  let loading = RwSignal::new(true);
  let settings_open = RwSignal::new(false);
  let result_open = RwSignal::new(false);
  let confirm_open = RwSignal::new(false);
  let card_open = RwSignal::new(false);
  let filter = RwSignal::new(AnswerCardFilter::All);
  let show_explanation = RwSignal::new(true);
  let error_open = RwSignal::new(false);
  let error_text = RwSignal::new(String::new());
  let resume_open = RwSignal::new(false);
  let pending: RwSignal<Option<ExamSavedState>> = RwSignal::new(None);
  let remaining = RwSignal::new(0i64);
  let generation = StoredValue::new(0u32);
  let help_shown = StoredValue::new(false);

  let rule = Memo::new(move |_| ExamRule::of(bank.get()));

  // 每个题库的答题卡筛选与解析显示偏好
  Effect::new(move |_| {
    let b = bank.get();
    filter.set(
      storage::get(&keys::answer_card_filter(b))
        .and_then(|s| AnswerCardFilter::parse(&s))
        .unwrap_or_default(),
    );
    show_explanation.set(storage::get(&keys::exam_show_explanation(b)).as_deref() != Some("0"));
  });

  // 加载题库、抽题或准备恢复
  Effect::new(move |_| {
    let (v, b, weak) = (version.get(), bank.get(), weak.get());
    store.reset();
    deltas.set(Vec::new());
    loading.set(true);
    pending.set(None);
    resume_open.set(false);
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    spawn_local(async move {
      let result = data::load_bank(v.as_deref(), b, true).await;
      if generation.try_get_value() != Some(current) {
        return;
      }
      let rule = ExamRule::of(b);
      match result {
        Ok(all) => {
          bank_all.set(all.clone());
          let saved = (!weak)
            .then(|| store::load_exam(b, v.as_deref()))
            .flatten()
            .filter(|s| s.should_resume(now_ms()));
          if let Some(saved) = saved {
            store.questions.set(Arc::new(saved.reconstruct(&all)));
            store.end_at.set(Some(saved.end_at_ms));
            pending.set(Some(saved));
            resume_open.set(true);
          } else {
            store.start(Arc::new(pick_paper(&all, b, weak)), rule);
          }
        }
        Err(_) => {
          error_text.set(format!("题库 {b} 暂不可用"));
          error_open.set(true);
        }
      }
      loading.set(false);
    });
  });

  let submit = move || {
    store.finished.set(true);
    let (b, is_weak) = (bank.get_untracked(), weak.get_untracked());
    if !is_weak {
      store::clear_exam(b, version.get_untracked().as_deref());
    }
    // 保存本次成绩到历史
    let answers = store.answers.get_untracked();
    let sc = store
      .questions
      .with(|qs| ExamScore::calculate(qs, |q, i| answers.get(&q.answer_key(i)).map(Vec::as_slice)));
    crate::exam_history::save(b, sc, is_weak);
    let before = crate::study::load_stats();
    deltas.set(store.questions.with_untracked(|qs| {
      weak_exam::compare(
        qs,
        |i| {
          answers
            .get(&qs[i].answer_key(i))
            .is_some_and(|a| qs[i].is_answer_correct(a))
        },
        before.bank(b),
      )
    }));
    result_open.set(true);
    store.questions.with_untracked(|qs| {
      crate::study::record_many(
        qs.iter()
          .enumerate()
          .filter_map(|(i, q)| answers.get(&q.answer_key(i)).map(|a| (q, a.as_slice()))),
      );
    });
  };

  let (swipe_start, swipe_end) = crate::gesture::swipe_handlers(Callback::new(move |s| match s {
    crate::gesture::Swipe::Left => store.next(),
    crate::gesture::Swipe::Right => store.prev(),
  }));

  // 倒计时
  let tick = move || {
    let Some(Some(end)) = store.end_at.try_get_untracked() else {
      return;
    };
    let left = (end - now_ms()).max(0);
    remaining.set(left);
    if left == 0 && !store.finished.get_untracked() && pending.with_untracked(Option::is_none) {
      submit();
    }
  };
  Effect::new(move |_| {
    store.end_at.track();
    tick();
  });
  if let Ok(handle) = set_interval_with_handle(tick, Duration::from_secs(1)) {
    on_cleanup(move || handle.clear());
  }

  // 持久化进度
  Effect::new(move |_| {
    let qs = store.questions.get();
    let answers = store.answers.get();
    let flags = store.flags.get();
    let index = store.index.get();
    let (Some(end_at), false) = (store.end_at.get(), store.finished.get()) else {
      return;
    };
    if qs.is_empty() || pending.with(Option::is_some) || weak.get_untracked() {
      return;
    }
    let mut answers_by_position = Vec::with_capacity(qs.len());
    let mut flags_by_position = Vec::with_capacity(qs.len());
    for (i, q) in qs.iter().enumerate() {
      let key = q.answer_key(i);
      answers_by_position.push(answers.get(&key).filter(|a| !a.is_empty()).cloned());
      flags_by_position.push(flags.get(&key).copied().unwrap_or(false));
    }
    store::save_exam(&ExamSavedState {
      version: 2,
      bank: bank.get_untracked(),
      version_id: version.get_untracked(),
      timestamp: now_ms(),
      end_at_ms: end_at,
      index,
      answers_by_position,
      flags_by_position,
      total: qs.len(),
      question_ids: Some(qs.iter().map(QuestionItem::stable_id).collect()),
      questions_snapshot: None,
    });
  });

  // 考试进行中离开页面时提示
  let unload = window_event_listener_untyped("beforeunload", move |e| {
    let active = store.finished.try_get_untracked() == Some(false)
      && store.end_at.try_get_untracked().flatten().is_some();
    if active {
      e.prevent_default();
      let _ = js_sys::Reflect::set(
        &e,
        &"returnValue".into(),
        &"考试仍在进行，离开页面计时不会暂停".into(),
      );
    }
  });
  let unload = SendWrapper::new(Some(unload));
  on_cleanup(move || {
    if let Some(h) = unload.take() {
      h.remove();
    }
  });

  // 首次进入自动展示快捷键说明
  Effect::new(move |_| {
    if help_shown.get_value() || store.questions.with(|q| q.is_empty()) {
      return;
    }
    help_shown.set_value(true);
    if store::help_seen(keys::HELP_SEEN_EXAM) {
      return;
    }
    set_timeout(
      move || {
        settings_open.try_set(true);
        store::mark_help_seen(keys::HELP_SEEN_EXAM);
      },
      Duration::from_millis(300),
    );
  });

  let selected = Signal::derive(move || {
    store
      .current_key()
      .and_then(|k| store.answers.with(|a| a.get(&k).cloned()))
      .unwrap_or_default()
  });
  let set_answer = Callback::new(move |ans: Vec<String>| {
    if let Some(k) = store.current_key() {
      store.answers.update(|a| {
        a.insert(k, ans);
      });
    }
  });
  let answered = Signal::derive(move || {
    store
      .answers
      .with(|a| a.values().filter(|v| !v.is_empty()).count())
  });
  let flagged = Signal::derive(move || store.flags.with(|f| f.values().filter(|v| **v).count()));
  let is_flagged = move || {
    store
      .current_key()
      .is_some_and(|k| store.flags.with(|f| f.get(&k).copied().unwrap_or(false)))
  };
  let toggle_flag = move |_| {
    if let Some(k) = store.current_key() {
      store.flags.update(|f| {
        let v = f.entry(k).or_insert(false);
        *v = !*v;
      });
    }
  };
  let score = Signal::derive(move || {
    let answers = store.answers.get();
    store
      .questions
      .with(|qs| ExamScore::calculate(qs, |q, i| answers.get(&q.answer_key(i)).map(Vec::as_slice)))
  });
  let total = Signal::derive(move || store.len());
  let percent = Signal::derive(move || {
    let n = store.len();
    if n == 0 {
      0
    } else {
      ((store.index.get() + 1) as f64 / n as f64 * 100.0).round() as i64
    }
  });

  use_question_shortcuts(Shortcuts {
    on_prev: Callback::new(move |()| store.prev()),
    on_next: Callback::new(move |()| store.next()),
    on_digit: Callback::new(move |(n, d): (usize, DigitDetail)| {
      if store.finished.get_untracked() {
        return;
      }
      let Some((_, q)) = store.current() else {
        return;
      };
      let keys: Vec<String> = q.options.iter().map(|o| o.key.clone()).collect();
      if let Some(next) = digit_answer(
        &keys,
        q.is_multiple(),
        &selected.get_untracked(),
        n,
        d.strict,
      ) {
        set_answer.run(next);
      }
    }),
    enter_search: None,
  });

  let on_resume = Callback::new(move |()| {
    let Some(saved) = pending.get_untracked() else {
      return;
    };
    let qs = store.questions.get_untracked();
    let mut answers = HashMap::new();
    let mut flags = HashMap::new();
    for (pos, q) in qs.iter().enumerate() {
      let key = q.answer_key(pos);
      if let Some(Some(a)) = saved.answers_by_position.get(pos)
        && !a.is_empty()
      {
        answers.insert(key.clone(), a.clone());
      }
      if saved.flags_by_position.get(pos).copied().unwrap_or(false) {
        flags.insert(key, true);
      }
    }
    store.answers.set(answers);
    store.flags.set(flags);
    store.index.set(saved.index.min(qs.len().saturating_sub(1)));
    store.end_at.set(Some(saved.end_at_ms));
    resume_open.set(false);
    pending.set(None);
  });
  let on_restart = Callback::new(move |()| {
    store::clear_exam(bank.get_untracked(), version.get_untracked().as_deref());
    let all = bank_all.get_untracked();
    let picked = pick_paper(&all, bank.get_untracked(), weak.get_untracked());
    store.start(Arc::new(picked), rule.get_untracked());
    resume_open.set(false);
    pending.set(None);
  });

  let remaining_view = move || {
    let ms = remaining.get();
    let class = if ms <= 60_000 {
      "text-red-600 dark:text-red-400"
    } else {
      ""
    };
    view! {
      <span class=class aria-live="polite">
        {format_ms(ms)}
      </span>
    }
  };
  let flag_label = move || {
    if is_flagged() {
      "取消标记"
    } else {
      "标记"
    }
  };
  let at_start = move || store.index.get() == 0;
  let at_end = move || store.index.get() + 1 >= store.len();

  let content = move || {
    if loading.get() {
      return view! { <div class="p-6" aria-live="polite">"加载题库中..."</div> }.into_any();
    }
    if store.questions.with(|q| q.is_empty()) {
      return view! { <div class="p-6" role="alert">"题库暂不可用或为空"</div> }.into_any();
    }
    let b = bank.get();
    let r = rule.get();
    view! {
      <div on:touchstart=swipe_start on:touchend=swipe_end class="container mx-auto px-4 py-6 max-w-5xl space-y-4 pb-28 sm:pb-20 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
        <QuestionProgressHeader
          percent=percent
          right=move || {
            let href = move || {
              let base = crate::pages::bank_href("/exam", version.get().as_deref(), bank.get());
              if weak.get() { base } else { format!("{base}&mode=weak") }
            };
            view! {
              <a
                class=button_class(Variant::Outline, Size::Sm, "")
                href=href
                title="按分类正确率与错题加权抽题，不计入备考状态"
              >
                {move || if weak.get() { "常规模考" } else { "薄弱项组卷" }}
              </a>
              <button
                class=button_class(Variant::Outline, Size::Icon, "")
                aria-label="设置"
                title="设置"
                on:click=move |_| settings_open.set(true)
              >
                <Icon kind=IconKind::Settings class="h-4 w-4" />
              </button>
            }
          }
          meta=ViewFn::from(move || {
            view! {
              {move || weak.get().then(|| view! { <span class="mr-1 rounded bg-amber-500/15 px-1.5 py-0.5 text-amber-800 dark:text-amber-300">"薄弱项组卷"</span> })}
              "考试类别：" {b.as_str()} " 类｜试题数：" {r.total} "（单选 " {r.singles} "，多选 " {r.multiples}
              "）｜限时：" {r.minutes} " 分钟｜剩余时间：" {remaining_view}
            }
          })
        />
        <div class="sm:hidden grid grid-cols-2 gap-x-3 gap-y-1 text-xs text-muted-foreground">
          <div>"考试类别：" {b.as_str()} " 类"</div>
          <div>"试题数：" {r.total} "（单选 " {r.singles} "，多选 " {r.multiples} "）"</div>
          <div>"限时：" {r.minutes} " 分钟"</div>
          <div>"剩余：" {remaining_view}</div>
        </div>

        {move || {
          store
            .current()
            .map(|(i, q)| {
              let finished = store.finished;
              let expl = q.clone();
              view! {
                <QuestionCard
                  index=i
                  total=store.len()
                  question=q
                  selected=selected
                  on_change=set_answer
                  show_answer=finished
                  read_only=finished
                />
                {move || {
                  (finished.get() && show_explanation.get()).then(|| view! { <ExplanationCard question=expl.clone() /> })
                }}
              }
            })
        }}

        <BottomBar
          stats=move || view! { "已作答 " {answered} " / " {total} "｜标记 " {flagged} }
          left=move || {
            view! {
              <button
                class=button_class(Variant::Secondary, Size::Default, PRESS)
                disabled=at_start
                on:click=move |_| store.prev()
              >
                "上一题"
              </button>
            }
          }
          right=move || {
            view! {
              <button class=button_class(Variant::Outline, Size::Default, PRESS) on:click=toggle_flag>
                {flag_label}
              </button>
              <button class=button_class(Variant::Outline, Size::Default, PRESS) on:click=move |_| card_open.set(true)>
                "答题卡"
              </button>
              <button class=button_class(Variant::Default, Size::Default, PRESS) disabled=at_end on:click=move |_| store.next()>
                "下一题"
              </button>
              <button
                class=button_class(Variant::Destructive, Size::Default, PRESS)
                disabled=move || store.finished.get()
                on:click=move |_| confirm_open.set(true)
              >
                {move || if store.finished.get() { "已交卷" } else { "交卷" }}
              </button>
            }
          }
          mobile_top=move || {
            view! {
              <div class="grid grid-cols-2 gap-2">
                <button
                  class=button_class(Variant::Secondary, Size::Default, PRESS_FULL)
                  disabled=at_start
                  on:click=move |_| store.prev()
                >
                  "上一题"
                </button>
                <button
                  class=button_class(Variant::Default, Size::Default, PRESS_FULL)
                  disabled=at_end
                  on:click=move |_| store.next()
                >
                  "下一题"
                </button>
              </div>
            }
          }
          mobile_bottom=ViewFn::from(move || {
            view! {
              <div class="grid grid-cols-3 gap-2 mt-2">
                <button class=button_class(Variant::Outline, Size::Default, PRESS_FULL) on:click=toggle_flag>
                  {flag_label}
                </button>
                <button
                  class=button_class(Variant::Outline, Size::Default, PRESS_FULL)
                  on:click=move |_| card_open.set(true)
                >
                  "答题卡"
                </button>
                <button
                  class=button_class(Variant::Destructive, Size::Default, PRESS_FULL)
                  disabled=move || store.finished.get()
                  on:click=move |_| confirm_open.set(true)
                >
                  {move || if store.finished.get() { "已交卷" } else { "交卷" }}
                </button>
              </div>
            }
          })
        />
      </div>
    }
    .into_any()
  };

  view! {
    <h1 class="sr-only">{move || if weak.get() { "薄弱项组卷" } else { "模拟考试" }}</h1>
    {content}
    <ExamResultDialog
      open=result_open
      score=score
      pass_line=Signal::derive(move || rule.get().pass)
      deltas=deltas
      weak=weak
    />
    <AnswerCardSheet
      open=card_open
      questions=store.questions
      answers=store.answers
      flags=store.flags
      finished=store.finished
      filter=filter
      on_change_filter=Callback::new(move |f: AnswerCardFilter| {
        filter.set(f);
        storage::set(&keys::answer_card_filter(bank.get_untracked()), f.as_str());
      })
      on_jump=Callback::new(move |i| store.jump(i))
      current_index=store.index
    />
    <ExamSubmitConfirmDialog
      open=confirm_open
      on_confirm=Callback::new(move |()| {
        confirm_open.set(false);
        submit();
      })
      total=total
      answered=answered
      flagged=flagged
    />
    <ExamSettingsDialog
      open=settings_open
      show_explanation=show_explanation
      on_change_show_explanation=Callback::new(move |v: bool| {
        show_explanation.set(v);
        storage::set(&keys::exam_show_explanation(bank.get_untracked()), if v { "1" } else { "0" });
      })
    />
    <ExamResumeDialog
      open=resume_open
      expires_in_ms=Signal::derive(move || {
        remaining.track();
        pending.with(|p| p.as_ref().map_or(0, |s| (s.end_at_ms - now_ms()).max(0)))
      })
      answered=Signal::derive(move || pending.with(|p| p.as_ref().map_or(0, ExamSavedState::answered)))
      total=Signal::derive(move || pending.with(|p| p.as_ref().map(|s| s.total)).unwrap_or_else(|| store.len()))
      on_resume=on_resume
      on_restart=on_restart
    />
    <MessageDialog open=error_open title="加载失败" description=error_text confirm_text="知道了" />
  }
}

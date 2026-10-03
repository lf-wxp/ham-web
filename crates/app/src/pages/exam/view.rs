use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ham_web_core::exam::CustomPaper;
use ham_web_core::saved_state::{ExamSavedState, keys};
use ham_web_core::weak_exam::{self, CategoryDelta};
use ham_web_core::{ExamRule, ExamScore, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;
use send_wrapper::SendWrapper;

use crate::components::common::{ExplanationCard, MessageDialog};
use crate::components::exam::{
  AnswerCardFilter, AnswerCardSheet, CustomPaperDialog, ExamResultDialog, ExamResumeDialog,
  ExamSettingsDialog, ExamSubmitConfirmDialog,
};
use crate::components::question_card::QuestionCard;
use crate::components::shortcut_help::ShortcutHelpDialog;
use crate::data;
use crate::pages::{DEFAULT_TITLE, use_bank_query, use_no_site_footer};
use crate::shortcuts::{DigitDetail, Shortcuts, digit_answer, use_question_shortcuts};
use crate::store;
use crate::ui::{Size, Variant, button_class};
use crate::util::now_ms;
use crate::util::set_title;
use crate::util::storage;
use ham_web_core::exam_review::{ExamReview, ReviewItem};

use super::exam_bottom_bar::ExamBottomBar;
use super::exam_header::ExamHeader;
use super::paper::pick_paper;
use super::store::ExamStore;
use crate::i18n::{t, tf};

#[component]
pub fn ExamPage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  use_no_site_footer();
  let (version, bank) = use_bank_query();
  let query = use_query_map();
  // 薄弱项组卷不保存断点，也不覆盖常规模考的断点
  let weak = Memo::new(move |_| query.with(|q| q.get("mode")).as_deref() == Some("weak"));
  // 自定义组卷：自选题量与分类范围，同样不保存断点、不计入备考状态。
  let custom = Memo::new(move |_| query.with(|q| q.get("mode")).as_deref() == Some("custom"));
  // 是否按分类占比抽题（贴近真实大纲分布）。
  let weighted = RwSignal::new(storage::get("exam:weighted").as_deref() == Some("1"));
  let strict = RwSignal::new(storage::get("exam:strict").as_deref() == Some("1"));
  let deltas = RwSignal::new(Vec::<CategoryDelta>::new());
  let wrong_count = RwSignal::new(0usize);
  let store = ExamStore::new();

  // 机考仿真：考试进行中进入全屏，结束后退出全屏。
  Effect::new(move |_| {
    let active = strict.get() && store.end_at.get().is_some();
    let doc = crate::util::document();
    if active && doc.fullscreen_element().is_none() {
      if let Some(el) = doc.document_element() {
        let _ = el.request_fullscreen();
      }
    } else if !active && doc.fullscreen_element().is_some() {
      doc.exit_fullscreen();
    }
  });
  let bank_all = RwSignal::new(Arc::<Vec<QuestionItem>>::new(Vec::new()));

  let loading = RwSignal::new(true);
  let settings_open = RwSignal::new(false);
  let help_open = RwSignal::new(false);
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

  let custom_paper = RwSignal::new(None::<CustomPaper>);
  let custom_open = RwSignal::new(false);
  // 自定义组卷时按用户配置生成规则，否则用 A/B/C 标准规则。
  let rule = Memo::new(move |_| {
    custom_paper
      .get()
      .map_or_else(|| ExamRule::of(bank.get()), |p| p.rule())
  });

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
    let (v, b, weak, weighted) = (version.get(), bank.get(), weak.get(), weighted.get());
    let is_custom = custom.get();
    let paper = custom_paper.get();
    // 自定义组卷尚未配置：打开对话框等待用户确认（触发 custom_paper 变化）后再组卷。
    if is_custom && paper.is_none() {
      custom_open.set(true);
      return;
    }
    store.reset();
    // 开始一场新考试时清除上一场暂存的错题列表，避免「重练错题」读到旧数据。
    store::clear_wrong_exam(b);
    deltas.set(Vec::new());
    loading.set(true);
    pending.set(None);
    resume_open.set(false);
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    let current_rule = rule.get();
    spawn_local(async move {
      let result = data::load_bank(v.as_deref(), b, true).await;
      if generation.try_get_value() != Some(current) {
        return;
      }
      match result {
        Ok(all) => {
          bank_all.set(all.clone());
          let saved = (!weak && !is_custom)
            .then(|| store::load_exam(b, v.as_deref()))
            .flatten()
            .filter(|s| s.should_resume(now_ms()));
          if let Some(saved) = saved {
            store.questions.set(Arc::new(saved.reconstruct(&all)));
            store.end_at.set(Some(saved.end_at_ms));
            pending.set(Some(saved));
            resume_open.set(true);
          } else {
            store.start(
              Arc::new(pick_paper(&all, b, weak, weighted, paper.as_ref())),
              current_rule,
            );
          }
        }
        Err(_) => {
          error_text.set(tf("题库 {} 暂不可用", &[&b.to_string()]));
          error_open.set(true);
        }
      }
      loading.set(false);
    });
  });

  let submit = move || {
    store.finished.set(true);
    // 自定义组卷与薄弱项组卷一样：不保存断点、不计入备考状态。
    let (b, is_weak) = (
      bank.get_untracked(),
      weak.get_untracked() || custom.get_untracked(),
    );
    if !is_weak {
      store::clear_exam(b, version.get_untracked().as_deref());
    }
    // 保存本次成绩到历史
    let answers = store.answers.get_untracked();
    let sc = store
      .questions
      .with(|qs| ExamScore::calculate(qs, |q, i| answers.get(&q.answer_key(i)).map(Vec::as_slice)));
    crate::exam_history::save(b, sc, is_weak);
    // 存一份逐题快照供「考后复盘」页使用：交卷弹窗关掉后，原本就再也回不去
    // 逐题对错与解析了。只保留最近一次，避免占满本地存储。
    let review = ExamReview {
      bank: b.as_str().to_owned(),
      finished_at_ms: now_ms(),
      items: store.questions.with_untracked(|qs| {
        qs.iter()
          .enumerate()
          .map(|(i, q)| ReviewItem {
            id: q.stable_id().unwrap_or_else(|| q.answer_key(i)),
            code: q.j_code().unwrap_or_default().to_owned(),
            question: q.question.clone(),
            options: q
              .options
              .iter()
              .map(|o| format!("{}. {}", o.key, o.text))
              .collect(),
            answer: q.answer_keys.clone(),
            given: answers.get(&q.answer_key(i)).cloned().unwrap_or_default(),
            explanation: q.explanation.clone().unwrap_or_default(),
            category: q.p_code().unwrap_or_default().to_owned(),
          })
          .collect()
      }),
    };
    // 直接写，不再推到宏任务：构建 `ReviewItem` 列表（克隆题干/选项/解析）本就是同步
    // 完成的，被推迟的只剩一次 JSON 序列化，收益很小；而推迟会留下「交卷后立刻关掉
    // 标签页就丢快照」的窗口。快照只保留最近一次，丢了就再也回不来。
    store::save_exam_review(&review);
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
      let answered: Vec<(&QuestionItem, &[String])> = qs
        .iter()
        .enumerate()
        .filter_map(|(i, q)| answers.get(&q.answer_key(i)).map(|a| (q, a.as_slice())))
        .collect();
      let elapsed = (now_ms() - store.started_ms.get_untracked()).max(0) as u64;
      let per = if answered.is_empty() {
        0
      } else {
        elapsed / answered.len() as u64
      };
      crate::study::record_many(answered.into_iter().map(|(q, a)| (q, a, per)));
      // 收集本次错题（含未作答的题），暂存供练习页一键重练。
      let wrong: Vec<QuestionItem> = qs
        .iter()
        .enumerate()
        .filter(|(i, q)| {
          !answers
            .get(&q.answer_key(*i))
            .is_some_and(|a| q.is_answer_correct(a))
        })
        .map(|(_, q)| q.clone())
        .collect();
      wrong_count.set(wrong.len());
      store::save_wrong_exam(b, &wrong);
    });
  };

  let (swipe_start, swipe_end) = crate::gesture::swipe_handlers(Callback::new(move |s| match s {
    crate::gesture::Swipe::Left => store.next(),
    crate::gesture::Swipe::Right => {
      if !strict.get_untracked() {
        store.prev();
      }
    }
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
    if qs.is_empty()
      || pending.with(Option::is_some)
      || weak.get_untracked()
      || custom.get_untracked()
    {
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
        &t("考试仍在进行，离开页面计时不会暂停").into(),
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
  let is_flagged = Signal::derive(move || {
    store
      .current_key()
      .is_some_and(|k| store.flags.with(|f| f.get(&k).copied().unwrap_or(false)))
  });
  let on_toggle_flag = Callback::new(move |()| {
    if let Some(k) = store.current_key() {
      store.flags.update(|f| {
        let v = f.entry(k).or_insert(false);
        *v = !*v;
      });
    }
  });
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
    on_prev: Callback::new(move |()| {
      if !strict.get_untracked() {
        store.prev();
      }
    }),
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
    on_help: Some(Callback::new(move |()| help_open.set(true))),
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
    let picked = pick_paper(
      &all,
      bank.get_untracked(),
      weak.get_untracked(),
      weighted.get_untracked(),
      custom_paper.get_untracked().as_ref(),
    );
    store.start(Arc::new(picked), rule.get_untracked());
    resume_open.set(false);
    pending.set(None);
  });

  let at_start = Signal::derive(move || store.index.get() == 0);
  let at_end = Signal::derive(move || store.index.get() + 1 >= store.len());

  let content = move || {
    if loading.get() {
      return view! { <div class="p-6" aria-live="polite">{move || t("加载题库中...")}</div> }
        .into_any();
    }
    if store.questions.with(|q| q.is_empty()) {
      return view! { <div class="p-6" role="alert">{move || t("题库暂不可用或为空")}</div> }
        .into_any();
    }
    view! {
      <div on:touchstart=swipe_start on:touchend=swipe_end class="mx-auto max-w-5xl px-4 py-6 space-y-4 pb-28 sm:pb-20 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
        <ExamHeader
          percent=percent
          weak=weak
          version=version
          bank=bank
          rule=rule
          remaining=remaining
          on_open_settings=Callback::new(move |()| settings_open.set(true))
        />

        {move || {
          store
            .current()
            .map(|(i, q)| {
              let finished = store.finished;
              let expl = q.clone();
              let check_q = q.clone();
              let wrong_key = q.answer_key(i);
              let p_code = q.p_code().map(str::to_owned);
              let bank_val = bank.get_untracked();
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
                {move || {
                  if !finished.get() {
                    return ().into_any();
                  }
                  let wrong = store.answers.with(|a| {
                    a.get(&wrong_key)
                      .is_some_and(|ans| !ans.is_empty() && !check_q.is_answer_correct(ans))
                  });
                  let Some(p) = p_code.as_ref() else {
                    return ().into_any();
                  };
                  if !wrong {
                    return ().into_any();
                  }
                  let href = format!("/practice?bank={}&sub={}", bank_val.as_str(), p);
                  view! {
                    <div class="flex flex-wrap items-center gap-2">
                      <span class="text-xs text-muted-foreground">{t("这道题答错了")}</span>
                      <a href=href class=button_class(Variant::Outline, Size::Sm, "")>
                        {t("同类题再练")}
                      </a>
                    </div>
                  }
                  .into_any()
                }}
              }
            })
        }}

        <ExamBottomBar
          answered=answered
          total=total
          flagged=flagged
          is_flagged=is_flagged
          finished=store.finished
          at_start=at_start
          at_end=at_end
          on_prev=Callback::new(move |()| {
            if !strict.get_untracked() {
              store.prev();
            }
          })
          on_next=Callback::new(move |()| store.next())
          on_toggle_flag=on_toggle_flag
          on_open_card=Callback::new(move |()| card_open.set(true))
          on_submit=Callback::new(move |()| confirm_open.set(true))
        />
      </div>
    }
    .into_any()
  };

  view! {
    <h1 class="sr-only">
      {move || {
        if custom.get() {
          t("自定义组卷")
        } else if weak.get() {
          t("薄弱项组卷")
        } else {
          t("模拟考试")
        }
      }}
    </h1>
    {content}
    <ExamResultDialog
      open=result_open
      score=score
      pass_line=Signal::derive(move || rule.get().pass)
      deltas=deltas
      weak=weak
      wrong_count=wrong_count
      wrong_href=Signal::derive(move || format!("/practice?bank={}&src=exam", bank.get()))
      bank=bank
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
      on_jump=Callback::new(move |i| {
        if !strict.get_untracked() {
          store.jump(i);
        }
      })
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
      weighted=weighted
      on_change_weighted=Callback::new(move |v: bool| {
        weighted.set(v);
        storage::set("exam:weighted", if v { "1" } else { "0" });
      })
      strict=strict
      on_change_strict=Callback::new(move |v: bool| {
        strict.set(v);
        storage::set("exam:strict", if v { "1" } else { "0" });
      })
    />
    <CustomPaperDialog
      open=custom_open
      on_confirm=Callback::new(move |p: CustomPaper| custom_paper.set(Some(p)))
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
    <MessageDialog open=error_open title=t("加载失败") description=error_text confirm_text=t("知道了") />
    <ShortcutHelpDialog open=help_open />
  }
}

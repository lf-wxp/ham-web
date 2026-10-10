use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ham_web_core::Bank;
use ham_web_core::categories::top_of;
use ham_web_core::practice::{PracticeOrder, find_jump_target, search};
use ham_web_core::saved_state::{PracticeSavedState, keys};
use ham_web_core::text::js_trim;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::components::common::{ExplanationCard, MessageDialog, NoteEditor, PageContainer};
use crate::components::practice::{
  PracticeResumeDialog, PracticeSearchDialog, PracticeSettingsDialog,
};
use crate::components::question_card::QuestionCard;
use crate::components::shortcut_help::ShortcutHelpDialog;
use crate::data::Questions;
use crate::pages::{DEFAULT_TITLE, bank_href, use_bank_query, use_no_site_footer};
use crate::shortcuts::{DigitDetail, Shortcuts, digit_answer, use_question_shortcuts};
use crate::store;
use crate::ui::{
  Button, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Size, Variant,
};
use crate::util::alert;
use crate::util::now_ms;
use crate::util::set_title;

use super::bottom_bar::PracticeBottomBar;
use super::header_bar::PracticeHeaderBar;
use super::store::{PracticeStore, load_questions};
use crate::i18n::{t, tf};

#[component]
pub fn PracticePage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  use_no_site_footer();
  let (version, bank) = use_bank_query();
  let query = use_query_map();
  let topic = Memo::new(move |_| query.with(|q| q.get("topic")).filter(|t| !t.is_empty()));
  let sub_code = Memo::new(move |_| query.with(|q| q.get("sub")).filter(|t| !t.is_empty()));
  let src = Memo::new(move |_| query.with(|q| q.get("src")).filter(|t| !t.is_empty()));
  // 只练多选：不再占用独立导航入口，改由页面顶部开关切换；URL 带 `?multi=1` 时默认开启。
  let multi_only = RwSignal::new(
    query
      .with_untracked(|q| q.get("multi"))
      .is_some_and(|v| v == "1"),
  );
  let navigate = use_navigate();
  let store = PracticeStore::new();
  on_cleanup(move || store.commit_current());

  // 收藏状态（跟随当前题）
  let bookmarked = RwSignal::new(false);
  Effect::new(move |_| {
    let Some((i, q)) = store.current() else {
      return;
    };
    let id = q.stable_id().unwrap_or_else(|| q.answer_key(i));
    bookmarked.set(store::is_bookmarked(&id));
  });
  let on_toggle_bookmark = Callback::new(move |()| {
    let Some((i, q)) = store.current() else {
      return;
    };
    let id = q.stable_id().unwrap_or_else(|| q.answer_key(i));
    // 变体只在当次练习里存在，收藏没有意义（下次就换题了）；按钮已置灰，这里兜住快捷键等其它入口。
    if id.starts_with(ham_web_core::calc_variants::VARIANT_PREFIX) {
      return;
    }
    bookmarked.set(store::toggle_bookmark(&id));
  });
  let current_is_variant = Signal::derive(move || {
    store.current().is_some_and(|(_, q)| {
      q.id_str()
        .is_some_and(|id| id.starts_with(ham_web_core::calc_variants::VARIANT_PREFIX))
    })
  });

  // 当前题的 stable_id，供笔记编辑区跟随切题加载。
  let current_id = Signal::derive(move || {
    store
      .current()
      .and_then(|(_, q)| q.stable_id())
      .unwrap_or_default()
  });

  let jump_input = RwSignal::new(String::new());
  let resume_open = RwSignal::new(false);
  let pending: RwSignal<Option<PracticeSavedState>> = RwSignal::new(None);
  let search_open = RwSignal::new(false);
  let settings_open = RwSignal::new(false);
  let help_open = RwSignal::new(false);
  let no_prompt = RwSignal::new(false);
  let error_open = RwSignal::new(false);
  let error_text = RwSignal::new(String::new());
  let unique_only = RwSignal::new(false);
  let unseen_only = RwSignal::new(query.with_untracked(|q| q.get("unseen").is_some()));
  // 计算变体：同型题换数重新生成（防背答案），只在当次练习里出现。
  let variants_on = RwSignal::new(query.with_untracked(|q| q.get("variant").is_some()));
  // 专项 / 只练没做过 / 错题重练 / 计算变体：题目与完整题库不一致（子集，或末尾追加了本地生成的题），
  // 不保存也不恢复进度，以免覆盖完整题库的顺序进度（变体开启时总数是 N+8，次日变体还会换题）。
  let subset = Memo::new(move |_| {
    topic.get().is_some()
      || sub_code.get().is_some()
      || unseen_only.get()
      || src.get().is_some()
      || multi_only.get()
      || variants_on.get()
  });
  let generation = StoredValue::new(0u32);
  let help_shown = StoredValue::new(false);
  let pending_order = RwSignal::new(None::<PracticeOrder>);
  let confirm_order_open = RwSignal::new(false);

  Effect::new(move |_| {
    no_prompt.set(store::load_no_resume(bank.get(), version.get().as_deref()));
  });

  let try_prompt_resume = move || {
    let (b, v) = (bank.get_untracked(), version.get_untracked());
    if subset.get_untracked() || store::load_no_resume(b, v.as_deref()) {
      return;
    }
    let total = store.all.with_untracked(|a| a.len());
    if let Some(saved) = store::load_practice(b, v.as_deref())
      && saved.total == total
      && saved.order == PracticeOrder::Sequential.as_str()
      && saved.should_resume(now_ms())
    {
      pending.set(Some(saved));
      resume_open.set(true);
    }
  };

  // 加载题库（题库类别 / 版本 / 「只看本类新增」变化时重新加载）
  Effect::new(move |_| {
    let (v, b, unique) = (version.get(), bank.get(), unique_only.get());
    let topic_val = topic.get();
    let sub_val = sub_code.get();
    let src_val = src.get();
    let unseen = unseen_only.get();
    let multi_val = multi_only.get();
    // 读一次：变体开关翻转时整个加载重跑（重新组卷）。
    let _variants = variants_on.get();
    store.reset();
    pending.set(None);
    resume_open.set(false);
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    spawn_local(async move {
      // 模拟考错题重练：直接读交卷时暂存的错题列表，无需加载题库。
      if src_val.as_deref() == Some("exam") {
        let wrong = store::load_wrong_exam(b).unwrap_or_default();
        if generation.try_get_value() != Some(current) {
          return;
        }
        let filtered: Questions = if unseen {
          let seen = crate::study::load_stats()
            .bank(b)
            .cloned()
            .unwrap_or_default();
          Arc::new(wrong.into_iter().filter(|q| !seen.has_seen(q)).collect())
        } else {
          Arc::new(wrong)
        };
        store.load(filtered);
        store.set_order(PracticeOrder::Random);
        return;
      }
      let result = load_questions(v.as_deref(), b, unique).await;
      if generation.try_get_value() != Some(current) {
        return;
      }
      match result {
        Ok(qs) => {
          // 专项练习：按一级分类过滤，随机顺序；只练没做过：去掉已做过的题，保持顺序。均不提示恢复。
          let seen = unseen.then(|| {
            crate::study::load_stats()
              .bank(b)
              .cloned()
              .unwrap_or_default()
          });
          let filtered: Questions =
            if topic_val.is_none() && sub_val.is_none() && seen.is_none() && !multi_val {
              qs
            } else {
              Arc::new(
                qs.iter()
                  .filter(|q| {
                    topic_val
                      .as_deref()
                      .is_none_or(|t| q.p_code().and_then(top_of).is_some_and(|top| top.key == t))
                      && sub_val
                        .as_deref()
                        .is_none_or(|s| q.p_code().is_some_and(|p| p == s))
                      && seen.as_ref().is_none_or(|s| !s.has_seen(q))
                      && (!multi_val || q.is_multiple())
                  })
                  .cloned()
                  .collect(),
              )
            };
          // 计算变体：追加到当次练习末尾（种子取当日序号 —— 同一天同一套，
          // 断点续做能原样重建；次日自动换一套）。
          let filtered = if variants_on.get_untracked() {
            let seed = ham_web_core::calc_variants::day_seed(
              crate::util::now_ms(),
              crate::util::utc_offset_minutes(),
            );
            let extra = ham_web_core::calc_variants::variant_questions(seed);
            Arc::new(filtered.iter().chain(extra.iter()).cloned().collect())
          } else {
            filtered
          };
          store.load(filtered);
          if topic_val.is_some() || sub_val.is_some() || multi_val {
            store.set_order(PracticeOrder::Random);
          } else if !unseen {
            let last = store::load_last_mode();
            if let Some(mode) = last
              && mode != PracticeOrder::Sequential
            {
              store.set_order(mode);
            }
            if last != Some(PracticeOrder::Random) {
              try_prompt_resume();
            }
          }
        }
        Err(_) => {
          error_text.set(tf("exam.bank-unavailable", &[&b.to_string()]));
          error_open.set(true);
        }
      }
    });
  });

  // 保存进度（仅顺序模式）
  Effect::new(move |_| {
    let (loading, order) = (store.loading.get(), store.order.get());
    let order_idx = store.order_idx.get();
    let answers = store.answers.get();
    let index = store.index.get();
    let (show_answer, show_explanation) = (store.show_answer.get(), store.show_explanation.get());
    if loading || order != PracticeOrder::Sequential || subset.get() {
      return;
    }
    let all = store.all.get_untracked();
    let answers_by_position: Vec<Option<Vec<String>>> = order_idx
      .iter()
      .enumerate()
      .map(|(pos, &i)| {
        answers
          .get(&all[i].answer_key(pos))
          .filter(|a| !a.is_empty())
          .cloned()
      })
      .collect();
    if index == 0 && answers_by_position.iter().all(Option::is_none) {
      return;
    }
    store::save_practice(&PracticeSavedState {
      version: 2,
      bank: bank.get_untracked(),
      version_id: version.get_untracked(),
      timestamp: now_ms(),
      index,
      order: order.as_str().to_owned(),
      show_answer,
      show_explanation: Some(show_explanation),
      order_indices: order_idx,
      answers_by_position,
      total: all.len(),
    });
  });

  let (swipe_start, swipe_end) = crate::gesture::swipe_handlers(Callback::new(move |s| match s {
    crate::gesture::Swipe::Left => store.next(),
    crate::gesture::Swipe::Right => store.prev(),
  }));

  // 首次进入自动展示快捷键说明
  Effect::new(move |_| {
    if help_shown.get_value() || store.loading.get() || resume_open.get() {
      return;
    }
    help_shown.set_value(true);
    if store::help_seen(keys::HELP_SEEN_PRACTICE) {
      return;
    }
    set_timeout(
      move || {
        settings_open.try_set(true);
        store::mark_help_seen(keys::HELP_SEEN_PRACTICE);
      },
      Duration::from_millis(300),
    );
  });

  let switch_bank = Callback::new(move |b: Bank| {
    if b == bank.get_untracked() {
      return;
    }
    navigate(
      &bank_href("/practice", version.get_untracked().as_deref(), b),
      NavigateOptions {
        replace: true,
        ..Default::default()
      },
    );
  });

  let apply_order = move |next: PracticeOrder| {
    store.set_order(next);
    store::save_last_mode(next);
    if next == PracticeOrder::Sequential {
      try_prompt_resume();
    } else {
      search_open.set(false);
    }
  };

  let handle_set_order = Callback::new(move |next: PracticeOrder| {
    if next == store.order.get_untracked() {
      return;
    }
    let has_answers = store
      .answers
      .with_untracked(|a| a.values().any(|v| !v.is_empty()));
    if next == PracticeOrder::Random && has_answers {
      pending_order.set(Some(next));
      confirm_order_open.set(true);
      return;
    }
    apply_order(next);
  });

  let confirm_change_order = Callback::new(move |()| {
    if let Some(next) = pending_order.get_untracked() {
      apply_order(next);
    }
    confirm_order_open.set(false);
    pending_order.set(None);
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

  let matches = Signal::derive(move || {
    if store.order.get() != PracticeOrder::Sequential {
      return Vec::new();
    }
    let q = jump_input.with(|s| js_trim(s).to_uppercase());
    store.ordered(|qs| search(qs.iter().copied(), &q))
  });

  let on_jump = Callback::new(move |()| {
    if store.order.get_untracked() != PracticeOrder::Sequential {
      return;
    }
    let input = jump_input.get_untracked();
    let raw = js_trim(&input).to_uppercase();
    if raw.is_empty() {
      return;
    }
    match store.ordered(|qs| find_jump_target(qs, &input)) {
      Some(pos) => store.jump(pos),
      None => alert(&tf("exam.question-not-found", &[&(raw).to_string()])),
    }
  });

  let on_resume = Callback::new(move |()| {
    let Some(saved) = pending.get_untracked() else {
      return;
    };
    let total = store.all.with_untracked(|a| a.len());
    let idx: Vec<usize> = saved
      .order_indices
      .iter()
      .copied()
      .filter(|&i| i < total)
      .collect();
    let all = store.all.get_untracked();
    let answers: HashMap<String, Vec<String>> = idx
      .iter()
      .enumerate()
      .filter_map(|(pos, &i)| {
        let a = saved
          .answers_by_position
          .get(pos)?
          .as_ref()
          .filter(|a| !a.is_empty())?;
        Some((all[i].answer_key(pos), a.clone()))
      })
      .collect();
    let len = idx.len();
    store.recorded.set_value(answers.keys().cloned().collect());
    store.order.set(PracticeOrder::Sequential);
    store.order_idx.set(idx);
    store.answers.set(answers);
    store.index.set(saved.index.min(len.saturating_sub(1)));
    store.show_answer.set(saved.show_answer);
    store
      .show_explanation
      .set(saved.show_explanation.unwrap_or(true));
    if no_prompt.get_untracked() {
      store::save_no_resume(
        bank.get_untracked(),
        version.get_untracked().as_deref(),
        true,
      );
    }
    resume_open.set(false);
    pending.set(None);
  });

  let on_restart = Callback::new(move |()| {
    store::clear_practice(bank.get_untracked(), version.get_untracked().as_deref());
    store.set_order(store.order.get_untracked());
    if no_prompt.get_untracked() {
      store::save_no_resume(
        bank.get_untracked(),
        version.get_untracked().as_deref(),
        true,
      );
    }
    resume_open.set(false);
    pending.set(None);
  });

  use_question_shortcuts(Shortcuts {
    on_prev: Callback::new(move |()| store.prev()),
    on_next: Callback::new(move |()| store.next()),
    on_digit: Callback::new(move |(n, d): (usize, DigitDetail)| {
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
    enter_search: Some((
      Signal::derive(move || store.order.get() == PracticeOrder::Sequential),
      Callback::new(move |()| search_open.set(true)),
    )),
    on_help: Some(Callback::new(move |()| help_open.set(true))),
  });

  let percent = Signal::derive(move || {
    let n = store.len();
    if n == 0 {
      0
    } else {
      ((store.index.get() + 1) as f64 / n as f64 * 100.0).round() as i64
    }
  });
  let at_start = Signal::derive(move || store.index.get() == 0);
  let at_end = Signal::derive(move || store.index.get() + 1 >= store.len());
  let sequential = Signal::derive(move || store.order.get() == PracticeOrder::Sequential);
  let on_open_search = Callback::new(move |()| search_open.set(true));
  let on_open_settings = Callback::new(move |()| settings_open.set(true));

  let content = move || {
    if store.loading.get() {
      return view! { <div class="p-6">{move || t("exam.loading-questions-u-2026")}</div> }
        .into_any();
    }
    if store.len() == 0 {
      if unseen_only.get() {
        return view! {
          <div class="p-6">
            {move || t("exam.you-ve-completed-all")}
            <button type="button" class="ml-2 text-primary underline-offset-4 hover:underline" on:click=move |_| unseen_only.set(false)>
              {move || t("exam.practice-all")}
            </button>
          </div>
        }
        .into_any();
      }
      return view! { <div class="p-6">{move || t("exam.question-bank-unavailable-or")}</div> }
        .into_any();
    }
    view! {
      <div on:touchstart=swipe_start on:touchend=swipe_end>
        <PageContainer class="py-6 space-y-4 pb-24 sm:pb-20 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
        <PracticeHeaderBar
          percent=percent
          bank=bank
          on_switch_bank=switch_bank
          unique_only=unique_only
          unseen_only=unseen_only
          multi_only=multi_only
          bookmarked=bookmarked
          bookmark_disabled=current_is_variant
          on_toggle_bookmark=on_toggle_bookmark
          sequential=sequential
          on_open_search=on_open_search
          on_open_settings=on_open_settings
        />

        {move || {
          store
            .current()
            .map(|(i, q)| {
              let expl = q.clone();
              view! {
                {move || {
                  (store.streak.get() >= 2).then(|| {
                    view! {
                      <div class="flex items-center gap-1 text-xs font-medium text-emerald-700 dark:text-emerald-400">
                        <span>{tf("radio.correct-in-a-row", &[&store.streak.get().to_string()])}</span>
                        <span class="text-muted-foreground">{" · "}</span>
                        <span>{tf("radio.longest-streak", &[&store.best_streak.get().to_string()])}</span>
                      </div>
                    }
                  })
                }}
                <QuestionCard
                  index=i
                  total=store.len()
                  question=q
                  selected=selected
                  on_change=set_answer
                  show_answer=store.show_answer
                />
                {move || store.show_explanation.get().then(|| view! { <ExplanationCard question=expl.clone() /> })}
                <NoteEditor question_id=current_id />
              }
            })
        }}

        <PracticeBottomBar
          bank=bank
          order=store.order
          index=store.index
          len=Signal::derive(move || store.len())
          at_start=at_start
          at_end=at_end
          on_prev=Callback::new(move |()| store.prev())
          on_next=Callback::new(move |()| store.next())
        />
        </PageContainer>
      </div>
    }
    .into_any()
  };

  view! {
    <h1 class="sr-only">{move || t("exam.practice")}</h1>
    {content}
    <PracticeResumeDialog open=resume_open no_prompt=no_prompt on_restart=on_restart on_resume=on_resume />
    <PracticeSettingsDialog
      open=settings_open
      order=store.order
      on_change_order=handle_set_order
      show_answer=store.show_answer
      on_toggle_show_answer=Callback::new(move |_| store.show_answer.update(|v| *v = !*v))
      show_explanation=store.show_explanation
      on_toggle_show_explanation=Callback::new(move |_| store.show_explanation.update(|v| *v = !*v))
      variants=variants_on
      on_toggle_variants=Callback::new(move |_| variants_on.update(|v| *v = !*v))
    />
    <PracticeSearchDialog
      open=search_open
      input=jump_input
      matches=matches
      on_pick=Callback::new(move |pos| store.jump(pos))
      on_jump=on_jump
    />
    <Dialog open=confirm_order_open>
      <DialogHeader>
        <DialogTitle>{move || t("exam.changing-order-clears-answers")}</DialogTitle>
        <DialogDescription>{move || t("exam.switching-to-random-reshuffles")}</DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <Button
          variant=Variant::Outline
          size=Size::Default
          on_click=Callback::new(move |_| {
                    confirm_order_open.set(false);
                    pending_order.set(None);
                  })
        >
          {move || t("exam.cancel")}
        </Button>
        <Button
          variant=Variant::Default
          size=Size::Default
          on_click=Callback::new(move |_| confirm_change_order.run(()))
        >
          {move || t("exam.switch")}
        </Button>
      </DialogFooter>
    </Dialog>
    <MessageDialog open=error_open title=t("exam.load-failed") description=error_text confirm_text=t("exam.ok") />
    <ShortcutHelpDialog open=help_open />
  }
}

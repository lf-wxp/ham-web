//! 练习模式：顺序/随机练习、即时答案与解析、题号/关键词搜索、进度保存与恢复、跨题库「只看本类新增」。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ham_web_core::exam::shuffle_in_place;
use ham_web_core::practice::{PracticeOrder, find_jump_target, search, unique_to_bank};
use ham_web_core::saved_state::{PracticeSavedState, keys};
use ham_web_core::text::js_trim;
use ham_web_core::{Bank, QuestionItem};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use super::{DEFAULT_TITLE, bank_href, use_bank_query, use_no_site_footer};
use crate::cn::cn;
use crate::components::common::{
  BottomBar, ExplanationCard, MessageDialog, QuestionProgressHeader,
};
use crate::components::practice::{
  PracticeResumeDialog, PracticeSearchDialog, PracticeSettingsDialog,
};
use crate::components::question_card::QuestionCard;
use crate::data::{self, AppError, Questions};
use crate::icons::{Icon, IconKind};
use crate::shortcuts::{DigitDetail, Shortcuts, digit_answer, use_question_shortcuts};
use crate::store;
use crate::ui::{Size, Variant, button_class};
use crate::util::{alert, now_ms, random, set_title};

const PRESS: &str = "active:scale-[0.98] transition-transform";
const PRESS_FULL: &str = "w-full active:scale-[0.98] transition-transform";

#[derive(Clone, Copy)]
struct PracticeStore {
  /// 原始题库（顺序）。
  all: RwSignal<Questions>,
  /// 当前题序：`all` 的下标。
  order_idx: RwSignal<Vec<usize>>,
  index: RwSignal<usize>,
  answers: RwSignal<HashMap<String, Vec<String>>>,
  order: RwSignal<PracticeOrder>,
  show_answer: RwSignal<bool>,
  show_explanation: RwSignal<bool>,
  loading: RwSignal<bool>,
}

impl PracticeStore {
  fn new() -> Self {
    Self {
      all: RwSignal::new(Arc::new(Vec::new())),
      order_idx: RwSignal::new(Vec::new()),
      index: RwSignal::new(0),
      answers: RwSignal::new(HashMap::new()),
      order: RwSignal::new(PracticeOrder::Sequential),
      show_answer: RwSignal::new(true),
      show_explanation: RwSignal::new(true),
      loading: RwSignal::new(true),
    }
  }

  fn reset(self) {
    self.all.set(Arc::new(Vec::new()));
    self.order_idx.set(Vec::new());
    self.index.set(0);
    self.answers.set(HashMap::new());
    self.order.set(PracticeOrder::Sequential);
    self.show_answer.set(true);
    self.show_explanation.set(true);
    self.loading.set(true);
  }

  fn load(self, qs: Questions) {
    self.reset();
    self.order_idx.set((0..qs.len()).collect());
    self.all.set(qs);
    self.loading.set(false);
  }

  fn set_order(self, order: PracticeOrder) {
    let mut idx: Vec<usize> = (0..self.all.with_untracked(|a| a.len())).collect();
    if order == PracticeOrder::Random {
      let mut rng = random;
      shuffle_in_place(&mut idx, &mut rng);
    }
    self.order.set(order);
    self.order_idx.set(idx);
    self.index.set(0);
    self.answers.set(HashMap::new());
  }

  fn len(self) -> usize {
    self.order_idx.with(Vec::len)
  }

  fn next(self) {
    let max = self.len().saturating_sub(1);
    self.index.update(|i| *i = (*i + 1).min(max));
  }

  fn prev(self) {
    self.index.update(|i| *i = i.saturating_sub(1));
  }

  fn jump(self, i: usize) {
    self.index.set(i.min(self.len().saturating_sub(1)));
  }

  fn question_at(self, pos: usize) -> Option<QuestionItem> {
    let idx = self.order_idx.with(|o| o.get(pos).copied())?;
    self.all.with(|a| a.get(idx).cloned())
  }

  fn current(self) -> Option<(usize, QuestionItem)> {
    let i = self.index.get();
    self.question_at(i).map(|q| (i, q))
  }

  fn current_key(self) -> Option<String> {
    let i = self.index.get();
    self.question_at(i).map(|q| q.answer_key(i))
  }

  /// 当前题序下的题目引用列表。
  fn ordered<R>(self, f: impl FnOnce(&[&QuestionItem]) -> R) -> R {
    self.all.with(|all| {
      self
        .order_idx
        .with(|idx| f(&idx.iter().filter_map(|&i| all.get(i)).collect::<Vec<_>>()))
    })
  }
}

async fn load_questions(
  version: Option<&str>,
  bank: Bank,
  unique: bool,
) -> Result<Questions, AppError> {
  if !unique {
    return data::load_bank(version, bank, true).await;
  }
  let a = data::load_bank(version, Bank::A, false).await?;
  let b = data::load_bank(version, Bank::B, false).await?;
  let c = data::load_bank(version, Bank::C, false).await?;
  Ok(Arc::new(unique_to_bank(bank, &a, &b, &c)))
}

#[component]
pub fn PracticePage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  use_no_site_footer();
  let (version, bank) = use_bank_query();
  let navigate = use_navigate();
  let store = PracticeStore::new();

  let jump_input = RwSignal::new(String::new());
  let resume_open = RwSignal::new(false);
  let pending: RwSignal<Option<PracticeSavedState>> = RwSignal::new(None);
  let search_open = RwSignal::new(false);
  let settings_open = RwSignal::new(false);
  let no_prompt = RwSignal::new(false);
  let error_open = RwSignal::new(false);
  let error_text = RwSignal::new(String::new());
  let unique_only = RwSignal::new(false);
  let generation = StoredValue::new(0u32);
  let help_shown = StoredValue::new(false);

  Effect::new(move |_| {
    no_prompt.set(store::load_no_resume(bank.get(), version.get().as_deref()));
  });

  let try_prompt_resume = move || {
    let (b, v) = (bank.get_untracked(), version.get_untracked());
    if store::load_no_resume(b, v.as_deref()) {
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
    store.reset();
    pending.set(None);
    resume_open.set(false);
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    spawn_local(async move {
      let result = load_questions(v.as_deref(), b, unique).await;
      if generation.try_get_value() != Some(current) {
        return;
      }
      match result {
        Ok(qs) => {
          store.load(qs);
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
        Err(_) => {
          error_text.set(format!("题库 {b} 暂不可用"));
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
    if loading || order != PracticeOrder::Sequential {
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

  let handle_set_order = Callback::new(move |next: PracticeOrder| {
    store.set_order(next);
    store::save_last_mode(next);
    if next == PracticeOrder::Sequential {
      try_prompt_resume();
    } else {
      search_open.set(false);
    }
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
      None => alert(&format!("未找到题号：{raw}")),
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
  });

  let percent = Signal::derive(move || {
    let n = store.len();
    if n == 0 {
      0
    } else {
      ((store.index.get() + 1) as f64 / n as f64 * 100.0).round() as i64
    }
  });
  let at_start = move || store.index.get() == 0;
  let at_end = move || store.index.get() + 1 >= store.len();
  let toggle_class = |on: bool, base: &str| {
    cn(&[
      base,
      if on {
        "bg-primary text-primary-foreground"
      } else {
        "hover:bg-accent"
      },
    ])
  };

  let content = move || {
    if store.loading.get() {
      return view! { <div class="p-6">"加载题库中..."</div> }.into_any();
    }
    if store.len() == 0 {
      return view! { <div class="p-6">"题库暂不可用或为空"</div> }.into_any();
    }
    view! {
      <div class="container mx-auto px-4 py-6 max-w-4xl space-y-4 pb-24 sm:pb-20 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
        <QuestionProgressHeader
          percent=percent
          left=move || {
            view! {
              <span class="text-sm text-muted-foreground">"题库类别"</span>
              <div class="flex overflow-hidden rounded-lg border">
                {Bank::ALL
                  .into_iter()
                  .map(|b| {
                    view! {
                      <button
                        type="button"
                        class=move || toggle_class(bank.get() == b, "px-3 py-1.5 text-sm font-medium transition-colors")
                        on:click=move |_| switch_bank.run(b)
                      >
                        {b.as_str()}
                        " 类"
                      </button>
                    }
                  })
                  .collect_view()}
              </div>
              <button
                type="button"
                class=move || toggle_class(unique_only.get(), "rounded-lg border px-3 py-1.5 text-sm font-medium transition-colors")
                on:click=move |_| unique_only.update(|v| *v = !*v)
              >
                "只看本类新增"
              </button>
            }
          }
          right=move || {
            view! {
              {move || {
                (store.order.get() == PracticeOrder::Sequential)
                  .then(|| {
                    view! {
                      <button
                        class=button_class(Variant::Outline, Size::Icon, "")
                        aria-label="搜索"
                        title="搜索"
                        on:click=move |_| search_open.set(true)
                      >
                        <Icon kind=IconKind::Search class="h-4 w-4" />
                      </button>
                    }
                  })
              }}
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
        />

        {move || {
          store
            .current()
            .map(|(i, q)| {
              let expl = q.clone();
              view! {
                <QuestionCard
                  index=i
                  total=store.len()
                  question=q
                  selected=selected
                  on_change=set_answer
                  show_answer=store.show_answer
                />
                {move || store.show_explanation.get().then(|| view! { <ExplanationCard question=expl.clone() /> })}
              }
            })
        }}

        <BottomBar
          stats=move || {
            view! {
              "题库：" {move || bank.get().as_str()} " 类｜模式：" {move || store.order.get().label()} "｜进度："
              {move || store.index.get() + 1} " / " {move || store.len()}
            }
          }
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
              <button class=button_class(Variant::Default, Size::Default, PRESS) disabled=at_end on:click=move |_| store.next()>
                "下一题"
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
        />
      </div>
    }
    .into_any()
  };

  view! {
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
    />
    <PracticeSearchDialog
      open=search_open
      input=jump_input
      matches=matches
      on_pick=Callback::new(move |pos| store.jump(pos))
      on_jump=on_jump
    />
    <MessageDialog open=error_open title="加载失败" description=error_text confirm_text="知道了" />
  }
}

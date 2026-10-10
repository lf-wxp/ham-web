//! 知识卡片：Q 简语、通联缩语、字母解释法、莫尔斯电码与术语的间隔复习。

use ham_web_core::card_review::{Deck, Grade, Schedule};
use ham_web_core::glossary::GlossaryEntry;
use ham_web_core::morse::{DIGITS, LETTERS};
use ham_web_core::phonetic::PHONETIC;
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;
use wasm_bindgen::JsCast;

use crate::components::common::Loading;
use crate::components::common::PageContainer;
use crate::data;
use crate::gesture::{Swipe, swipe_handlers};
use crate::i18n::{t, tf, tp};
use crate::morse_audio::play_morse;
use crate::pages::SLANG_CATEGORY;
use crate::speech::speak_en;
use crate::ui::{Button, Size, Variant};
use crate::util::{local_today, now_ms, set_title, storage};

const KEY: &str = "card-review";
/// 莫尔斯卡片试听速度。
const MORSE_WPM: f64 = 18.0;

pub fn load_schedule() -> Schedule {
  storage::get_json(KEY).unwrap_or_default()
}

fn save_schedule(s: &Schedule) {
  storage::set_json(KEY, s);
}

/// 一张卡片的正反面。
#[derive(Clone, PartialEq)]
struct Card {
  id: String,
  front: String,
  /// 正面的提示（如英文全称）。
  hint: Option<String>,
  back: String,
  detail: Option<String>,
  morse: Option<&'static str>,
  speak: Option<String>,
}

fn is_qcode(e: &GlossaryEntry) -> bool {
  let t = e.term.as_str();
  t.len() == 3 && t.starts_with('Q') && t.is_ascii()
}

fn glossary_card(deck: Deck, e: &GlossaryEntry) -> Card {
  let front = match e.abbr.as_deref() {
    Some(a) if a != e.term => format!("{}（{a}）", e.term),
    _ => e.term.clone(),
  };
  Card {
    id: deck.card_id(&e.term),
    front,
    hint: None,
    back: e.desc.clone(),
    detail: e.en.clone(),
    morse: None,
    speak: None,
  }
}

async fn load_cards(deck: Deck) -> Vec<Card> {
  match deck {
    Deck::Phonetic => PHONETIC
      .iter()
      .map(|p| Card {
        id: deck.card_id(p.letter),
        front: p.letter.to_owned(),
        hint: None,
        back: p.word.to_owned(),
        detail: Some(p.pronunciation.to_owned()),
        morse: None,
        speak: Some(p.word.to_owned()),
      })
      .collect(),
    Deck::Morse => LETTERS
      .iter()
      .chain(DIGITS)
      .map(|m| Card {
        id: deck.card_id(m.ch),
        front: m.ch.to_owned(),
        hint: Some(t("exam.think-about-its-dots")),
        back: m
          .code
          .chars()
          .map(|c| if c == '.' { "•" } else { "—" })
          .collect::<Vec<_>>()
          .join(" "),
        detail: None,
        morse: Some(m.code),
        speak: None,
      })
      .collect(),
    Deck::QCode | Deck::Abbrev | Deck::Glossary => {
      let g = data::load_glossary().await;
      g.entries()
        .iter()
        .filter(|e| e.common)
        .filter(|e| {
          let slang = e.category_key() == SLANG_CATEGORY;
          match deck {
            Deck::QCode => slang && is_qcode(e),
            Deck::Abbrev => slang && e.term.is_ascii() && !is_qcode(e),
            _ => !slang,
          }
        })
        .map(|e| glossary_card(deck, e))
        .collect()
    }
  }
}

fn days_label(days: f64) -> String {
  if days <= 0.0 {
    t("exam.come-back-in-10")
  } else if days < 1.5 {
    t("exam.review-tomorrow")
  } else {
    tp(
      "common.review-again-in-days",
      days,
      &[&format!("{days:.0}")],
    )
  }
}

fn typing_in_field(e: &web_sys::KeyboardEvent) -> bool {
  e.target()
    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    .is_some_and(|el| {
      matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
        || el.closest("[role=dialog]").ok().flatten().is_some()
    })
}

#[component]
pub fn CardsPage() -> impl IntoView {
  set_title("shell.study-cards");
  let query = use_query_map();
  let schedule = RwSignal::new(load_schedule());
  let initial = query
    .with_untracked(|q| q.get("deck").and_then(|d| Deck::from_param(&d)))
    .or_else(|| {
      schedule.with_untracked(|s| {
        s.due_by_deck(now_ms())
          .into_iter()
          .max_by_key(|&(_, n)| n)
          .map(|(d, _)| d)
      })
    })
    .unwrap_or(Deck::QCode);
  let deck = RwSignal::new(initial);
  let cards = RwSignal::new(None::<Vec<Card>>);
  let queue = RwSignal::new(Vec::<String>::new());
  let pos = RwSignal::new(0usize);
  let revealed = RwSignal::new(false);
  let feedback = RwSignal::new(None::<String>);
  let reviewed = RwSignal::new(0usize);

  Effect::new(move |_| {
    let d = deck.get();
    cards.set(None);
    feedback.set(None);
    spawn_local(async move {
      let list = load_cards(d).await;
      if deck.get_untracked() != d {
        return;
      }
      let ids: Vec<String> = list.iter().map(|c| c.id.clone()).collect();
      queue.set(schedule.with_untracked(|s| s.queue(d, &ids, now_ms(), &local_today())));
      pos.set(0);
      revealed.set(false);
      reviewed.set(0);
      cards.set(Some(list));
    });
  });

  let current = Memo::new(move |_| {
    let id = queue.with(|q| q.get(pos.get()).cloned())?;
    cards.with(|c| c.as_ref()?.iter().find(|c| c.id == id).cloned())
  });

  let reveal = move || {
    if current.get_untracked().is_none() || revealed.get_untracked() {
      return;
    }
    revealed.set(true);
    if let Some(code) = current.get_untracked().and_then(|c| c.morse) {
      play_morse(code, MORSE_WPM);
    }
  };

  let grade = move |g: Grade| {
    let Some(card) = current.get_untracked() else {
      return;
    };
    if !revealed.get_untracked() {
      return;
    }
    let mut days = 0.0;
    schedule.update(|s| {
      days = s.grade(&card.id, g, now_ms(), &local_today());
      save_schedule(s);
    });
    // 忘了的卡放到本轮末尾，趁热再记一遍
    if g == Grade::Again {
      queue.update(|q| q.push(card.id.clone()));
    }
    feedback.set(Some(days_label(days)));
    reviewed.update(|n| *n += 1);
    revealed.set(false);
    pos.update(|p| *p += 1);
  };

  let handle = window_event_listener(ev::keydown, move |e| {
    if e.repeat() || e.ctrl_key() || e.meta_key() || e.alt_key() || typing_in_field(&e) {
      return;
    }
    match e.key().as_str() {
      " " | "Enter" if !revealed.get_untracked() && current.get_untracked().is_some() => {
        e.prevent_default();
        reveal();
      }
      "1" => grade(Grade::Again),
      "2" => grade(Grade::Hard),
      "3" => grade(Grade::Good),
      _ => {}
    }
  });
  on_cleanup(move || handle.remove());

  let (swipe_start, swipe_end) = swipe_handlers(Callback::new(move |s| {
    match (revealed.get_untracked(), s) {
      (false, _) => reveal(),
      (true, Swipe::Right) => grade(Grade::Good),
      (true, Swipe::Left) => grade(Grade::Again),
    }
  }));

  let chip = move |d: Deck| {
    let due = move || schedule.with(|s| s.due_by_deck(now_ms()).get(&d).copied().unwrap_or(0));
    view! {
      <button
        type="button"
        aria-pressed=move || (deck.get() == d).to_string()
        class=move || if deck.get() == d {
          "inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border border-primary bg-primary px-3 py-1 text-xs font-medium text-primary-foreground"
        } else {
          "inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border px-3 py-1 text-xs text-muted-foreground hover:text-foreground"
        }
        on:click=move |_| deck.set(d)
      >
        {d.label()}
        {move || (due() > 0).then(|| view! {
          <span class="rounded-full bg-amber-500/20 px-1.5 tabular-nums text-amber-800 dark:text-amber-300">{due()}</span>
        })}
      </button>
    }
  };

  let progress = move || {
    let d = deck.get();
    let ids: Vec<String> = cards.with(|c| {
      c.as_ref()
        .map(|l| l.iter().map(|c| c.id.clone()).collect())
        .unwrap_or_default()
    });
    let (learned, mature) = schedule.with(|s| s.progress(&ids));
    let left = schedule.with(|s| s.new_left(d, &local_today()));
    tp(
      "common.cards-learned-mature-new",
      ids.len(),
      &[
        &(ids.len()).to_string(),
        &(learned).to_string(),
        &(mature).to_string(),
        &(left).to_string(),
      ],
    )
  };

  // 记忆巩固分段条：熟记（≥21 天）与学习中（已学未熟记）占当前卡组的比例。
  let mastery_bar = move || {
    // 订阅 deck 变化（切换卡组时重算），计算本身只用 cards / schedule。
    let _ = deck.get();
    let ids: Vec<String> = cards.with(|c| {
      c.as_ref()
        .map(|l| l.iter().map(|c| c.id.clone()).collect())
        .unwrap_or_default()
    });
    let (learned, mature) = schedule.with(|s| s.progress(&ids));
    let total = ids.len();
    (total > 0).then(|| {
      let pct = |n: usize| -> String { format!("{:.2}%", n as f64 / total as f64 * 100.0) };
      view! {
        <div
          data-testid="card-mastery-bar"
          class="flex h-2 w-full overflow-hidden rounded-full bg-muted"
          title=tf("learning.learned-mature", &[&learned.to_string(), &mature.to_string()])
        >
          <div class="bg-emerald-500/70" style=format!("width: {}", pct(mature))></div>
          <div class="bg-sky-400/70" style=format!("width: {}", pct(learned - mature))></div>
        </div>
      }
    })
  };

  let card_view = move || {
    if cards.with(Option::is_none) {
      return view! { <Loading label=t("exam.loading-cards") class="py-16" /> }.into_any();
    }
    let Some(c) = current.get() else {
      let other: Vec<(Deck, usize)> = schedule.with(|s| {
        s.due_by_deck(now_ms())
          .into_iter()
          .filter(|&(d, n)| d != deck.get_untracked() && n > 0)
          .collect()
      });
      return view! {
        <div class="space-y-3 py-10 text-center">
          <p class="text-base font-semibold">{tf("common.today-s-cards-are", &[(deck.get_untracked().label())])}</p>
          <p class="text-sm text-muted-foreground">
            {move || if reviewed.get() > 0 { tp("common.reviewed-cards-this-round", reviewed.get(), &[&(reviewed.get()).to_string()]) } else { t("exam.due-cards-and-new") }}
          </p>
          <div class="flex flex-wrap justify-center gap-2">
            {other.into_iter().map(|(d, n)| view! {
              <Button
                variant=Variant::Outline
                size=Size::Sm
                on_click=Callback::new(move |_| deck.set(d))
              >
                {tf("common.review-2", &[(d.label()), &(n).to_string()])}
              </Button>
            }).collect_view()}
          </div>
        </div>
      }
      .into_any();
    };
    let total = queue.with(Vec::len);
    let speak = c.speak.clone();
    let morse = c.morse;
    view! {
      <div on:touchstart=swipe_start on:touchend=swipe_end class="space-y-4">
        <div class="flex items-center justify-between text-xs text-muted-foreground">
          <span class="tabular-nums">{tf("common.card", &[&(pos.get() + 1).to_string(), &(total).to_string()])}</span>
          <span>{move || feedback.get().unwrap_or_default()}</span>
        </div>
        <button
          type="button"
          class="flex min-h-48 w-full flex-col items-center justify-center gap-2 rounded-xl border bg-card p-6 text-center shadow-sm"
          aria-label=if revealed.get_untracked() { t("knowledge.back-of-the-card") } else { t("knowledge.flip-to-see-the") }
          on:click=move |_| reveal()
        >
          <span class="font-mono text-3xl font-bold tracking-wide">{c.front.clone()}</span>
          {c.hint.clone().filter(|_| !revealed.get_untracked()).map(|h| view! { <span class="text-xs text-muted-foreground">{h}</span> })}
          {move || revealed.get().then(|| {
            let c = c.clone();
            view! {
              <span class="mt-3 border-t pt-3 text-base leading-relaxed" data-testid="card-back">{c.back}</span>
              {c.detail.map(|d| view! { <span class="text-xs text-muted-foreground">{d}</span> })}
            }
          })}
          {move || (!revealed.get()).then(|| view! { <span class="mt-3 text-xs text-muted-foreground">{move || t("exam.click-press-space-or")}</span> })}
        </button>
        <div class="flex flex-wrap items-center justify-center gap-2">
          {speak.map(|w| view! {
            <Button
              variant=Variant::Ghost
              size=Size::Sm
              on_click=Callback::new(move |_| speak_en(&w))
            >{move || t("exam.read-aloud")}</Button>
          })}
          {morse.map(|code| view! {
            <Button
              variant=Variant::Ghost
              size=Size::Sm
              on_click=Callback::new(move |_| play_morse(code, MORSE_WPM))
            >{move || t("exam.preview")}</Button>
          })}
        </div>
        <div class="grid grid-cols-3 gap-2">
          <Button
            variant=Variant::Outline
            size=Size::Default
            disabled=Signal::derive(move || !revealed.get())
            on_click=Callback::new(move |_| grade(Grade::Again))
          >
            {move || t("exam.forgot")} <kbd class="ml-1 hidden text-xs text-muted-foreground sm:inline">"1"</kbd>
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Default
            disabled=Signal::derive(move || !revealed.get())
            on_click=Callback::new(move |_| grade(Grade::Hard))
          >
            {move || t("exam.unsure")} <kbd class="ml-1 hidden text-xs text-muted-foreground sm:inline">"2"</kbd>
          </Button>
          <Button
            variant=Variant::Default
            size=Size::Default
            disabled=Signal::derive(move || !revealed.get())
            on_click=Callback::new(move |_| grade(Grade::Good))
          >
            {move || t("exam.remember-2")} <kbd class="ml-1 hidden text-xs opacity-70 sm:inline">"3"</kbd>
          </Button>
        </div>
        <p class="text-center text-xs text-muted-foreground">{move || t("exam.after-flipping-swipe-right")}</p>
      </div>
    }
    .into_any()
  };

  view! {
    <PageContainer class="space-y-4 py-6 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <div>
        <h1 class="text-base font-semibold leading-tight">{move || t("shell.study-cards")}</h1>
        <p class="text-xs text-muted-foreground">{move || t("exam.reviews-follow-a-forgetting")}</p>
      </div>
      <div class="flex gap-2 overflow-x-auto pb-1" role="group" aria-label=move || t("exam.deck")>
        {Deck::ALL.into_iter().map(chip).collect_view()}
      </div>
      <p class="text-xs text-muted-foreground tabular-nums">{progress}</p>
      {mastery_bar}
      {card_view}
    </PageContainer>
  }
}

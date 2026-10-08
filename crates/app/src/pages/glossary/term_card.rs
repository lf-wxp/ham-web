use ham_web_core::glossary::GlossaryEntry;
use leptos::prelude::*;

use super::{Haystacks, browse_href, category_meta, is_countable};
use crate::i18n::{t, tf};

#[component]
pub(super) fn TermCard(
  entry: &'static GlossaryEntry,
  haystacks: RwSignal<Option<Haystacks>>,
  on_jump: Callback<String>,
) -> impl IntoView {
  let (cat_name, color) = category_meta(entry.category_key());
  let term_class = if entry.term.is_ascii() {
    "font-mono text-base font-semibold"
  } else {
    "text-base font-semibold"
  };
  let abbr = entry.abbr.as_deref().filter(|a| *a != entry.term);
  let countable = is_countable(&entry.term);

  // 题库出现次数（懒计算，仅对当前渲染的卡片统计）
  let counts = Memo::new(move |_| {
    if !countable {
      return None;
    }
    let needle = entry.term.to_lowercase();
    haystacks.with(|h| {
      h.as_ref().map(|banks| {
        banks
          .iter()
          .map(|(b, texts)| (*b, texts.iter().filter(|t| t.contains(&needle)).count()))
          .collect::<Vec<_>>()
      })
    })
  });

  let footer = move || {
    if !countable {
      return None;
    }
    let body = match counts.get() {
      None => view! { <span>{move || t("knowledge.counting-occurrences-in-the")}</span> }.into_any(),
      Some(list) if list.iter().all(|(_, n)| *n == 0) => view! { <span>{move || t("knowledge.no-questions-in-the")}</span> }.into_any(),
      Some(list) => view! {
        <span>{move || t("knowledge.appears-in-the-bank")}</span>
        {list
          .into_iter()
          .filter(|(_, n)| *n > 0)
          .map(|(b, n)| {
            view! {
              <a
                href=browse_href(b, &entry.term)
                class="rounded-md border px-2 py-0.5 transition-colors hover:bg-accent hover:text-foreground"
                title=tf("common.view-questions-containing-in", &[&(b).to_string(), &(entry.term).to_string()])
              >
                {b.as_str()}
                {t("knowledge.class")}
                <span class="font-semibold tabular-nums">{n}</span>
              </a>
            }
          })
          .collect_view()}
      }
      .into_any(),
    };
    Some(view! {
      <div class="mt-3 flex flex-wrap items-center gap-2 border-t border-dashed pt-3 text-xs text-muted-foreground">
        {body}
      </div>
    })
  };

  view! {
    <article class="flex flex-col rounded-xl border bg-card p-4 sm:p-5">
      <div class="mb-2 flex flex-wrap items-center gap-2">
        <h2 class=term_class>{entry.term.as_str()}</h2>
        {abbr
          .map(|a| {
            view! {
              <span class="rounded-md bg-muted px-2 py-0.5 font-mono text-xs font-semibold text-foreground">{a}</span>
            }
          })}
        <span
          class="ml-auto rounded-full px-2.5 py-0.5 text-xs font-semibold text-white"
          style=format!("background: {color}")
        >
          {cat_name}
        </span>
      </div>
      {entry.en.as_deref().map(|en| view! { <div class="mb-2 text-xs italic text-muted-foreground">{en}</div> })}
      <p class="text-sm leading-6 text-muted-foreground">{entry.desc.as_str()}</p>
      {(!entry.aliases.is_empty())
        .then(|| {
          view! { <div class="mt-2 text-xs text-muted-foreground">{t("knowledge.also-known-as")} {entry.aliases.join("、")}</div> }
        })}
      {entry
        .see
        .as_deref()
        .map(|see| {
          view! {
            <div class="mt-2 text-xs text-muted-foreground">
              {move || t("knowledge.see-also")}
              <button
                type="button"
                class="font-medium text-foreground underline-offset-2 hover:underline"
                on:click=move |_| on_jump.run(see.to_owned())
              >
                {see}
              </button>
            </div>
          }
        })}
      <div class="mt-auto">{footer}</div>
    </article>
  }
}

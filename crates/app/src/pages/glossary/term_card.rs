use ham_web_core::glossary::GlossaryEntry;
use leptos::prelude::*;

use super::{Haystacks, browse_href, category_meta, is_countable};
use crate::i18n::{t, tf};
use crate::ui::Tooltip;

#[component]
pub(super) fn TermCard(
  entry: &'static GlossaryEntry,
  haystacks: RwSignal<Option<Haystacks>>,
  on_jump: Callback<String>,
) -> impl IntoView {
  let (cat_name, cat_tone) = category_meta(entry.category_key());
  let term_class = if entry.term.is_ascii() {
    "font-mono text-base"
  } else {
    "text-base"
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

  let modern = crate::kit::is_modern_at_mount();
  let footer = move || {
    if !countable {
      return None;
    }
    // 标签：现代风是一枚安静的小标题，像素风是一行说明文字。
    let label = |text: String| {
      if modern {
        view! { <span class="md-term-foot-label">{text}</span> }.into_any()
      } else {
        view! { <span>{text}</span> }.into_any()
      }
    };
    let body = match counts.get() {
      None => label(t("knowledge.counting-occurrences-in-the")),
      Some(list) if list.iter().all(|(_, n)| *n == 0) => {
        label(t("knowledge.no-questions-in-the"))
      }
      Some(list) => view! {
        {label(t("knowledge.appears-in-the-bank"))}
        {list
          .into_iter()
          .filter(|(_, n)| *n > 0)
          .map(|(b, n)| {
            let tip = tf("common.view-questions-containing-in", &[&(b).to_string(), &(entry.term).to_string()]);
            // 现代风用胶囊；像素风仍是描边方块。外层不再统一套 `a` 样式。
            let item_class = if modern {
              "md-chip md-chip-sm"
            } else {
              "inline-flex items-center border px-2 py-0.5 transition-colors hover:bg-accent hover:text-foreground pointer-coarse:min-h-8"
            };
            // 这两段是**两套皮肤共用**的 `body`：现代类名必须按风格分叉，
            // 否则像素风里会命中 `.md-mono` 的等宽/字号规则（样式表不分层）。
            let bank_cls = if modern { "md-mono" } else { "font-mono" };
            let count_cls = if modern { "md-term-foot-count tabular-nums" } else { "tabular-nums" };
            view! {
              <Tooltip label=tip>
                <a href=browse_href(b, &entry.term) class=item_class>
                  <span class=bank_cls>{b.as_str()}</span>
                  {t("knowledge.class")}
                  <span class=count_cls>{n}</span>
                </a>
              </Tooltip>
            }
          })
          .collect_view()}
      }
      .into_any(),
    };
    Some(if modern {
      view! { <div class="md-term-foot">{body}</div> }.into_any()
    } else {
      view! {
        <div class="mt-3 flex flex-wrap items-center gap-2 border-t border-dashed pt-3 text-xs text-muted-foreground">
          {body}
        </div>
      }
        .into_any()
    })
  };

  if crate::kit::is_modern_at_mount() {
    return view! {
      <article class="md-term">
        <div class="md-term-head">
          <h2 class=if entry.term.is_ascii() { "md-term-title md-mono" } else { "md-term-title" }>
            {move || crate::data::kt_tracked(entry.term.as_str())}
          </h2>
          {abbr
            .map(|a| view! { <span class="md-term-abbr">{a}</span> })}
          <span class="md-term-cat">{move || crate::data::core_text_tracked(cat_name)}</span>
        </div>
        {entry
          .en
          .as_deref()
          .map(|en| view! { <p class="md-term-en">{en}</p> })}
        <p class="md-term-desc">{move || crate::data::kt_tracked(entry.desc.as_str())}</p>
        {(!entry.aliases.is_empty())
          .then(|| {
            view! {
              <p class="md-term-meta">
                {move || t("knowledge.also-known-as")} " "
                {move || entry.aliases.iter().map(|a| crate::data::kt_tracked(a)).collect::<Vec<_>>().join("、")}
              </p>
            }
          })}
        {entry
          .see
          .as_deref()
          .map(|see| {
            view! {
              <p class="md-term-meta">
                {move || t("knowledge.see-also")} " "
                <button
                  type="button"
                  class="md-link-inline"
                  on:click=move |_| on_jump.run(see.to_owned())
                >
                  {see}
                </button>
              </p>
            }
          })}
        {move || footer()}
      </article>
    }
    .into_any();
  }
  view! {
    <article class=crate::kit::card_class_with("flex flex-col p-4 sm:p-5")>
      <div class="mb-2 flex flex-wrap items-center gap-2">
        <h2 class=term_class>{move || crate::data::kt_tracked(entry.term.as_str())}</h2>
        {abbr
          .map(|a| {
            view! {
              <span class="bg-muted px-2 py-0.5 font-mono text-xs text-foreground">{a}</span>
            }
          })}
        <span class=format!("ml-auto border-2 px-2 py-0.5 text-xs text-foreground {}", cat_tone.map_or("bg-muted", crate::tone::chip))>
          {move || crate::data::core_text_tracked(cat_name)}
        </span>
      </div>
      {entry.en.as_deref().map(|en| view! { <div class="mb-2 text-xs italic text-muted-foreground">{en}</div> })}
      <p class="text-sm leading-6 text-muted-foreground">{move || crate::data::kt_tracked(entry.desc.as_str())}</p>
      {(!entry.aliases.is_empty())
        .then(|| {
          view! { <div class="mt-2 text-xs text-muted-foreground">{move || t("knowledge.also-known-as")} {move || entry.aliases.iter().map(|a| crate::data::kt_tracked(a)).collect::<Vec<_>>().join("、")}</div> }
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
                class=crate::kit::link_class()
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
  .into_any()
}

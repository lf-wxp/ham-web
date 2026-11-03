//! 现代风跳题搜索：输入框 + 命中列表 + 「跳转」。
//! 命中摘要复用 `practice_search_dialog` 的 `match_snippet`（同一套高亮口径）。

use ham_web_core::practice::SearchMatch;
use leptos::prelude::*;

use crate::components::practice::practice_search_dialog::match_snippet::match_snippet;
use crate::i18n::{t, tf, tp};

use super::dialog::ModernDialog;

#[component]
pub fn ModernPracticeSearch(
  open: RwSignal<bool>,
  input: RwSignal<String>,
  #[prop(into)] matches: Signal<Vec<SearchMatch>>,
  on_pick: Callback<usize>,
  on_jump: Callback<()>,
) -> impl IntoView {
  let query_upper = move || input.with(|s| s.trim().to_uppercase());
  let has_input = move || input.with(|s| !s.trim().is_empty());
  let jump = move || {
    on_jump.run(());
    open.set(false);
  };

  view! {
    <ModernDialog open=open title=Signal::derive(move || t("exam.search-questions"))>
      <p class="md-muted md-sm">{move || t("exam.type-a-question-number")}</p>

      <super::ModernInput
        value=input
        on_input=Callback::new(move |v: String| input.set(v))
        kind="search"
        placeholder=Signal::derive(move || t("exam.question-number-or-keyword"))
        aria_label=Signal::derive(move || t("exam.search-questions"))
        on_enter=Callback::new(move |()| jump())
      />

      {move || {
        has_input()
          .then(|| {
            view! {
              <span class="md-muted md-sm">
                {move || {
                  let n = matches.with(Vec::len);
                  tp("exam.matches", n, &[&n.to_string()])
                }}
              </span>
            }
          })
      }}

      {move || {
        let list = matches.get();
        if list.is_empty() {
          let msg = if has_input() { t("exam.no-matches") } else { t("exam.type-to-search") };
          return view! { <p class="md-muted md-sm">{msg}</p> }.into_any();
        }
        let q = query_upper();
        view! {
          <ul class="md-search-list">
            {list
              .into_iter()
              .take(10)
              .map(|m| {
                let pos = m.pos;
                view! {
                  <li>
                    <button
                      type="button"
                      class="md-search-row"
                      on:click=move |_| {
                        on_pick.run(pos);
                        open.set(false);
                      }
                    >
                      <span class="md-search-key">{m.j}</span>
                      <span class="md-search-text">{match_snippet(&m.text, &q)}</span>
                      <span class="md-muted md-sm md-mono">
                        {tf("exam.question-number", &[&(pos + 1).to_string()])}
                      </span>
                    </button>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        }
          .into_any()
      }}

      <div class="md-dialog-actions">
        <button type="button" class="md-btn md-btn-accent" on:click=move |_| jump()>
          {move || t("exam.go")}
        </button>
      </div>
    </ModernDialog>
  }
}

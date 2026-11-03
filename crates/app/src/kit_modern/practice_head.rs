//! 练习页顶部：一行进度读数 + 进度条 + 题库 / 筛选胶囊 + 右侧图标操作。
//!
//! 与像素版 `pages/practice/header_bar.rs` 同一份入参、同一批回调，只换呈现：
//! 像素版是「进度条 + 一排 chip + 描边图标按钮」，这里把进度做成渐变细条、
//! 题库切换做成胶囊分段，操作收成三个图标按钮。

use ham_web_core::Bank;
use leptos::prelude::*;

use crate::i18n::{bank_class, t};

use super::{MIcon, ModernIcon};

#[component]
pub fn ModernPracticeHeader(
  #[prop(into)] percent: Signal<i64>,
  #[prop(into)] progress_label: Signal<String>,
  #[prop(into)] bank: Signal<Bank>,
  on_switch_bank: Callback<Bank>,
  unique_only: RwSignal<bool>,
  unseen_only: RwSignal<bool>,
  multi_only: RwSignal<bool>,
  bookmarked: RwSignal<bool>,
  #[prop(into)] bookmark_disabled: Signal<bool>,
  on_toggle_bookmark: Callback<()>,
  #[prop(into)] sequential: Signal<bool>,
  on_open_search: Callback<()>,
  on_open_settings: Callback<()>,
) -> impl IntoView {
  view! {
    <div class="md-practice-head">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <span class="md-mono md-sm md-muted">{move || progress_label.get()}</span>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="md-icon-btn"
            aria-label=move || t("exam.bookmark")
            aria-pressed=move || bookmarked.get().to_string()
            disabled=bookmark_disabled
            on:click=move |_| on_toggle_bookmark.run(())
          >
            <ModernIcon kind=MIcon::Bookmark />
          </button>
          {move || {
            sequential
              .get()
              .then(|| {
                view! {
                  <button
                    type="button"
                    class="md-icon-btn"
                    aria-label=move || t("shell.search")
                    on:click=move |_| on_open_search.run(())
                  >
                    <ModernIcon kind=MIcon::Search />
                  </button>
                }
              })
          }}
          <button
            type="button"
            class="md-icon-btn"
            aria-label=move || t("exam.settings")
            on:click=move |_| on_open_settings.run(())
          >
            <ModernIcon kind=MIcon::Settings />
          </button>
        </div>
      </div>

      <div class="md-progress" role="presentation">
        <div
          class="md-progress-fill"
          style=move || format!("width: {}%", percent.get().clamp(0, 100))
        ></div>
      </div>

      <div class="md-chip-row">
        <span class="md-sm md-muted">{move || t("exam.bank-class")}</span>
        {Bank::ALL
          .into_iter()
          .map(|b| {
            view! {
              <button
                type="button"
                class="md-chip"
                aria-pressed=move || (bank.get() == b).to_string()
                on:click=move |_| on_switch_bank.run(b)
              >
                {move || bank_class(b.as_str())}
              </button>
            }
          })
          .collect_view()}

        <span class="md-chip-row-divider" aria-hidden="true"></span>

        <button
          type="button"
          class="md-chip"
          aria-pressed=move || unique_only.get().to_string()
          on:click=move |_| unique_only.update(|v| *v = !*v)
        >
          {move || t("exam.new-in-this-class")}
        </button>
        <button
          type="button"
          class="md-chip"
          aria-pressed=move || unseen_only.get().to_string()
          on:click=move |_| unseen_only.update(|v| *v = !*v)
        >
          {move || t("exam.unseen-only")}
        </button>
        <button
          type="button"
          class="md-chip"
          aria-pressed=move || multi_only.get().to_string()
          on:click=move |_| multi_only.update(|v| *v = !*v)
        >
          {move || t("exam.multi-answer-only")}
        </button>
      </div>

      // 键位提示：快捷键以前只写在文档里，界面上看不到。
      <div class="md-keys" aria-hidden="true">
        <span><kbd class="md-kbd">"1–4"</kbd>{move || t("exam.keys-pick")}</span>
        <span><kbd class="md-kbd">"Enter"</kbd>{move || t("exam.check-answer")}</span>
        <span><kbd class="md-kbd">"← →"</kbd>{move || t("exam.keys-turn")}</span>
      </div>
    </div>
  }
}

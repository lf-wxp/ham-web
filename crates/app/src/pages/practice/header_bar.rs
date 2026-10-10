//! 练习页顶部进度头：进度条、题库类别切换、只看本类新增 / 只练没做过 / 只练多选、收藏、搜索与设置入口。

use ham_web_core::Bank;
use leptos::prelude::*;

use crate::cn::cn;
use crate::components::common::QuestionProgressHeader;
use crate::i18n::{bank_class, t};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, Size, Variant};

#[component]
pub(super) fn PracticeHeaderBar(
  #[prop(into)] percent: Signal<i64>,
  #[prop(into)] bank: Signal<Bank>,
  on_switch_bank: Callback<Bank>,
  unique_only: RwSignal<bool>,
  unseen_only: RwSignal<bool>,
  multi_only: RwSignal<bool>,
  bookmarked: RwSignal<bool>,
  /// 当前题不能收藏（计算变体）：按钮置灰并在提示里说明原因。
  #[prop(into)]
  bookmark_disabled: Signal<bool>,
  on_toggle_bookmark: Callback<()>,
  #[prop(into)] sequential: Signal<bool>,
  on_open_search: Callback<()>,
  on_open_settings: Callback<()>,
) -> impl IntoView {
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

  view! {
    <QuestionProgressHeader
      percent=percent
      left=move || {
        view! {
          <span class="text-sm text-muted-foreground">{move || t("exam.bank-class")}</span>
          <div class="flex overflow-hidden rounded-lg border">
            {Bank::ALL
              .into_iter()
              .map(|b| {
                view! {
                  <button
                    type="button"
                    class=move || toggle_class(bank.get() == b, "px-3 py-1.5 text-sm font-medium transition-colors")
                    on:click=move |_| on_switch_bank.run(b)
                  >
                    {move || bank_class(b.as_str())}
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
            {move || t("exam.new-in-this-class")}
          </button>
          <button
            type="button"
            class=move || toggle_class(unseen_only.get(), "rounded-lg border px-3 py-1.5 text-sm font-medium transition-colors")
            title=move || t("exam.exclude-questions-already-done")
            on:click=move |_| unseen_only.update(|v| *v = !*v)
          >
            {move || t("exam.unseen-only")}
          </button>
          <button
            type="button"
            class=move || toggle_class(multi_only.get(), "rounded-lg border px-3 py-1.5 text-sm font-medium transition-colors")
            title=move || t("exam.multi-answer-questions-only")
            on:click=move |_| multi_only.update(|v| *v = !*v)
          >
            {move || t("exam.multi-answer-only")}
          </button>
        }
      }
      right=move || {
        view! {
          <Button
            variant=Variant::Outline
            size=Size::Icon
            aria_label=Signal::derive(move || t("exam.bookmark"))
            disabled=bookmark_disabled
            title=Signal::derive(move || {
              if bookmark_disabled.get() {
                t("exam.variant-no-bookmark")
              } else if bookmarked.get() {
                t("exam.remove-bookmark")
              } else {
                t("exam.bookmark-this-question")
              }
            })
            on_click=Callback::new(move |_| on_toggle_bookmark.run(()))
          >
            {move || {
              if bookmarked.get() {
                view! { <Icon kind=IconKind::BookMarked class="h-4 w-4" /> }
              } else {
                view! { <Icon kind=IconKind::Bookmark class="h-4 w-4" /> }
              }
            }}
          </Button>
          {move || {
            sequential.get()
              .then(|| {
                view! {
                  <Button
                    variant=Variant::Outline
                    size=Size::Icon
                    aria_label=Signal::derive(move || t("shell.search"))
                    title=Signal::derive(move || t("shell.search"))
                    on_click=Callback::new(move |_| on_open_search.run(()))
                  >
                    <Icon kind=IconKind::Search class="h-4 w-4" />
                  </Button>
                }
              })
          }}
          <Button
            variant=Variant::Outline
            size=Size::Icon
            aria_label=Signal::derive(move || t("exam.settings"))
            title=Signal::derive(move || t("exam.settings"))
            on_click=Callback::new(move |_| on_open_settings.run(()))
          >
            <Icon kind=IconKind::Settings class="h-4 w-4" />
          </Button>
        }
      }
    />
  }
}

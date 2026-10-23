//! 练习页顶部进度头：进度条、题库类别切换、只看本类新增 / 只练没做过 / 只练多选、收藏、搜索与设置入口。

use ham_web_core::Bank;
use leptos::prelude::*;

use crate::components::common::QuestionProgressHeader;
use crate::i18n::{bank_class, t};
use crate::icons::{Icon, IconKind};
use crate::ui::{Button, Chip, ChipGroup, ChipToggle, Size, Variant};

#[component]
pub(super) fn PracticeHeaderBar(
  #[prop(into)] percent: Signal<i64>,
  /// 进度文字：`已作答 N / 总数`（比百分比更能反映小步推进）。
  #[prop(into)]
  progress_label: Signal<String>,
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
  view! {
    <QuestionProgressHeader
      percent=percent
      label=progress_label
      left=move || {
        view! {
          <span class="text-sm text-muted-foreground">{move || t("exam.bank-class")}</span>
          <ChipGroup
            value=Signal::derive(move || bank.get().as_str().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(b) = v.parse::<Bank>() {
                on_switch_bank.run(b);
              }
            })
            aria_label=Signal::derive(move || t("exam.bank-class"))
          >
            {Bank::ALL
              .into_iter()
              .map(|b| {
                let label = view! { {move || bank_class(b.as_str())} }.into_any();
                view! { <Chip value=b.as_str().to_string()>{label}</Chip> }
              })
              .collect_view()}
          </ChipGroup>
          <ChipToggle
            active=unique_only
            on_change=Callback::new(move |v: bool| unique_only.set(v))
          >
            {view! { {move || t("exam.new-in-this-class")} }.into_any()}
          </ChipToggle>
          <ChipToggle
            active=unseen_only
            on_change=Callback::new(move |v: bool| unseen_only.set(v))
            title=Signal::derive(move || t("exam.exclude-questions-already-done"))
          >
            {view! { {move || t("exam.unseen-only")} }.into_any()}
          </ChipToggle>
          <ChipToggle
            active=multi_only
            on_change=Callback::new(move |v: bool| multi_only.set(v))
            title=Signal::derive(move || t("exam.multi-answer-questions-only"))
          >
            {view! { {move || t("exam.multi-answer-only")} }.into_any()}
          </ChipToggle>
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

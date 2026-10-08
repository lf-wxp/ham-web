use ham_web_core::QuestionItem;
use leptos::prelude::*;

use crate::components::common::NoteEditor;
use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::store;
use crate::ui::{ControlSize, NativeSelect, SelectOption};

/// 一道收藏题卡片；`groups_of` 为该题当前所属分组，`all_groups` 为全部分组（用于下拉加入）。
#[component]
pub(super) fn BookmarkCard(
  question: QuestionItem,
  groups_of: Vec<String>,
  all_groups: Vec<String>,
  on_remove: Callback<QuestionItem>,
  on_add_to_group: Callback<(QuestionItem, String)>,
  on_remove_from_group: Callback<(QuestionItem, String)>,
) -> impl IntoView {
  let j = question
    .j_code()
    .map(str::to_owned)
    .unwrap_or_else(|| "—".to_owned());
  let q = question.clone();
  let q_add = question.clone();
  let pick = RwSignal::new(String::new());
  // 笔记按需展开：收藏列表可能很长，每题都铺开一个输入框会把页面撑得没法浏览。
  let q_note = question.clone();
  let note_id = Signal::derive(move || q_note.stable_id().unwrap_or_default());
  let note_open = RwSignal::new(false);
  // 已有笔记的题目标出来，省得逐条展开去找（笔记保存后展开状态会重渲染，此处够用）。
  let q_has_note = question.clone();
  let has_note = store::load_note(&q_has_note.stable_id().unwrap_or_default()).is_some();
  view! {
    <div class="rounded-xl border bg-card p-4">
      <div class="mb-2 flex items-start gap-2">
        <span class="mt-0.5 shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-xs">{j}</span>
        <p class="flex-1 text-sm font-medium leading-snug">{question.question.clone()}</p>
        <button
          type="button"
          class="shrink-0 text-muted-foreground transition-colors hover:text-destructive"
          title=move || t("exam.remove-bookmark")
          aria-label=move || t("exam.remove-bookmark")
          on:click=move |_| on_remove.run(q.clone())
        >
          <Icon kind=IconKind::BookMarked class="h-4 w-4" />
        </button>
      </div>
      <div class="space-y-1">
        {question
          .options
          .iter()
          .map(|o| {
            view! {
              <div class="text-xs text-muted-foreground">
                <span class="font-mono">{o.key.clone()}</span> "　" {o.text.clone()}
              </div>
            }
          })
          .collect_view()}
      </div>
      <div class="mt-2 text-xs text-emerald-600 dark:text-emerald-400">
        {move || t("exam.answer")} <span class="font-mono font-semibold">{question.answer_keys.join("、")}</span>
      </div>
      <div class="mt-3 flex flex-wrap items-center gap-1.5 border-t pt-2">
        <span class="text-xs text-muted-foreground">{move || t("exam.group")}</span>
        {groups_of
          .into_iter()
          .map(|name| {
            let n = name.clone();
            let qr = question.clone();
            view! {
              <button
                type="button"
                class="inline-flex items-center gap-1 rounded-full border bg-accent px-2 py-0.5 text-[11px] transition-colors hover:bg-destructive/10"
                title=move || t("exam.remove-from-this-group")
                on:click=move |_| on_remove_from_group.run((qr.clone(), n.clone()))
              >
                {name} " ×"
              </button>
            }
          })
          .collect_view()}
        {(!all_groups.is_empty()).then(|| {
          let group_options: Vec<SelectOption> = all_groups
            .iter()
            .map(|g| SelectOption::new(g.clone(), g.clone()))
            .collect();
          view! {
            <NativeSelect
              value=pick
              on_change=Callback::new(move |v: String| {
                if !v.is_empty() {
                  on_add_to_group.run((q_add.clone(), v));
                }
                pick.set(String::new());
              })
              options=group_options
              placeholder=Signal::derive(move || t("exam.add-to-group"))
              size=ControlSize::Sm
              aria_label=Signal::derive(move || t("exam.add-to-group"))
              class="h-7 w-auto rounded-full px-2 py-0 text-[11px] md:text-[11px]"
            />
          }
        })}
        <button
          type="button"
          class="ml-auto inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] text-muted-foreground transition-colors hover:bg-accent"
          aria-expanded=move || note_open.get().to_string()
          on:click=move |_| note_open.update(|v| *v = !*v)
        >
          {move || t("exam.notes")}
          {has_note.then(|| {
            view! { <span class="size-1.5 rounded-full bg-primary"></span> }
          })}
        </button>
      </div>
      {move || {
        note_open
          .get()
          .then(|| {
            view! {
              <div class="mt-3">
                <NoteEditor question_id=note_id />
              </div>
            }
          })
      }}
    </div>
  }
}

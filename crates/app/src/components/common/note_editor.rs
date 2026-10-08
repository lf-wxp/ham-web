use leptos::prelude::*;

use crate::i18n::t;
use crate::store;
use crate::ui::{Button, Size, Textarea, Variant};

/// 题目私人笔记编辑区：随当前题 `question_id` 切换自动加载对应笔记。
#[component]
pub fn NoteEditor(#[prop(into)] question_id: Signal<String>) -> impl IntoView {
  let draft = RwSignal::new(String::new());
  let saved = RwSignal::new(false);

  Effect::new(move |_| {
    draft.set(store::load_note(&question_id.get()).unwrap_or_default());
    saved.set(false);
  });

  let save = move || {
    store::save_note(&question_id.get_untracked(), draft.get_untracked().as_str());
    saved.set(true);
  };

  let clear = move || {
    draft.set(String::new());
    store::save_note(&question_id.get_untracked(), "");
    saved.set(false);
  };

  view! {
    <div class="rounded-xl border bg-card p-4">
      <div class="mb-2 flex items-center justify-between">
        <div class="text-sm font-semibold">{move || t("exam.my-notes")}</div>
        <span class="text-[11px] text-muted-foreground">
          {move || saved.get().then(|| t("exam.saved"))}
        </span>
      </div>
      <Textarea
        value=draft
        on_change=Callback::new(move |v: String| draft.set(v))
        rows=3
        placeholder=Signal::derive(move || t("exam.note-your-understanding-common"))
        aria_label=Signal::derive(move || t("common.note-content"))
      />
      <div class="mt-2 flex items-center gap-2">
        <Button variant=Variant::Default size=Size::Sm on_click=Callback::new(move |_| save())>
          {move || t("exam.save-note")}
        </Button>
        <Button
          variant=Variant::Ghost
          size=Size::Sm
          class="text-muted-foreground"
          on_click=Callback::new(move |_| clear())
        >
          {move || t("exam.clear")}
        </Button>
      </div>
    </div>
  }
}

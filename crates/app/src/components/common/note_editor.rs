use leptos::prelude::*;

use crate::i18n::t;
use crate::store;
use crate::ui::{Size, Variant, button_class};

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
        <div class="text-sm font-semibold">{move || t("我的笔记")}</div>
        <span class="text-[11px] text-muted-foreground">
          {move || saved.get().then(|| t("已保存"))}
        </span>
      </div>
      <textarea
        aria-label=move || t("笔记内容")
        prop:value=move || draft.get()
        on:input=move |e| draft.set(event_target_value(&e))
        rows=3
        placeholder=move || t("记录这道题的个人理解、易错点或口诀…")
        class="flex w-full min-w-0 rounded-md border bg-transparent px-3 py-2 text-sm shadow-xs outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-ring/50"
      ></textarea>
      <div class="mt-2 flex items-center gap-2">
        <button type="button" class=button_class(Variant::Default, Size::Sm, "") on:click=move |_| save()>
          {move || t("保存笔记")}
        </button>
        <button type="button" class=button_class(Variant::Ghost, Size::Sm, "text-muted-foreground") on:click=move |_| clear()>
          {move || t("清除")}
        </button>
      </div>
    </div>
  }
}

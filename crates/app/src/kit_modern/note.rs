//! 现代风「我的笔记」：一块圆角卡 + 无边框文本域 + 主次两个按钮。
//! 存取逻辑与像素版完全一致（`store::load_note` / `save_note`），只换外观。

use leptos::prelude::*;

use crate::i18n::t;
use crate::store;

#[component]
pub fn ModernNote(#[prop(into)] question_id: Signal<String>) -> impl IntoView {
  let draft = RwSignal::new(String::new());
  // 只在换题时写进 textarea（`prop:value` 由它驱动）：直接绑 `draft` 的话，
  // 每次输入都会把值写回控件，光标会被顶到末尾。
  let initial = RwSignal::new(String::new());
  let saved = RwSignal::new(false);

  Effect::new(move |_| {
    let text = store::load_note(&question_id.get()).unwrap_or_default();
    initial.set(text.clone());
    draft.set(text);
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
    <div class="md-x">
      <div class="md-x-head">
        <span class="md-h2">{move || t("exam.my-notes")}</span>
        <span class="md-muted md-sm">{move || saved.get().then(|| t("exam.saved"))}</span>
      </div>
      <div class="md-x-body-plain">
        <super::ModernTextarea
          value=initial
          on_input=Callback::new(move |v: String| draft.set(v))
          rows="3"
          aria_label=Signal::derive(move || t("common.note-content"))
          placeholder=Signal::derive(move || t("exam.note-your-understanding-common"))
        />
        <div class="mt-3 flex items-center gap-2">
          <button type="button" class="md-btn md-btn-accent" on:click=move |_| save()>
            {move || t("exam.save-note")}
          </button>
          <button type="button" class="md-btn md-btn-ghost" on:click=move |_| clear()>
            {move || t("exam.clear")}
          </button>
        </div>
      </div>
    </div>
  }
}

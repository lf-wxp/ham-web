//! `notes_view`：从 `mod.rs` 拆出的视图构造函数（一个组件一个文件）。

use ham_web_core::bands::Note;
use leptos::prelude::*;

use crate::i18n::tf;

/// 备注单元格：多条备注之间用「、」分隔（如「5.162A、CHN4、CHN8、6米业余波段」）。
pub(super) fn notes_view(notes: &'static [Note], jump: Callback<&'static str>) -> impl IntoView {
  notes
    .iter()
    .enumerate()
    .map(|(i, note)| {
      let sep = (i > 0).then(|| view! { <span class="text-muted-foreground/60">"、"</span> });
      let body = match *note {
        Note::Text(t) => view! { <span>{t}</span> }.into_any(),
        Note::Ref(code) => view! {
          <button
            type="button"
            class="font-mono font-medium text-blue-600 underline underline-offset-2 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
            title=tf("common.view-footnote", &[(code)])
            on:click=move |_| jump.run(code)
          >
            {code}
          </button>
        }
        .into_any(),
      };
      view! { {sep} {body} }
    })
    .collect_view()
}

//! 测验选项按钮：题库小测 / 听音辨字里的选项，点击即判分。
//!
//! 八个题库的选项原来各写一份 `rounded-lg border px-3 py-2.5 … hover:bg-accent`，
//! 只有对齐（`text-left` / `text-center`）与字体（`font-mono`）不同 —— 这些差异由
//! 调用点经 `class` 覆盖，底座只留这一份。

use leptos::prelude::*;

use crate::cn::cn;

/// 测验选项按钮。
#[component]
pub fn QuizOption(
  /// 追加类名（对齐 / 字体 / 内边距）。
  #[prop(optional, into)]
  class: String,
  /// 判分后禁用（避免重复点击改分）。
  #[prop(optional, into)]
  disabled: Signal<bool>,
  #[prop(optional, into)] on_click: Option<Callback<()>>,
  children: Children,
) -> impl IntoView {
  let class = cn(&[
    "border px-3 py-2.5 text-sm transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60",
    &class,
  ]);
  view! {
    <button
      type="button"
      data-slot="quiz-option"
      class=class
      disabled=move || disabled.get()
      on:click=move |_| {
        if let Some(cb) = on_click {
          cb.run(());
        }
      }
    >
      {children()}
    </button>
  }
}

//! 评测里「测试条件」的一行：小号标签 + 条件值（知识库正文，走 `kt` 翻译）。

use leptos::prelude::*;

use crate::data;

#[component]
pub(super) fn ConditionRow(
  /// 标签（界面文案，切换语言时随之刷新）。
  #[prop(into)]
  label: Signal<String>,
  /// 条件值（知识库中文原文，经 `kt` 查译文）。
  value: &'static str,
) -> impl IntoView {
  view! {
    <div class="flex flex-col gap-0.5 rounded-lg px-3 py-1.5">
      <span class="text-[11px] text-muted-foreground">{move || label.get()}</span>
      // 条件值走知识库译文：先订阅加载状态，切语言后这一格才跟着重算。
      <span class="text-sm">{move || {
        data::track_knowledge();
        data::kt(value)
      }}</span>
    </div>
  }
}

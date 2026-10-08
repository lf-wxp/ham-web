//! `task_row`：从 `study_plan_card.rs` 拆出的视图构造函数（一个组件一个文件）。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{ButtonLink, Size, Variant};

/// 一条今日任务。
pub(super) fn task_row(
  done: bool,
  label: String,
  detail: String,
  href: String,
  action: String,
) -> impl IntoView {
  view! {
    <li class="flex items-center gap-3 py-2">
      <span
        aria-hidden="true"
        class=if done {
          "flex size-5 shrink-0 items-center justify-center rounded-full bg-emerald-500 text-xs text-white"
        } else {
          "size-5 shrink-0 rounded-full border-2 border-muted-foreground/40"
        }
      >
        {done.then_some("✓")}
      </span>
      <div class="min-w-0 flex-1">
        <div class=if done { "text-sm text-muted-foreground line-through" } else { "text-sm font-medium" }>
          {label}
          {done.then(|| view! { <span class="sr-only">{move || t("common.done")}</span> })}
        </div>
        <div class="text-xs text-muted-foreground">{detail}</div>
      </div>
      {(!done).then(|| view! {
        <ButtonLink
          href=href
          variant=Variant::Outline
          size=Size::Sm
          class="shrink-0"
        >{action}</ButtonLink>
      })}
    </li>
  }
}

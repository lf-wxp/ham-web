//! 侧栏筛选列表项：整行宽度的互斥选择（术语分类、题库分类树）。
//!
//! 与 [`Chip`](crate::ui::Chip) 的区别是版式：chip 是工具条上的横向胶囊，这里是侧栏里
//! 「整行宽 + 左侧色点 + 右侧计数」的列表项 —— 两侧的内容由调用点用 `children` 给出。

use leptos::prelude::*;

use crate::cn::cn;

/// 侧栏列表项（整行宽度的单选）。
#[component]
pub fn SidebarItem(
  #[prop(into)] active: Signal<bool>,
  /// 追加类名（缩进、内边距等）。
  #[prop(optional, into)]
  class: String,
  #[prop(optional, into)] on_click: Option<Callback<()>>,
  children: Children,
) -> impl IntoView {
  let class = Signal::derive(move || {
    cn(&[
      "flex w-full items-center gap-2 px-3 py-2 text-sm font-medium transition-colors",
      &class,
      if active.get() {
        "bg-primary text-primary-foreground"
      } else {
        "hover:bg-accent"
      },
    ])
  });
  view! {
    <button
      type="button"
      data-slot="sidebar-item"
      aria-pressed=move || active.get().to_string()
      class=class
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

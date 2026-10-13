//! 呼号抄收练习的操作药丸：一排「重播 / 跳过 / 结束本轮 / 显示答案」。
//!
//! 原先四份逐字相同的 `<button class="… rounded-full …">`，只有图标与文案不同 ——
//! 收成一个组件（顺带去掉了 `active:scale-95`：像素风不做缩放动画，见
//! `docs/ui-components.md` §2）。

use leptos::prelude::*;

/// 操作药丸按钮。
#[component]
pub fn PillButton(
  #[prop(optional, into)] on_click: Option<Callback<()>>,
  children: Children,
) -> impl IntoView {
  view! {
    <button
      type="button"
      data-slot="morse-pill"
      class="inline-flex items-center gap-2 rounded-full border bg-card px-4 py-1.5 text-sm transition-colors hover:bg-accent hover:text-accent-foreground"
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

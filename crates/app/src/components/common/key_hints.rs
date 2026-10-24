use leptos::prelude::*;

use crate::i18n::t;

/// 快捷键提示条：`← →` 切题、`1–4` 选 A–D、`?` 看全部快捷键。
///
/// 为什么要有：快捷键以前只在 `?` 弹窗和首次进入时的说明里出现，是「隐藏知识」；而且数字键
/// （`1–9`）与选项字母（`A–D`）对不上号，用户得自己猜 `1` 是不是 `A`。这里在底栏常驻一行，
/// 并点明对应关系。
///
/// 只在有悬停能力的设备（鼠标 / 触控板）上显示：触屏没有键盘，这行字只是噪声，
/// 还会把手机底栏顶高。
#[component]
pub fn KeyHints() -> impl IntoView {
  let kbd = "pxl-label border-2 border-ink bg-secondary px-1 text-xs text-secondary-foreground";
  view! {
    <div class="hidden flex-wrap items-center justify-center gap-x-4 gap-y-1 text-xs [@media(hover:hover)]:flex">
      <span class="inline-flex items-center gap-1">
        <kbd class=kbd>"←"</kbd>
        <kbd class=kbd>"→"</kbd>
        {move || t("exam.key-hint-nav")}
      </span>
      <span class="inline-flex items-center gap-1">
        <kbd class=kbd>"1–4"</kbd>
        {move || t("exam.key-hint-pick")}
      </span>
      <span class="inline-flex items-center gap-1">
        <kbd class=kbd>"?"</kbd>
        {move || t("exam.key-hint-help")}
      </span>
    </div>
  }
}

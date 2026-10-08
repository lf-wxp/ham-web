//! `row`：从 `eqsl.rs` 拆出的视图构造函数（一个组件一个文件）。

use leptos::prelude::*;

/// 对照表的一行：字段 / 值 / 值是从哪来的。
pub(super) fn row(label: String, value: String, source: String) -> impl IntoView {
  view! {
    <div class="flex flex-wrap items-baseline gap-x-3 border-b py-2 last:border-b-0">
      <dt class="w-20 shrink-0 text-xs text-muted-foreground">{label}</dt>
      <dd class="font-mono text-sm font-medium">{value}</dd>
      // 来源列用第二个 `<dd>` 而不是 `<span>`：`<dl>` 里除「div 包一组 dt/dd」之外
      // 不能放别的元素，原来的写法既过不了 HTML 校验，读屏也可能整列漏读。
      <dd class="ml-auto text-xs text-muted-foreground">{source}</dd>
    </div>
  }
}

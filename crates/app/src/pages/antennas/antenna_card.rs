use ham_web_core::antennas::AntennaType;
use leptos::prelude::*;

use crate::data;

#[component]
pub(super) fn AntennaCard(entry: &'static AntennaType) -> impl IntoView {
  view! {
    <article class="flex flex-col rounded-xl border bg-card p-4">
      <svg
        viewBox="0 0 100 60"
        class="h-16 w-full text-primary"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        inner_html=entry.svg
      ></svg>
      <div class="mt-3 flex items-baseline gap-2">
        <h2 class="font-semibold">{move || text(entry.name)}</h2>
        // 缩写（DP / YAGI / EFHW）是国际通用写法，不翻译
        <span class="font-mono text-xs font-semibold text-muted-foreground">{entry.abbr}</span>
      </div>
      <div class="mt-1.5 flex flex-wrap gap-1.5 text-xs">
        <span class="rounded-md bg-muted px-2 py-0.5 text-muted-foreground">
          {move || text(entry.gain)}
        </span>
        <span class="rounded-md bg-muted px-2 py-0.5 text-muted-foreground">
          {move || text(entry.usage)}
        </span>
      </div>
      <p class="mt-2 text-sm leading-6 text-muted-foreground">{move || text(entry.desc)}</p>
    </article>
  }
}

/// 卡片文案来自 `crates/core`，因此查知识库词典而不是界面词典 —— 与
/// [`ConceptsSection`](crate::components::common::ConceptsSection) 同一条通道：
/// 尚未翻译时回退中文，译好后无需改这里的代码。
fn text(zh: &'static str) -> String {
  data::track_knowledge();
  data::kt(zh)
}

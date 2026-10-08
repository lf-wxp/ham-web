//! 网格地图统计概览徽章：field / square / 网格 / DXCC / QSL 计数 + 导出 SVG / 复制摘要。

use leptos::prelude::*;

use super::state::GridMapState;
use crate::i18n::t;

#[component]
pub(super) fn GridStatsBar(state: GridMapState) -> impl IntoView {
  view! {
    // 统计概览徽章 + 复制摘要
    <div class="mb-2 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
      <span class="rounded-full border bg-muted/40 px-2 py-0.5">
        "field " <span class="font-semibold tabular-nums">{move || state.field_grids.get().len()}</span>
      </span>
      <span class="rounded-full border bg-muted/40 px-2 py-0.5">
        "square " <span class="font-semibold tabular-nums">{move || state.square_entries.get().len()}</span>
      </span>
      <span class="rounded-full border bg-muted/40 px-2 py-0.5">
        {move || t("log.grid")} " " <span class="font-semibold tabular-nums">{move || state.grid_count.get()}</span>
      </span>
      <span class="rounded-full border bg-muted/40 px-2 py-0.5">
        "DXCC " <span class="font-semibold tabular-nums">{move || state.dxcc_count.get()}</span>
      </span>
      <span class="rounded-full border bg-muted/40 px-2 py-0.5">
        "QSL "
        <span class="font-semibold tabular-nums text-emerald-600 dark:text-emerald-400">
          {move || state.qsl_confirmed_count.get()}
        </span>
      </span>
      <div class="ml-auto flex items-center gap-1">
        <button
          type="button"
          class="rounded-md border px-2 py-0.5 text-xs transition-colors hover:bg-accent"
          on:click=move |_| state.export_svg()
        >
          {move || t("log.export-svg")}
        </button>
        <button
          type="button"
          class="rounded-md border px-2 py-0.5 text-xs transition-colors hover:bg-accent"
          on:click=move |_| state.copy_summary()
        >
          {move || t("log.copy-summary")}
        </button>
      </div>
    </div>
  }
}

//! 数据与导航组件：表格全族、排序 / 密度、空表、分页、结果计数与分批加载。

use leptos::prelude::*;

use crate::components::common::{LoadMore, ResultCount, SectionCard};
use crate::i18n::t;
use crate::ui::{
  Pagination, Separator, SortOrder, Table, TableBody, TableCell, TableDensity, TableEmpty,
  TableHead, TableHeader, TableRow,
};

const ROWS: &[(&str, &str, &str)] = &[
  ("20m", "14.000", "CW"),
  ("40m", "7.000", "SSB"),
  ("2m", "144.000", "FM"),
];

const TOTAL: usize = 24;

#[component]
pub fn DataGallery() -> impl IntoView {
  let page = RwSignal::new(1_u32);
  let visible = RwSignal::new(6_usize);
  let count = Signal::derive(move || visible.get().min(TOTAL));
  let shown = Memo::new(move |_| (0..visible.get().min(TOTAL)).collect::<Vec<usize>>());

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-data"))>
      <div class="space-y-4 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.comp-data-hint")}
        </p>

        <div class="grid gap-3 lg:grid-cols-2">
          <div class="space-y-2 min-w-0">
            <code class="block text-xs text-muted-foreground">"TableDensity::Standard"</code>
            <Table
              aria_label=Signal::derive(move || t("design.comp-table-title"))
              class="min-w-[640px]"
            >
              <TableHead>
                <TableRow>
                  <TableHeader sort=SortOrder::Ascending>
                    {move || t("design.comp-table-band")}
                  </TableHeader>
                  <TableHeader>{move || t("design.comp-table-freq")}</TableHeader>
                  <TableHeader>{move || t("design.comp-table-mode")}</TableHeader>
                </TableRow>
              </TableHead>
              <TableBody>
                {ROWS
                  .iter()
                  .map(|(band, freq, mode)| {
                    view! {
                      <TableRow>
                        <TableHeader scope="row">{*band}</TableHeader>
                        <TableCell numeric=true>{*freq}</TableCell>
                        <TableCell>{*mode}</TableCell>
                      </TableRow>
                    }
                  })
                  .collect_view()}
              </TableBody>
            </Table>
          </div>
          <div class="space-y-2 min-w-0">
            <code class="block text-xs text-muted-foreground">"TableDensity::Compact"</code>
            <Table
              aria_label=Signal::derive(move || t("design.comp-table-compact"))
              density=TableDensity::Compact
              class="min-w-[640px]"
            >
              <TableHead>
                <TableRow>
                  <TableHeader>{move || t("design.comp-table-band")}</TableHeader>
                  <TableHeader>{move || t("design.comp-table-freq")}</TableHeader>
                  <TableHeader>{move || t("design.comp-table-mode")}</TableHeader>
                </TableRow>
              </TableHead>
              <TableBody>
                <TableEmpty
                  cols=3
                  label=Signal::derive(move || t("design.comp-table-empty"))
                />
              </TableBody>
            </Table>
          </div>
        </div>

        <Separator />

        <div class=crate::kit::tile_class("space-y-3 p-3")>
          <div class="flex flex-wrap items-center justify-between gap-2">
            <code class="text-xs text-muted-foreground">"Pagination"</code>
            <span class=crate::kit::stat_text_class("text-xs")>
              {move || format!("{} / 8", page.get())}
            </span>
          </div>
          <Pagination
            page=page
            total_pages=Signal::derive(|| 8_u32)
            on_change=Callback::new(move |p: u32| page.set(p))
            aria_label=Signal::derive(move || t("design.comp-pagination"))
          />
        </div>

        <div class=crate::kit::tile_class("space-y-3 p-3")>
          <div class="flex flex-wrap items-center justify-between gap-2">
            <code class="text-xs text-muted-foreground">"ResultCount / LoadMore"</code>
            <ResultCount
              count=count
              class="text-xs text-muted-foreground"
            />
          </div>
          <div class="grid grid-cols-4 gap-1 sm:grid-cols-8">
            <For each=move || shown.get() key=|i| *i let:i>
              <code class=crate::kit::tag_class("text-center text-xs")>
                {move || format!("{:02}", i + 1)}
              </code>
            </For>
          </div>
          <LoadMore
            visible=visible
            total=Signal::derive(|| TOTAL)
            step=6
            class="w-full"
          />
        </div>
      </div>
    </SectionCard>
  }
}

//! 「应避免占用的频段」速查表。
//!
//! 与 `ModeWindowsTable` 互补：那张表说某个模式能落在哪一段，这张表说哪些段要让开
//! （卫星业余业务专用段 + 本地联络要避开的弱信号段），原先同样只散落在题库解析里。

use ham_web_core::bandplan::AVOID_RANGES;
use leptos::prelude::*;

use crate::data;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};

/// 表头词条（界面词典）。
const HEADERS: &[&str] = &[
  "knowledge.band",
  "knowledge.avoid-range",
  "knowledge.reason",
];

#[component]
pub(super) fn AvoidRangesTable() -> impl IntoView {
  view! {
    <section class="pxl-card border bg-card">
      <h2 class="pxl-section-title">{move || t("knowledge.avoid-ranges")}</h2>
      <Table
        aria_label=Signal::derive(move || t("knowledge.avoid-ranges"))
        class="min-w-[560px]"
      >
        <TableHead>
          <TableRow>
            {HEADERS
              .iter()
              .map(|h| view! { <TableHeader>{move || t(h)}</TableHeader> })
              .collect_view()}
          </TableRow>
        </TableHead>
        <TableBody>
          {AVOID_RANGES
            .iter()
            .map(|a| {
              view! {
                <TableRow>
                  <TableCell class="align-top whitespace-nowrap font-mono">{a.band}</TableCell>
                  <TableCell class="align-top whitespace-nowrap font-mono text-primary">
                    {a.range}
                  </TableCell>
                  <TableCell class="align-top text-muted-foreground">
                    {move || data::kt_tracked(a.note)}
                  </TableCell>
                </TableRow>
              }
            })
            .collect_view()}
        </TableBody>
      </Table>
    </section>
  }
}

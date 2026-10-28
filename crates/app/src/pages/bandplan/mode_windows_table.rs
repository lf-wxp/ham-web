//! 「按调制方式的可实际占用频率」速查表。
//!
//! 逐波段的规划卡片只划到「这一段允许话音」，而 `29.510` 这类
//! 已扣掉信标保护带、CW / 数字段与专用 FM 段的边界只存在于题库：这张表把它单独列出来。

use ham_web_core::bandplan::MODE_WINDOWS;
use leptos::prelude::*;

use crate::data;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};

/// 表头词条（界面词典）。
const HEADERS: &[&str] = &[
  "knowledge.band",
  "knowledge.modulation-mode",
  "knowledge.usable-range",
  "knowledge.note",
];

#[component]
pub(super) fn ModeWindowsTable() -> impl IntoView {
  view! {
    <section class="pxl-card border bg-card">
      <h2 class="pxl-section-title">{move || t("knowledge.mode-windows")}</h2>
      <Table
        aria_label=Signal::derive(move || t("knowledge.mode-windows"))
        class="min-w-[720px]"
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
          {MODE_WINDOWS
            .iter()
            .map(|w| {
              view! {
                <TableRow>
                  <TableCell class="align-top whitespace-nowrap font-mono">{w.band}</TableCell>
                  <TableCell class="align-top whitespace-nowrap">{w.mode}</TableCell>
                  <TableCell class="align-top whitespace-nowrap font-mono text-primary">
                    {move || data::kt_tracked(w.window)}
                  </TableCell>
                  <TableCell class="align-top text-muted-foreground">
                    {move || data::kt_tracked(w.note)}
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

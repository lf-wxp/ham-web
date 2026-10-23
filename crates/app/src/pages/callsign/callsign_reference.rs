//! 呼号速查：结构、台站类别、分区、斜杠后缀与规则要点。

use ham_web_core::callsign::{
  CALL_AREA_TABLE, CALLSIGN_RULES, CALLSIGN_STRUCTURE, SLASH_SUFFIXES, STATION_TYPES,
};
use leptos::prelude::*;

use crate::components::common::{BulletSection, TableSection};
use crate::i18n::t;

const STRUCTURE_HEADERS: &[&str] = &["radio.part", "radio.rule", "knowledge.example"];
const STATION_HEADERS: &[&str] = &["morse.letters", "exam.class-column", "radio.notes"];
const SLASH_HEADERS: &[&str] = &["radio.suffix", "knowledge.meaning", "knowledge.example"];
const AREA_HEADERS: &[&str] = &[
  "radio.zone-label",
  "radio.regions",
  "radio.callsign-example",
];

#[component]
pub fn CallsignReference() -> impl IntoView {
  view! {
    // `TableSection` 内部会对 title / headers 再走一次 `t()`，这里必须传词条 key 而不是译文。
    <TableSection
      title="radio.callsign-structure"
      headers=STRUCTURE_HEADERS
      rows=CALLSIGN_STRUCTURE
    />
    <TableSection
      title="radio.chinese-station-class-second"
      headers=STATION_HEADERS
      rows=STATION_TYPES
    />
    <TableSection
      title="radio.call-area-table"
      headers=AREA_HEADERS
      rows=CALL_AREA_TABLE
    />
    <TableSection
      title="radio.slash-suffix"
      headers=SLASH_HEADERS
      rows=SLASH_SUFFIXES
    />
    <BulletSection title="radio.callsign-rules" items=CALLSIGN_RULES />

    <section class="pxl-card border bg-card p-4 text-sm text-muted-foreground">
      {move || t("radio.see-prefix-page-for")}
      <a
        href="/prefixes"
        class="ml-1 font-medium text-primary underline underline-offset-2"
      >
        {move || t("shell.callsign-prefixes")}
      </a>
    </section>
  }
}

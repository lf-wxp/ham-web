//! DX 奖状体系速查。

use ham_web_core::awards::AWARDS;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::data::kt_tracked;
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

#[component]
pub fn AwardsPage() -> impl IntoView {
  set_title("shell.dx-awards-title");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.dx-awards-title")
        subtitle=move || t("log.dxcc-waz-all-cq")
      />

      <PageContainer>
        <section class=crate::kit::card_class()>
          <h2 class=crate::kit::card_title_class()>{move || t("log.major-international-awards")}</h2>
          <Table class="min-w-[720px]" density=crate::ui::TableDensity::Compact>
            <TableHead>
              <TableRow>
                <TableHeader class=crate::kit::table_cell_class("")>{move || t("log.abbreviation")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("")>{move || t("common.full-name")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("")>{move || t("log.organisation")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("")>{move || t("log.basic-rules")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {AWARDS
                .iter()
                .map(|&(abbr, full, org, rule)| {
                  view! {
                    <TableRow>
                      <TableCell class=crate::kit::table_cell_class("whitespace-nowrap font-mono text-primary")>
                        {move || kt_tracked(abbr)}
                      </TableCell>
                      <TableCell class=crate::kit::table_cell_class("whitespace-nowrap")>{move || kt_tracked(full)}</TableCell>
                      <TableCell class=crate::kit::table_cell_class("whitespace-nowrap text-muted-foreground")>{move || kt_tracked(org)}</TableCell>
                      <TableCell class=crate::kit::table_cell_class("text-muted-foreground")>{move || kt_tracked(rule)}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>
      </PageContainer>
    </div>
  }
}

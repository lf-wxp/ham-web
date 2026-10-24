//! 考试速查页：操作证权限、分区号、RST 信号报告、发射类别标识、通联英语短句。

use ham_web_core::reference::{
  CALL_AREAS, EMISSION_DESIGNATION, EMISSION_TYPES, KNOWLEDGE_SOURCES, LICENSE_CLASSES, PHRASES,
  RST_SCALES,
};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, TableSection};
use crate::i18n::t;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

#[component]
pub fn ReferencePage() -> impl IntoView {
  set_title("shell.exam-reference-title");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader title=t("shell.exam-reference-title") subtitle=t("exam.licence-privileges-zones-rst") />
      <PageContainer>
        // 操作证类别与权限
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("exam.operator-licence-classes-and")}</h2>
          <Table
            aria_label=Signal::derive(move || t("exam.operator-licence-classes-and"))
            class="min-w-[560px]"
          >
            <TableHead>
              <TableRow>
                <TableHeader>{move || t("exam.class-column")}</TableHeader>
                <TableHeader>{move || t("exam.frequency-range")}</TableHeader>
                <TableHeader>{move || t("exam.power-limit")}</TableHeader>
                <TableHeader>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {LICENSE_CLASSES
                .iter()
                .map(|c| {
                  view! {
                    <TableRow>
                      <TableCell class="align-top font-mono">
                        {c.class} " 类"
                      </TableCell>
                      <TableCell class="align-top">{c.freq}</TableCell>
                      <TableCell class="align-top">{c.power}</TableCell>
                      <TableCell class="align-top text-muted-foreground">{c.note}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        // 分区号
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("exam.amateur-radio-zone-numbers")}</h2>
          <div class="grid grid-cols-2 gap-2 p-4 sm:grid-cols-3 lg:grid-cols-5">
            {CALL_AREAS
              .iter()
              .map(|a| {
                view! {
                  <div class="flex items-start gap-2 border bg-inset p-3">
                    <span class="font-mono text-lg text-primary">{a.digit}</span>
                    <span class="text-xs leading-5 text-muted-foreground">{a.regions}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        // RST 信号报告
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("shell.rst-report")}</h2>
          <div class="grid grid-cols-1 gap-3 p-4 md:grid-cols-3">
            {RST_SCALES
              .iter()
              .map(|s| {
                view! {
                  <div class="border">
                    <div class="border-b bg-inset px-3 py-2">
                      <span class="font-mono text-primary">{s.key}</span>
                      <span class="ml-2 text-sm">{s.name}</span>
                      <div class="text-xs text-muted-foreground">{s.note}</div>
                    </div>
                    <dl class="divide-y text-sm">
                      {s
                        .levels
                        .iter()
                        .map(|&(num, desc)| {
                          view! {
                            <div class="flex gap-2 px-3 py-1.5">
                              <dt class="font-mono tabular-nums">{num}</dt>
                              <dd class="text-muted-foreground">{desc}</dd>
                            </div>
                          }
                        })
                        .collect_view()}
                    </dl>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        // 发射类别
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("exam.emission-designators")}</h2>
          <Table
            aria_label=Signal::derive(move || t("exam.emission-designators"))
            class="min-w-[480px]"
          >
            <TableHead>
              <TableRow>
                <TableHeader>{move || t("exam.designator")}</TableHeader>
                <TableHeader>{move || t("exam.name")}</TableHeader>
                <TableHeader>{move || t("radio.notes")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {EMISSION_TYPES
                .iter()
                .map(|e| {
                  view! {
                    <TableRow>
                      <TableCell class="align-top font-mono">{e.code}</TableCell>
                      <TableCell class="align-top">{e.name}</TableCell>
                      <TableCell class="align-top text-muted-foreground">{e.desc}</TableCell>
                    </TableRow>
                  }
                })
                .collect_view()}
            </TableBody>
          </Table>
        </section>

        // 发射类别的构成
        <TableSection
          title="发射类别标识的构成"
          headers=&["组成", "含义", "示例"]
          rows=EMISSION_DESIGNATION
          class="min-w-[720px]"
        />

        // 通联英语
        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("exam.contact-english-phrases")}</h2>
          <dl class="divide-y">
            {PHRASES
              .iter()
              .map(|p| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[7rem_1fr]">
                    <dt class="text-xs text-muted-foreground">{p.usage}</dt>
                    <dd>
                      <div class="font-mono text-sm text-foreground">{p.en}</div>
                      <div class="mt-0.5 text-sm text-muted-foreground">{p.zh}</div>
                    </dd>
                  </div>
                }
              })
              .collect_view()}
          </dl>
        </section>
        // 知识来源与时效
        <TableSection
          title="知识来源与时效"
          headers=&["领域", "来源", "说明"]
          rows=KNOWLEDGE_SOURCES
          class="min-w-[720px]"
        />
      </PageContainer>
    </div>
  }
}

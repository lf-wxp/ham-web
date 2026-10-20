//! A/B/C 类操作证权限对比。

use ham_web_core::license_classes::{BAND_PERMISSIONS, CLASS_TIPS, CLASS_USAGE};
use ham_web_core::reference::LICENSE_CLASSES;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, ProvenanceNote};
use crate::i18n::{t, tf};
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 align-top";

#[component]
pub fn LicenseClassesPage() -> impl IntoView {
  set_title("shell.a-b-c-license");
  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.a-b-c-license")
        subtitle=move || t("radio.frequency-range-power-limit")
      />

      <PageContainer>
        <Table class="min-w-[640px] text-sm" wrapper_class="pxl-card border bg-card">
          <TableHead>
            <TableRow>
              <TableHeader class=CELL>{move || t("exam.class-column")}</TableHeader>
              <TableHeader class=CELL>{move || t("exam.frequency-range")}</TableHeader>
              <TableHeader class=CELL>{move || t("exam.power-limit")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {LICENSE_CLASSES
              .iter()
              .map(|c| {
                view! {
                  <TableRow>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-medium")>
                      {tf("learning.class", &[(c.class)])}
                    </TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{c.freq}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{c.power}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{c.note}</TableCell>
                  </TableRow>
                }
              })
              .collect_view()}
          </TableBody>
        </Table>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.typical-equipment-and-scenarios")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {CLASS_USAGE
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex flex-col gap-1 px-3 py-2">
                    <span class="text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.band-privileges-quick-reference")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {BAND_PERMISSIONS
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 px-3 py-2">
                    <span class="text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.exam-tips")}</h2>
          <ul class="space-y-2 p-4">
            {CLASS_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{*tip}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>
        <ProvenanceNote domain="license-classes" />
      </PageContainer>
    </div>
  }
}

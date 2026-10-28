use ham_web_core::satellites::{SATELLITE_TIPS, SATELLITES, TRACKING_SOFTWARE};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader, ProvenanceNote};
use crate::components::rotor_control::{Aim, RotorControl};
use crate::data::kt_tracked;
use crate::ui::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::util::set_title;

use super::CELL;
use super::doppler_tracker::DopplerTracker;
use super::iss_tracker::IssTracker;
use super::pass_predictor::PassPredictor;
use crate::i18n::t;

#[component]
pub fn SatellitesPage() -> impl IntoView {
  set_title("shell.satellites");
  // 本页渲染 `crates/core` 的中文原文，译文词典要自己挂一次拉取（本页不是 `KnowledgePage`）。
  crate::data::use_knowledge_i18n();
  // 多普勒面板此刻要对准的方向，旋转器控制条据此自动跟踪（默认关闭，需手动勾选）。
  let aim = RwSignal::new(None::<Aim>);
  view! {
    <div class="min-h-svh animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.satellites")
        subtitle=move || t("radio.fm-repeaters-and-linear")
      />

      <PageContainer>
        <PassPredictor />

        <DopplerTracker aim=aim />

        <IssTracker />

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.antenna-rotator")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("radio.during-a-pass-connect")}
          </p>
          <div class="p-4"><RotorControl aim=aim /></div>
        </section>

        // 横向可滚区域必须可聚焦，否则键盘用户滚不动它（axe: scrollable-region-focusable）
        <Table
          class="min-w-[680px] text-sm"
          wrapper_class="pxl-card border bg-card"
          tabindex="0"
          role="region"
          aria_label=Signal::derive(move || t("radio.satellite-frequency-table"))
        >
          <TableHead>
            <TableRow>
              <TableHeader class=CELL>{move || t("radio.satellite")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.type")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.uplink")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.downlink")}</TableHeader>
              <TableHeader class=CELL>{move || t("radio.notes")}</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {SATELLITES
              .iter()
              .map(|s| {
                view! {
                  <TableRow>
                    <TableCell class=format!("{CELL} whitespace-nowrap")>
                      <div>{move || kt_tracked(s.name)}</div>
                      <div class="font-mono text-xs text-muted-foreground">{move || kt_tracked(s.callsign)}</div>
                    </TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{move || kt_tracked(s.kind)}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{move || kt_tracked(s.uplink)}</TableCell>
                    <TableCell class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{move || kt_tracked(s.downlink)}</TableCell>
                    <TableCell class=format!("{CELL} text-muted-foreground")>{move || kt_tracked(s.note)}</TableCell>
                  </TableRow>
                }
              })
              .collect_view()}
          </TableBody>
        </Table>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.operating-tips")}</h2>
          <ul class="space-y-2 p-4">
            {SATELLITE_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{move || kt_tracked(tip)}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("radio.tracking-and-prediction-software")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {TRACKING_SOFTWARE
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 px-3 py-2">
                    <span class="shrink-0 text-sm">{move || kt_tracked(t)}</span>
                    <span class="text-sm text-muted-foreground">{move || kt_tracked(d)}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
        <ProvenanceNote domain="satellites" />
      </PageContainer>
    </div>
  }
}

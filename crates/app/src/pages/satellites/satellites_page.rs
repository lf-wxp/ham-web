use ham_web_core::satellites::{SATELLITE_TIPS, SATELLITES, TRACKING_SOFTWARE};
use leptos::prelude::*;

use crate::components::rotor_control::RotorControl;
use crate::util::set_title;

use super::CELL;
use super::iss_tracker::IssTracker;
use super::pass_predictor::PassPredictor;
use crate::i18n::t;

#[component]
pub fn SatellitesPage() -> impl IntoView {
  set_title(&t("业余卫星"));
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("业余卫星")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("FM 中继与线性转发器 · 上行 / 下行频率 · 过境预报")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <PassPredictor />

        <IssTracker />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("天线旋转器")}</h2>
          <p class="px-4 pt-3 text-xs text-muted-foreground">
            {move || t("过境时连接旋转器（GS-232 协议），输入方位角遥控天线对准卫星。")}
          </p>
          <div class="p-4"><RotorControl /></div>
        </section>

        <div class="overflow-x-auto rounded-xl border bg-card">
          <table class="w-full min-w-[680px] border-collapse text-sm">
            <thead class="bg-muted/60 text-xs">
              <tr>
                <th class=CELL>{move || t("卫星")}</th>
                <th class=CELL>{move || t("类型")}</th>
                <th class=CELL>{move || t("上行")}</th>
                <th class=CELL>{move || t("下行")}</th>
                <th class=CELL>{move || t("说明")}</th>
              </tr>
            </thead>
            <tbody>
              {SATELLITES
                .iter()
                .map(|s| {
                  view! {
                    <tr class="border-t transition-colors hover:bg-muted/40">
                      <td class=format!("{CELL} whitespace-nowrap")>
                        <div class="font-medium">{s.name}</div>
                        <div class="font-mono text-xs text-muted-foreground">{s.callsign}</div>
                      </td>
                      <td class=format!("{CELL} whitespace-nowrap text-muted-foreground")>{s.kind}</td>
                      <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{s.uplink}</td>
                      <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{s.downlink}</td>
                      <td class=format!("{CELL} text-muted-foreground")>{s.note}</td>
                    </tr>
                  }
                })
                .collect_view()}
            </tbody>
          </table>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("操作要点")}</h2>
          <ul class="space-y-2 p-4">
            {SATELLITE_TIPS
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

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("追踪与预报软件")}</h2>
          <div class="grid gap-1 p-4 sm:grid-cols-2">
            {TRACKING_SOFTWARE
              .iter()
              .map(|&(t, d)| {
                view! {
                  <div class="flex items-baseline gap-2 rounded-lg px-3 py-2">
                    <span class="shrink-0 text-sm font-medium">{t}</span>
                    <span class="text-sm text-muted-foreground">{d}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>
      </div>
    </div>
  }
}

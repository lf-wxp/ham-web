//! 考试速查页：操作证权限、分区号、RST 信号报告、发射类别标识、通联英语短句。

use ham_web_core::reference::{CALL_AREAS, EMISSION_TYPES, LICENSE_CLASSES, PHRASES, RST_SCALES};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::util::set_title;

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn ReferencePage() -> impl IntoView {
  set_title("考试速查");
  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader title="考试速查" subtitle="操作证权限 · 分区 · RST · 发射类别 · 通联英语" />
      <PageContainer>
        // 操作证类别与权限
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"操作证类别与使用权限"</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[560px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>"类别"</th>
                  <th class=CELL>"频率范围"</th>
                  <th class=CELL>"功率上限"</th>
                  <th class=CELL>"说明"</th>
                </tr>
              </thead>
              <tbody>
                {LICENSE_CLASSES
                  .iter()
                  .map(|c| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} font-mono font-semibold")>{c.class} " 类"</td>
                        <td class=CELL>{c.freq}</td>
                        <td class=CELL>{c.power}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{c.note}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>

        // 分区号
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"业余电台分区号"</h2>
          <div class="grid grid-cols-2 gap-2 p-4 sm:grid-cols-3 lg:grid-cols-5">
            {CALL_AREAS
              .iter()
              .map(|a| {
                view! {
                  <div class="flex items-start gap-2 rounded-lg border bg-muted/30 p-3">
                    <span class="font-mono text-lg font-semibold text-primary">{a.digit}</span>
                    <span class="text-xs leading-5 text-muted-foreground">{a.regions}</span>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        // RST 信号报告
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"RST 信号报告"</h2>
          <div class="grid grid-cols-1 gap-3 p-4 md:grid-cols-3">
            {RST_SCALES
              .iter()
              .map(|s| {
                view! {
                  <div class="rounded-lg border">
                    <div class="rounded-t-lg border-b bg-muted/40 px-3 py-2">
                      <span class="font-mono font-semibold text-primary">{s.key}</span>
                      <span class="ml-2 text-sm font-medium">{s.name}</span>
                      <div class="text-xs text-muted-foreground">{s.note}</div>
                    </div>
                    <dl class="divide-y text-sm">
                      {s
                        .levels
                        .iter()
                        .map(|&(num, desc)| {
                          view! {
                            <div class="flex gap-2 px-3 py-1.5">
                              <dt class="font-mono font-semibold tabular-nums">{num}</dt>
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
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"发射类别标识"</h2>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[480px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>"标识"</th>
                  <th class=CELL>"名称"</th>
                  <th class=CELL>"说明"</th>
                </tr>
              </thead>
              <tbody>
                {EMISSION_TYPES
                  .iter()
                  .map(|e| {
                    view! {
                      <tr class="border-t transition-colors hover:bg-muted/40">
                        <td class=format!("{CELL} font-mono font-semibold")>{e.code}</td>
                        <td class=CELL>{e.name}</td>
                        <td class=format!("{CELL} text-muted-foreground")>{e.desc}</td>
                      </tr>
                    }
                  })
                  .collect_view()}
              </tbody>
            </table>
          </div>
        </section>

        // 通联英语
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"通联英语短句"</h2>
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
      </PageContainer>
    </div>
  }
}

use std::collections::{HashMap, HashSet};

use leptos::prelude::*;

use crate::pages::log::use_log_store;
use crate::util::set_title;

use super::bar_chart::BarChart;

/// 取 Top N（按计数降序）。
fn top_n(mut map: HashMap<String, usize>, n: usize) -> Vec<(String, usize)> {
  let mut v: Vec<(String, usize)> = map.drain().collect();
  v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
  v.truncate(n);
  v
}

#[component]
pub fn StatsPage() -> impl IntoView {
  set_title("通联统计");

  let entries = use_log_store().logbook.get_untracked().entries;
  let total = entries.len();

  let mut dxcc: HashMap<String, usize> = HashMap::new();
  let mut band: HashMap<String, usize> = HashMap::new();
  let mut mode: HashMap<String, usize> = HashMap::new();
  let mut monthly: HashMap<String, usize> = HashMap::new();
  let mut band_dxcc: HashMap<String, HashSet<String>> = HashMap::new();

  for e in &entries {
    if !e.callsign.is_empty() {
      *dxcc
        .entry(e.entity().map(|x| x.name).unwrap_or("其他").to_owned())
        .or_default() += 1;
    }
    let b = e.band_label();
    if !b.is_empty() {
      if let Some(entity) = e.entity().map(|x| x.name) {
        band_dxcc
          .entry(b.clone())
          .or_default()
          .insert(entity.to_owned());
      }
      *band.entry(b).or_default() += 1;
    }
    if !e.mode.is_empty() {
      *mode.entry(e.mode.clone()).or_default() += 1;
    }
    if let Some(ym) = e.date.get(..7) {
      *monthly.entry(ym.to_owned()).or_default() += 1;
    }
  }

  let qsl_sent_count = entries.iter().filter(|e| e.qsl_sent).count();
  let qsl_rcvd_count = entries.iter().filter(|e| e.qsl_rcvd).count();

  let dxcc_total = dxcc.keys().filter(|k| *k != "其他").count();
  let dxcc_top = top_n(dxcc, 10);
  let mut band_dxcc_list: Vec<(String, usize)> = band_dxcc
    .iter()
    .map(|(b, s)| (b.clone(), s.len()))
    .collect();
  band_dxcc_list.sort();
  let band_top = top_n(band, 10);
  let mode_top = top_n(mode, 8);
  let mut months: Vec<(String, usize)> = monthly.into_iter().collect();
  months.sort_by(|a, b| a.0.cmp(&b.0));
  let month_max = months.iter().map(|(_, c)| *c).max().unwrap_or(1);

  let empty = total == 0;

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"通联统计"</h1>
            <div class="text-xs text-muted-foreground">"日志可视化 · DXCC / 波段 / 模式 / 趋势"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-4 px-4 py-5">
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">{total}</div>
            <div class="mt-1 text-xs text-muted-foreground">"总 QSO"</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">{dxcc_total}</div>
            <div class="mt-1 text-xs text-muted-foreground">"DXCC 实体"</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">{band_top.len()}</div>
            <div class="mt-1 text-xs text-muted-foreground">"波段数"</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">{mode_top.len()}</div>
            <div class="mt-1 text-xs text-muted-foreground">"模式数"</div>
          </div>
        </div>

        {if empty {
          view! {
            <section class="rounded-xl border bg-card p-8 text-center text-sm text-muted-foreground">
              "暂无通联日志。"
              <a
                href="/log"
                class="mt-2 inline-block underline underline-offset-4 transition-colors hover:text-foreground"
              >
                "去添加通联日志 →"
              </a>
            </section>
          }
          .into_any()
        } else {
          view! {
            <section class="rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">"DXCC 奖状进度"</h2>
              <div class="space-y-3 p-4">
                <div>
                  <div class="mb-1 flex items-center justify-between text-xs text-muted-foreground">
                    <span>"DXCC 实体（目标 100）"</span>
                    <span class="tabular-nums">{dxcc_total} " / 100"</span>
                  </div>
                  <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
                    <div
                      class="h-full rounded-full bg-primary transition-all"
                      style=format!("width: {:.1}%", dxcc_total.min(100) as f64)
                    ></div>
                  </div>
                </div>
                <div>
                  <div class="mb-1.5 text-xs font-medium text-muted-foreground">"按波段的 DXCC 实体数"</div>
                  <div class="flex flex-wrap gap-1.5">
                    {band_dxcc_list
                      .iter()
                      .map(|(b, n)| {
                        view! {
                          <span class="rounded-full border bg-muted/40 px-2.5 py-0.5 text-xs tabular-nums">
                            {b.clone()} " " <span class="font-semibold text-primary">{*n}</span>
                          </span>
                        }
                      })
                      .collect_view()}
                  </div>
                </div>
              </div>
            </section>

            <section class="rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">"QSL 确认状态"</h2>
              <div class="space-y-3 p-4">
                <div>
                  <div class="mb-1 flex items-center justify-between text-xs text-muted-foreground">
                    <span>"已确认收到 QSL"</span>
                    <span class="tabular-nums">{qsl_rcvd_count} " / " {total}</span>
                  </div>
                  <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
                    <div
                      class="h-full rounded-full bg-emerald-500"
                      style=format!("width: {:.1}%", qsl_rcvd_count as f64 / total.max(1) as f64 * 100.0)
                    ></div>
                  </div>
                </div>
                <div>
                  <div class="mb-1 flex items-center justify-between text-xs text-muted-foreground">
                    <span>"已寄出（待确认）"</span>
                    <span class="tabular-nums">{qsl_sent_count} " / " {total}</span>
                  </div>
                  <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
                    <div
                      class="h-full rounded-full bg-amber-500"
                      style=format!("width: {:.1}%", qsl_sent_count as f64 / total.max(1) as f64 * 100.0)
                    ></div>
                  </div>
                </div>
              </div>
            </section>

            <section class="rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">"DXCC 实体分布（Top 10）"</h2>
              <div class="p-4">
                <BarChart items=dxcc_top.clone() max=dxcc_top.first().map(|(_, c)| *c).unwrap_or(1) />
              </div>
            </section>

            <section class="rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">"波段分布"</h2>
              <div class="p-4">
                <BarChart items=band_top.clone() max=band_top.first().map(|(_, c)| *c).unwrap_or(1) />
              </div>
            </section>

            <section class="rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">"模式分布"</h2>
              <div class="p-4">
                <BarChart items=mode_top.clone() max=mode_top.first().map(|(_, c)| *c).unwrap_or(1) />
              </div>
            </section>

            <section class="rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">"月度 QSO 趋势"</h2>
              <div class="p-4">
                <div class="flex h-32 items-end gap-1">
                  {months
                    .iter()
                    .map(|(m, c)| {
                      let h = (*c as f64 / month_max as f64 * 100.0).max(2.0);
                      view! {
                        <div class="group flex min-w-0 flex-1 flex-col items-center gap-1">
                          <div
                            class="w-full rounded-t bg-primary/70 transition-all group-hover:bg-primary"
                            style=format!("height: {h}%")
                            title=format!("{m}：{c} 条")
                          ></div>
                          <span class="text-[10px] text-muted-foreground">{m.get(5..).unwrap_or(m)}</span>
                        </div>
                      }
                    })
                    .collect_view()}
                </div>
              </div>
            </section>
          }
          .into_any()
        }}

        <p class="text-xs text-muted-foreground">
          "统计基于本地通联日志；DXCC 实体由呼号前缀识别，波段按频率归并。"
        </p>
      </div>
    </div>
  }
}

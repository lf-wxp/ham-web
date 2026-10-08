//! 通联记录列表：关键字 / 波段 / 模式 / QSL 筛选，分页表格与编辑 / 删除 / 清空操作。

use std::sync::Arc;

use ham_web_core::qsl_status::{QslStatus, QslTone};
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::ui::{Button, Input, InputType, NativeSelect, SelectOption, Size, Variant};

use super::card_image_mark::CardImageMark;
use super::grid_cell::GridCell;
use super::log_helpers::{CELL, PAGE_SIZE, station_title};
use super::qsl_badge::{QslBadge, tone_label};
use super::{LogEntry, Logbook, use_log_store};
use crate::i18n::{t, tf, tp};

#[component]
pub(super) fn EntryList(
  logbook: RwSignal<Logbook>,
  on_edit: Callback<LogEntry>,
  on_remove: Callback<u64>,
  on_clear: Callback<()>,
  /// 初始搜索词（如从 DXCC 地图点击实体后带过来的实体名）。
  #[prop(optional, into)]
  initial_query: String,
) -> impl IntoView {
  // 台站档案册（多台站时每行标出归属）。
  let store = use_log_store();
  // 列表筛选与分页
  let query = RwSignal::new(initial_query);
  let band_filter = RwSignal::new(String::new());
  let mode_filter = RwSignal::new(String::new());
  let qsl_filter = RwSignal::new(String::new());
  let page = RwSignal::new(0usize);
  Effect::new(move |_| {
    query.track();
    band_filter.track();
    mode_filter.track();
    qsl_filter.track();
    page.set(0);
  });

  // `Arc` 包一层：`Memo` 只能返回自有数据，所以「筛选结果变了」时克隆整份结果没法避免；
  // 但**渲染**（翻页、改主题、任意重渲）不该再克隆一次整表 —— 一页只显示 `PAGE_SIZE` 行，
  // 用 `Arc` 后「取整表」只是加一次引用计数。
  let filtered = Memo::new(move |_| {
    let q = query.get().trim().to_uppercase();
    let (bf, mf, qf) = (band_filter.get(), mode_filter.get(), qsl_filter.get());
    let mut list: Vec<LogEntry> = logbook.with(|lb| {
      lb.entries
        .iter()
        .filter(|e| e.matches_query(&q))
        .filter(|e| bf.is_empty() || e.band_label() == bf)
        .filter(|e| mf.is_empty() || e.mode == mf)
        // QSL 筛选按**四色档位**（走到哪一步）而不是单个标志位：走 LoTW / eQSL
        // 确认的通联以前会被 `qsl_rcvd` 判成「未确认」，看着像没寄过卡。
        .filter(|e| match QslTone::from_key(&qf) {
          Some(tone) => QslStatus::of(e).tone() == tone,
          None => true,
        })
        .cloned()
        .collect()
    });
    list.sort_by(|a, b| (&b.date, &b.time, b.id).cmp(&(&a.date, &a.time, a.id)));
    Arc::new(list)
  });
  let page_count = Memo::new(move |_| filtered.with(|v| v.len()).div_ceil(PAGE_SIZE).max(1));

  view! {
    // 日志列表
    <section class="rounded-xl border bg-card">
      <div class="flex items-center justify-between border-b px-4 py-3">
        <h2 class="text-sm font-semibold">
          {move || t("log.records-3")}
          <span class="ml-2 text-xs font-normal text-muted-foreground">
            {move || {
              let (n, total) = (filtered.with(|v| v.len()), logbook.with(|l| l.entries.len()));
              if n == total {
                tp("log.records", total, &[&total.to_string()])
              } else {
                tp("log.records-2", total as u32, &[&n.to_string(), &total.to_string()])
              }
            }}
          </span>
        </h2>
        <Button
          variant=Variant::Ghost
          size=Size::Sm
          class="text-muted-foreground"
          on_click=Callback::new(move |_| on_clear.run(()))
        >
          {move || t("learning.clear")}
        </Button>
      </div>

      {move || {
        if logbook.with(|l| l.entries.is_empty()) {
          return view! {
            <div class="px-4 py-10 text-center text-sm text-muted-foreground">
              {move || t("log.no-records-yet-add")}
            </div>
          }
          .into_any();
        }
        // 去重与排序在 core（`distinct_bands_and_modes`）：这里只拼下拉项。
        let (bands, modes) =
          logbook.with(|l| ham_web_core::log_stats::distinct_bands_and_modes(&l.entries));
        let band_options: Vec<SelectOption> = bands
          .iter()
          .map(|b| SelectOption::new(b.as_str(), b.as_str()))
          .collect();
        let mode_options: Vec<SelectOption> = modes
          .iter()
          .map(|m| SelectOption::new(m.as_str(), m.as_str()))
          .collect();
        // QSL 四色档位的筛选项：颜色即「走到哪一步」。选项在 `view!` 之外先算好 ——
        // 宏里 `collect::<Vec<_>>()` 的 `>` 会被当成标签结束符，宏直接解析失败。
        let tone_options: Vec<SelectOption> = QslTone::ALL
          .into_iter()
          .map(|tone| SelectOption::new(tone.key(), Signal::derive(move || tone_label(tone))))
          .collect();
        view! {
          <div class="flex flex-wrap gap-2 border-b px-4 py-3">
            <Input
              value=query
              on_change=Callback::new(move |v: String| query.set(v))
              kind=InputType::Search
              placeholder=Signal::derive(move || t("log.search-callsign-name-qth"))
              // 工具栏上的筛选器都没有可见标签：无障碍名是读屏与 e2e 的定位契约。
              aria_label=Signal::derive(move || t("log.search-callsign-name-qth"))
              prefix=move || view! { <Icon kind=IconKind::Search /> }
              clearable=true
              wrapper_class="min-w-48 flex-1"
            />
            <NativeSelect
              value=band_filter
              on_change=Callback::new(move |v: String| band_filter.set(v))
              options=band_options
              placeholder=Signal::derive(move || t("log.all-bands"))
              aria_label=Signal::derive(move || t("log.band-filter"))
              class="w-28"
            />
            <NativeSelect
              value=mode_filter
              on_change=Callback::new(move |v: String| mode_filter.set(v))
              options=mode_options
              placeholder=Signal::derive(move || t("log.all-modes"))
              aria_label=Signal::derive(move || t("log.mode-filter"))
              class="w-28"
            />
            <NativeSelect
              value=qsl_filter
              on_change=Callback::new(move |v: String| qsl_filter.set(v))
              options=tone_options
              placeholder=Signal::derive(move || t("log.all-qsl"))
              aria_label=Signal::derive(move || t("log.qsl-filter"))
              class="w-32"
            />
          </div>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[880px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("log.date")}</th>
                  <th class=CELL>{move || t("log.time")}</th>
                  <th class=CELL>{move || t("log.freq-band")}</th>
                  <th class=CELL>{move || t("log.mode")}</th>
                  <th class=CELL>{move || t("log.callsign")}</th>
                  <th class=CELL>{move || t("log.rst-sent-rcvd")}</th>
                  <th class=CELL>{move || t("log.grid")}</th>
                  <th class=CELL>{move || t("log.notes")}</th>
                  <th class=CELL>"QSL"</th>
                  <th class=CELL>{move || t("log.actions")}</th>
                </tr>
              </thead>
              <tbody>
                {move || {
                  let start = page.get().min(page_count.get() - 1) * PAGE_SIZE;
                  filtered
                    .get()
                    .iter()
                    .skip(start)
                    .take(PAGE_SIZE)
                    .map(|e| {
                      let entry = e.clone();
                      let id = e.id;
                      // 归属在闭包外取出：行内那个 `move ||` 只该捕获这个 Copy 值，
                      // 捕获 `e` 会借住上面的临时迭代器。
                      let station_id = e.station_id;
                      let rst = if e.rst_sent.is_empty() && e.rst_rcvd.is_empty() {
                        "—".to_owned()
                      } else {
                        format!("{} / {}", e.rst_sent, e.rst_rcvd)
                      };
                      let band = e.band_label();
                      let tags: Vec<String> = [
                        (!e.sat_name.is_empty()).then(|| format!("SAT {}", e.sat_name)),
                        (e.sat_name.is_empty() && !e.prop_mode.is_empty()).then(|| e.prop_mode.clone()),
                        (!e.sota_ref.is_empty()).then(|| format!("SOTA {}", e.sota_ref)),
                        (!e.pota_ref.is_empty()).then(|| format!("POTA {}", e.pota_ref)),
                        (!e.tx_pwr.is_empty()).then(|| format!("{} W", e.tx_pwr)),
                      ]
                      .into_iter()
                      .flatten()
                      .collect();
                      view! {
                        <tr class="border-t transition-colors hover:bg-muted/40">
                          <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{e.date.clone()}</td>
                          <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{e.time.clone()}</td>
                          <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>
                            {if e.freq.is_empty() { "—".to_owned() } else { e.freq.clone() }}
                            <span class="ml-1.5 text-xs text-muted-foreground">{band}</span>
                          </td>
                          <td class=format!("{CELL} whitespace-nowrap")>{e.mode.clone()}</td>
                          <td class=format!("{CELL} whitespace-nowrap")>
                            <span class="font-mono font-medium">{e.callsign.clone()}</span>
                            // 多台站时标出这条通联的归属：只有一个台站（绝大多数人）时不显示，
                            // 免得每行都挂一个没信息量的标签。
                            {move || {
                              store
                                .station
                                .with(|b| {
                                  (b.profiles.len() > 1)
                                    .then(|| station_title(b.resolve(station_id)))
                                })
                                .map(|title| {
                                  view! {
                                    <span
                                      class="ml-1 text-xs font-normal text-muted-foreground"
                                      title=move || t("log.qso-station")
                                    >
                                      {title}
                                    </span>
                                  }
                                })
                            }}
                          </td>
                          <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{rst}</td>
                          <td class=format!("{CELL} whitespace-nowrap")>
                            {if e.gridsquare.is_empty() {
                              view! { <span class="text-muted-foreground">"—"</span> }.into_any()
                            } else {
                              view! { <GridCell grid=e.gridsquare.clone() /> }.into_any()
                            }}
                          </td>
                          <td class=format!("{CELL} text-muted-foreground")>
                            {e.remark.clone()}
                            {tags
                              .into_iter()
                              .map(|t| view! { <span class="ml-1 inline-block rounded border px-1 text-[10px]">{t}</span> })
                              .collect_view()}
                          </td>
                          <td data-slot="qsl-cell" class=format!("{CELL} whitespace-nowrap")>
                            <div class="flex items-center gap-1">
                              <QslBadge status=QslStatus::of(e) />
                              <CardImageMark entry_id=id />
                            </div>
                          </td>
                          <td class=format!("{CELL} whitespace-nowrap")>
                            <div class="flex items-center gap-2">
                              <button
                                type="button"
                                class="text-xs text-muted-foreground transition-colors hover:text-foreground"
                                on:click=move |_| on_edit.run(entry.clone())
                              >
                                {move || t("log.edit")}
                              </button>
                              <button
                                type="button"
                                class="text-xs text-muted-foreground transition-colors hover:text-destructive"
                                on:click=move |_| on_remove.run(id)
                              >
                                {move || t("log.delete")}
                              </button>
                            </div>
                          </td>
                        </tr>
                      }
                    })
                    .collect_view()
                }}
              </tbody>
            </table>
          </div>
          {move || {
            (page_count.get() > 1).then(|| {
              view! {
                <div class="flex items-center justify-end gap-2 px-4 py-3 text-xs text-muted-foreground">
                  <Button
                    variant=Variant::Outline
                    size=Size::Sm
                    disabled=Signal::derive(move || page.get() == 0)
                    on_click=Callback::new(move |_| page.update(|p| *p = p.saturating_sub(1)))
                  >
                    {move || t("log.previous")}
                  </Button>
                  <span class="tabular-nums">{move || tf("log.page", &[&(page.get() + 1).to_string(), &page_count.get().to_string()])}</span>
                  <Button
                    variant=Variant::Outline
                    size=Size::Sm
                    disabled=Signal::derive(move || page.get() + 1 >= page_count.get())
                    on_click=Callback::new(move |_| page.update(|p| *p += 1))
                  >
                    {move || t("log.next")}
                  </Button>
                </div>
              }
            })
          }}
        }
        .into_any()
      }}
    </section>
  }
}

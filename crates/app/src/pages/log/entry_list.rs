//! 通联记录列表：关键字 / 波段 / 模式 / QSL 筛选，分页表格与编辑 / 删除 / 清空操作。

use std::sync::Arc;

use ham_web_core::qsl_status::{QslStatus, QslTone};
use leptos::prelude::*;

use crate::components::common::EmptyState;
use crate::icons::{Icon, IconKind};
use crate::ui::{
  Button, Input, InputType, Pagination, SelectOption, SimpleSelect, Size, Table, TableBody,
  TableCell, TableHead, TableHeader, TableRow, Tooltip, Variant, card_title_class,
};

use super::card_image_mark::CardImageMark;
use super::grid_cell::GridCell;
use super::log_helpers::{PAGE_SIZE, station_title};
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
  // 搜索框绑 `query`（字立刻上屏），筛选按防抖后的副本算：连续输入只重算一次表格。
  let query_settled = crate::debounced::use_debounced(query, crate::debounced::SEARCH_DEBOUNCE);
  // 筛选条件变了回到第一页。跟的是**防抖后**的关键词，而且已经在第一页就不写：`RwSignal::set`
  // 不比较新旧值、照样通知，此前每敲一个字都会让当前这一页的 50 行整体重建一遍（筛选结果还没变）。
  Effect::new(move |_| {
    query_settled.track();
    band_filter.track();
    mode_filter.track();
    qsl_filter.track();
    crate::debounced::set_if_changed(page, 0);
  });

  // 筛选 + 排序只产出**下标**（指向 `logbook.entries`），不克隆记录：此前每输入一个字都要把全部命中的
  // 记录克隆一遍再排序，5000 条日志时每键一百多毫秒。渲染时只取当前这一页的 `PAGE_SIZE` 条。
  // `Arc` 包一层：翻页、改主题这类重渲拿整份下标只是加一次引用计数。
  //
  // 排序与关键词无关：按「日期、时间、编号」倒序的整表次序单独挂一个 Memo，日志变了才重排；
  // 筛选只是按这个次序挑出命中的，不再每换一次关键词就把几千条重排一遍。
  let order = Memo::new(move |_| {
    logbook.with(|lb| {
      let es = &lb.entries;
      let mut idx: Vec<usize> = (0..es.len()).collect();
      idx.sort_by(|&a, &b| {
        let (a, b) = (&es[a], &es[b]);
        (&b.date, &b.time, b.id).cmp(&(&a.date, &a.time, a.id))
      });
      Arc::new(idx)
    })
  });
  let filtered = Memo::new(move |_| {
    let q = query_settled.with(|q| q.trim().to_uppercase());
    let (bf, mf, qf) = (band_filter.get(), mode_filter.get(), qsl_filter.get());
    // QSL 筛选按**四色档位**（走到哪一步）而不是单个标志位：走 LoTW / eQSL
    // 确认的通联以前会被 `qsl_rcvd` 判成「未确认」，看着像没寄过卡。
    let tone = QslTone::from_key(&qf);
    let all = order.get();
    if q.is_empty() && bf.is_empty() && mf.is_empty() && tone.is_none() {
      return all;
    }
    logbook.with(|lb| {
      let es = &lb.entries;
      Arc::new(
        all
          .iter()
          .copied()
          .filter(|&i| {
            let e = &es[i];
            (bf.is_empty() || e.band_label() == bf)
              && (mf.is_empty() || e.mode == mf)
              && tone.is_none_or(|tone| QslStatus::of(e).tone() == tone)
              && e.matches_query(&q)
          })
          .collect::<Vec<usize>>(),
      )
    })
  });
  let page_count = Memo::new(move |_| filtered.with(|v| v.len()).div_ceil(PAGE_SIZE).max(1));

  view! {
    // 日志列表
    <section class=crate::kit::card_class()>
      <div class="flex items-center justify-between border-b px-4 py-3">
        <h2 class=card_title_class("")>
          {move || t("log.records-heading")}
          <span class="ml-2 text-xs font-normal text-muted-foreground">
            {move || {
              let (n, total) = (filtered.with(|v| v.len()), logbook.with(|l| l.entries.len()));
              if n == total {
                tp("log.records", total, &[&total.to_string()])
              } else {
                tp("log.records-filtered", total as u32, &[&n.to_string(), &total.to_string()])
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
            <EmptyState
              plain=true
              title=Signal::derive(move || t("log.no-records-yet-add"))
              icon=IconKind::ClipboardList
            />
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
            <SimpleSelect
              value=band_filter
              on_change=Callback::new(move |v: String| band_filter.set(v))
              options=band_options
              placeholder=Signal::derive(move || t("log.all-bands"))
              aria_label=Signal::derive(move || t("log.band-filter"))
              class="w-28"
            />
            <SimpleSelect
              value=mode_filter
              on_change=Callback::new(move |v: String| mode_filter.set(v))
              options=mode_options
              placeholder=Signal::derive(move || t("log.all-modes"))
              aria_label=Signal::derive(move || t("log.mode-filter"))
              class="w-28"
            />
            <SimpleSelect
              value=qsl_filter
              on_change=Callback::new(move |v: String| qsl_filter.set(v))
              options=tone_options
              placeholder=Signal::derive(move || t("log.all-qsl"))
              aria_label=Signal::derive(move || t("log.qsl-filter"))
              class="w-32"
            />
          </div>
          <Table class="min-w-[880px]" density=crate::ui::TableDensity::Compact>
            <TableHead>
              <TableRow>
                <TableHeader class=crate::kit::table_cell_class("align-top")>{move || t("log.date")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("align-top")>{move || t("log.time")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("align-top")>{move || t("log.freq-band")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("align-top")>{move || t("log.mode")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("align-top")>{move || t("log.callsign")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("align-top")>{move || t("log.rst-sent-rcvd")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("align-top")>{move || t("log.grid")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("align-top")>{move || t("log.notes")}</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("align-top")>"QSL"</TableHeader>
                <TableHeader class=crate::kit::table_cell_class("align-top")>{move || t("log.actions")}</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {move || {
                let start = page.get().min(page_count.get() - 1) * PAGE_SIZE;
                let rows: Vec<LogEntry> = filtered.with(|idx| {
                  logbook.with(|lb| {
                    idx
                      .iter()
                      .skip(start)
                      .take(PAGE_SIZE)
                      .filter_map(|&i| lb.entries.get(i).cloned())
                      .collect()
                  })
                });
                rows
                  .iter()
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
                      (!e.my_sota_ref.is_empty()).then(|| format!("SOTA {}", e.my_sota_ref)),
                      (!e.my_pota_ref.is_empty()).then(|| format!("POTA {}", e.my_pota_ref)),
                      (!e.contact_sota_ref.is_empty()).then(|| format!("S2S {}", e.contact_sota_ref)),
                      (!e.contact_pota_ref.is_empty()).then(|| format!("P2P {}", e.contact_pota_ref)),
                      (!e.tx_pwr.is_empty()).then(|| format!("{} W", e.tx_pwr)),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();
                    let date = e.date.clone();
                    let time = e.time.clone();
                    let freq = e.freq.clone();
                    let mode = e.mode.clone();
                    let callsign = e.callsign.clone();
                    let gridsquare = e.gridsquare.clone();
                    let remark = e.remark.clone();
                    let qsl = QslStatus::of(e);
                    view! {
                      <TableRow>
                        <TableCell class=crate::kit::table_cell_class("whitespace-nowrap tabular-nums")>{date}</TableCell>
                        <TableCell class=crate::kit::table_cell_class("whitespace-nowrap tabular-nums")>{time}</TableCell>
                        <TableCell class=crate::kit::table_cell_class("whitespace-nowrap font-mono tabular-nums")>
                          {if freq.is_empty() { "—".to_owned() } else { freq }}
                          <span class="ml-1.5 text-xs text-muted-foreground">{band}</span>
                        </TableCell>
                        <TableCell class=crate::kit::table_cell_class("whitespace-nowrap")>{mode}</TableCell>
                        <TableCell class=crate::kit::table_cell_class("whitespace-nowrap")>
                          <span class="font-mono">{callsign}</span>
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
                                  <Tooltip label=Signal::derive(move || t("log.qso-station"))>
                                    <span class="ml-1 text-xs font-normal text-muted-foreground">
                                      {title}
                                    </span>
                                  </Tooltip>
                                }
                              })
                          }}
                        </TableCell>
                        <TableCell class=crate::kit::table_cell_class("whitespace-nowrap tabular-nums")>{rst}</TableCell>
                        <TableCell class=crate::kit::table_cell_class("whitespace-nowrap")>
                          {if gridsquare.is_empty() {
                            view! { <span class="text-muted-foreground">"—"</span> }.into_any()
                          } else {
                            view! { <GridCell grid=gridsquare /> }.into_any()
                          }}
                        </TableCell>
                        <TableCell class=crate::kit::table_cell_class("text-muted-foreground")>
                          {remark}
                          {tags
                            .into_iter()
                            .map(|t| view! { <span class="ml-1 inline-block border px-1 text-xs">{t}</span> })
                            .collect_view()}
                        </TableCell>
                        <td data-slot="qsl-cell" class=crate::kit::table_cell_class("whitespace-nowrap")>
                          <div class="flex items-center gap-1">
                            <QslBadge status=qsl />
                            <CardImageMark entry_id=id />
                          </div>
                        </td>
                        <TableCell class=crate::kit::table_cell_class("whitespace-nowrap")>
                          <div class="flex items-center gap-2">
                            <Button
                              variant=Variant::Text
                              size=Size::Xs
                              on_click=Callback::new(move |_| on_edit.run(entry.clone()))
                            >
                              {move || t("log.edit")}
                            </Button>
                            <Button
                              variant=Variant::Text
                              size=Size::Xs
                              class="hover:text-hp-text"
                              on_click=Callback::new(move |_| on_remove.run(id))
                            >
                              {move || t("log.delete")}
                            </Button>
                          </div>
                        </TableCell>
                      </TableRow>
                    }
                  })
                  .collect_view()
              }}
            </TableBody>
          </Table>
          {move || {
            view! {
              <div class="flex flex-wrap items-center justify-end gap-2 px-4 py-3">
                <span class="text-xs tabular-nums text-muted-foreground">
                  {move || tf("log.page", &[&(page.get() + 1).to_string(), &page_count.get().to_string()])}
                </span>
                <Pagination
                  page=Signal::derive(move || page.get() as u32 + 1)
                  total_pages=Signal::derive(move || page_count.get() as u32)
                  on_change=Callback::new(move |p| page.set((p - 1) as usize))
                />
              </div>
            }
          }}
        }
        .into_any()
      }}
    </section>
  }
}

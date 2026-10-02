//! 通联记录列表：关键字 / 波段 / 模式 / QSL 筛选，分页表格与编辑 / 删除 / 清空操作。

use leptos::prelude::*;

use crate::ui::{Size, Variant, button_class, input_class};

use super::grid_cell::GridCell;
use super::log_helpers::{CELL, PAGE_SIZE};
use super::{LogEntry, Logbook};
use crate::i18n::{t, tf};

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

  let filtered = Memo::new(move |_| {
    let q = query.get().trim().to_uppercase();
    let (bf, mf, qf) = (band_filter.get(), mode_filter.get(), qsl_filter.get());
    let mut list: Vec<LogEntry> = logbook.with(|lb| {
      lb.entries
        .iter()
        .filter(|e| e.matches_query(&q))
        .filter(|e| bf.is_empty() || e.band_label() == bf)
        .filter(|e| mf.is_empty() || e.mode == mf)
        .filter(|e| match qf.as_str() {
          "rcvd" => e.qsl_rcvd,
          "pending" => !e.qsl_rcvd,
          _ => true,
        })
        .cloned()
        .collect()
    });
    list.sort_by(|a, b| (&b.date, &b.time, b.id).cmp(&(&a.date, &a.time, a.id)));
    list
  });
  let page_count = Memo::new(move |_| filtered.with(Vec::len).div_ceil(PAGE_SIZE).max(1));

  view! {
    // 日志列表
    <section class="rounded-xl border bg-card">
      <div class="flex items-center justify-between border-b px-4 py-3">
        <h2 class="text-sm font-semibold">
          {move || t("记录列表")}
          <span class="ml-2 text-xs font-normal text-muted-foreground">
            {move || {
              let (n, total) = (filtered.with(Vec::len), logbook.with(|l| l.entries.len()));
              if n == total {
                tf("共 {} 条", &[&total.to_string()])
              } else {
                tf("筛选出 {} / {} 条", &[&n.to_string(), &total.to_string()])
              }
            }}
          </span>
        </h2>
        <button
          type="button"
          class=button_class(Variant::Ghost, Size::Sm, "text-muted-foreground")
          on:click=move |_| on_clear.run(())
        >
          {move || t("清空")}
        </button>
      </div>

      {move || {
        if logbook.with(|l| l.entries.is_empty()) {
          return view! {
            <div class="px-4 py-10 text-center text-sm text-muted-foreground">
              {move || t("暂无记录，添加第一条通联日志吧。")}
            </div>
          }
          .into_any();
        }
        let (bands, modes): (Vec<String>, Vec<String>) = logbook.with(|l| {
          let mut b: Vec<String> = l.entries.iter().map(LogEntry::band_label).filter(|s| !s.is_empty()).collect();
          let mut m: Vec<String> = l.entries.iter().map(|e| e.mode.clone()).collect();
          b.sort();
          b.dedup();
          m.sort();
          m.dedup();
          (b, m)
        });
        view! {
          <div class="flex flex-wrap gap-2 border-b px-4 py-3">
            <input
              type="search"
              placeholder=move || t("搜索呼号 / 姓名 / QTH / 网格 / 备注")
              prop:value=move || query.get()
              on:input=move |e| query.set(event_target_value(&e))
              class=input_class("min-w-48 flex-1")
            />
            <select prop:value=move || band_filter.get() on:change=move |e| band_filter.set(event_target_value(&e)) class=input_class("w-28")>
              <option value="">{move || t("全部波段")}</option>
              {bands.into_iter().map(|b| view! { <option value=b.clone()>{b.clone()}</option> }).collect_view()}
            </select>
            <select prop:value=move || mode_filter.get() on:change=move |e| mode_filter.set(event_target_value(&e)) class=input_class("w-28")>
              <option value="">{move || t("全部模式")}</option>
              {modes.into_iter().map(|m| view! { <option value=m.clone()>{m.clone()}</option> }).collect_view()}
            </select>
            <select prop:value=move || qsl_filter.get() on:change=move |e| qsl_filter.set(event_target_value(&e)) class=input_class("w-28")>
              <option value="">{move || t("全部 QSL")}</option>
              <option value="pending">{move || t("未确认")}</option>
              <option value="rcvd">{move || t("已确认")}</option>
            </select>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full min-w-[880px] border-collapse text-sm">
              <thead class="bg-muted/60 text-xs">
                <tr>
                  <th class=CELL>{move || t("日期")}</th>
                  <th class=CELL>{move || t("时间")}</th>
                  <th class=CELL>{move || t("频率 / 波段")}</th>
                  <th class=CELL>{move || t("模式")}</th>
                  <th class=CELL>{move || t("呼号")}</th>
                  <th class=CELL>{move || t("RST 发/收")}</th>
                  <th class=CELL>{move || t("网格")}</th>
                  <th class=CELL>{move || t("备注")}</th>
                  <th class=CELL>"QSL"</th>
                  <th class=CELL>{move || t("操作")}</th>
                </tr>
              </thead>
              <tbody>
                {move || {
                  let start = page.get().min(page_count.get() - 1) * PAGE_SIZE;
                  filtered
                    .get()
                    .into_iter()
                    .skip(start)
                    .take(PAGE_SIZE)
                    .map(|e| {
                      let entry = e.clone();
                      let id = e.id;
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
                          <td class=format!("{CELL} whitespace-nowrap font-mono font-medium")>{e.callsign.clone()}</td>
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
                          <td class=format!("{CELL} whitespace-nowrap")>
                            {if e.qsl_rcvd {
                              view! {
                                <span class="font-medium text-emerald-600 dark:text-emerald-400">
                                  {move || t("已确认")}
                                </span>
                              }
                              .into_any()
                            } else if e.qsl_sent {
                              view! {
                                <span class="text-amber-600 dark:text-amber-400">{move || t("已寄出")}</span>
                              }
                              .into_any()
                            } else {
                              view! { <span class="text-muted-foreground">"—"</span> }.into_any()
                            }}
                          </td>
                          <td class=format!("{CELL} whitespace-nowrap")>
                            <div class="flex items-center gap-2">
                              <button
                                type="button"
                                class="text-xs text-muted-foreground transition-colors hover:text-foreground"
                                on:click=move |_| on_edit.run(entry.clone())
                              >
                                {move || t("编辑")}
                              </button>
                              <button
                                type="button"
                                class="text-xs text-muted-foreground transition-colors hover:text-destructive"
                                on:click=move |_| on_remove.run(id)
                              >
                                {move || t("删除")}
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
                  <button
                    type="button"
                    class=button_class(Variant::Outline, Size::Sm, "")
                    disabled=move || page.get() == 0
                    on:click=move |_| page.update(|p| *p = p.saturating_sub(1))
                  >
                    {move || t("上一页")}
                  </button>
                  <span class="tabular-nums">{move || tf("第 {} / {} 页", &[&(page.get() + 1).to_string(), &page_count.get().to_string()])}</span>
                  <button
                    type="button"
                    class=button_class(Variant::Outline, Size::Sm, "")
                    disabled=move || page.get() + 1 >= page_count.get()
                    on:click=move |_| page.update(|p| *p += 1)
                  >
                    {move || t("下一页")}
                  </button>
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

use std::collections::HashMap;

use ham_web_core::adif::parse_adif;
use ham_web_core::dxcc::dxcc_entity;
use ham_web_core::frequencies::band_of;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::ui::{Size, Stat, Variant, button_class, input_class};
use crate::util::{download_text, set_title};

use super::bar_list::BarList;
use super::grid_cell::GridCell;
use super::grid_map::GridMap;
use super::{
  LogEntry, Logbook, MODES, export_adif, export_csv, from_adif, load_logbook, load_station,
  next_id, save_logbook, save_station, utc_now_time, utc_today,
};

const CELL: &str = "border px-3 py-2 text-left align-top";

#[component]
pub fn LogPage() -> impl IntoView {
  set_title("通联日志");

  let logbook = RwSignal::new(load_logbook());
  let station = RwSignal::new(load_station());
  let show_station = RwSignal::new(false);
  let editing = RwSignal::new(None::<u64>);

  let date = RwSignal::new(utc_today());
  let time = RwSignal::new(utc_now_time());
  let freq = RwSignal::new(String::new());
  let mode = RwSignal::new("SSB".to_owned());
  let callsign = RwSignal::new(String::new());
  let rst_sent = RwSignal::new("59".to_owned());
  let rst_rcvd = RwSignal::new(String::new());
  let gridsquare = RwSignal::new(String::new());
  let name = RwSignal::new(String::new());
  let qth = RwSignal::new(String::new());
  let remark = RwSignal::new(String::new());
  let qsl_sent = RwSignal::new(false);
  let qsl_rcvd = RwSignal::new(false);

  let reset_form = move || {
    date.set(utc_today());
    time.set(utc_now_time());
    freq.set(String::new());
    mode.set("SSB".to_owned());
    callsign.set(String::new());
    rst_sent.set("59".to_owned());
    rst_rcvd.set(String::new());
    gridsquare.set(String::new());
    name.set(String::new());
    qth.set(String::new());
    remark.set(String::new());
    qsl_sent.set(false);
    qsl_rcvd.set(false);
    editing.set(None);
  };

  let save = move || {
    let call = callsign.get().trim().to_uppercase();
    if call.is_empty() {
      return;
    }
    let build = |id: u64| LogEntry {
      id,
      date: date.get().trim().to_owned(),
      time: time.get().trim().to_owned(),
      freq: freq.get().trim().to_owned(),
      mode: mode.get(),
      callsign: call.clone(),
      rst: String::new(),
      rst_sent: rst_sent.get().trim().to_owned(),
      rst_rcvd: rst_rcvd.get().trim().to_owned(),
      gridsquare: gridsquare.get().trim().to_uppercase(),
      name: name.get().trim().to_owned(),
      qth: qth.get().trim().to_owned(),
      remark: remark.get().trim().to_owned(),
      qsl_sent: qsl_sent.get(),
      qsl_rcvd: qsl_rcvd.get(),
    };
    if let Some(id) = editing.get() {
      logbook.update(|l| {
        if let Some(e) = l.entries.iter_mut().find(|e| e.id == id) {
          *e = build(id);
        }
      });
    } else {
      logbook.update(|l| {
        let id = next_id(&l.entries);
        l.entries.push(build(id));
      });
    }
    save_logbook(&logbook.get_untracked());
    reset_form();
  };

  let edit = move |e: LogEntry| {
    date.set(e.date);
    time.set(e.time);
    freq.set(e.freq);
    mode.set(e.mode);
    callsign.set(e.callsign);
    rst_sent.set(e.rst_sent);
    rst_rcvd.set(e.rst_rcvd);
    gridsquare.set(e.gridsquare);
    name.set(e.name);
    qth.set(e.qth);
    remark.set(e.remark);
    qsl_sent.set(e.qsl_sent);
    qsl_rcvd.set(e.qsl_rcvd);
    editing.set(Some(e.id));
  };

  let remove = move |id: u64| {
    logbook.update(|l| l.entries.retain(|e| e.id != id));
    save_logbook(&logbook.get_untracked());
  };

  let clear = move || {
    logbook.set(Logbook::default());
    save_logbook(&Logbook::default());
  };

  let export = move || {
    let content = export_adif(&logbook.get_untracked().entries, &station.get_untracked());
    download_text(
      &format!("logbook-{}.adi", utc_today()),
      &content,
      "text/plain",
    );
  };

  let export_csv_btn = move || {
    let content = export_csv(&logbook.get_untracked().entries, &station.get_untracked());
    download_text(
      &format!("logbook-{}.csv", utc_today()),
      &content,
      "text/csv",
    );
  };

  let save_station_btn = move || {
    save_station(&station.get_untracked());
    crate::util::alert("本台信息已保存");
  };

  let import_adif = move |e: web_sys::Event| {
    let Some(target) = e.target() else { return };
    let input: web_sys::HtmlInputElement = target.unchecked_into();
    let Some(file) = input.files().and_then(|f| f.get(0)) else {
      return;
    };
    spawn_local(async move {
      let Some(text) = crate::util::read_file_text(&file).await else {
        return;
      };
      let records = parse_adif(&text);
      let count = records.len();
      logbook.update(|l| {
        for (id, r) in (next_id(&l.entries)..).zip(records) {
          let mut entry = from_adif(r);
          entry.id = id;
          l.entries.push(entry);
        }
      });
      save_logbook(&logbook.get_untracked());
      crate::util::alert(&format!("已导入 {count} 条记录"));
    });
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"通联日志"</div>
            <div class="text-xs text-muted-foreground">"在线记录 · 本地保存 · 导出 ADIF / CSV"</div>
          </div>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Sm, "")
            on:click=move |_| export()
          >
            "导出 ADIF"
          </button>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Sm, "")
            on:click=move |_| export_csv_btn()
          >
            "导出 CSV"
          </button>
          <label class=button_class(Variant::Outline, Size::Sm, "cursor-pointer")>
            "导入 ADIF"
            <input
              type="file"
              accept=".adi,.adif,.txt"
              class="hidden"
              on:change=import_adif
            />
          </label>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        // 本台信息（站点级设置）
        <section class="rounded-xl border bg-card">
          <button
            type="button"
            class="flex w-full items-center justify-between px-4 py-3 text-left text-sm font-semibold"
            on:click=move |_| show_station.update(|v| *v = !*v)
          >
            <span>"本台信息"</span>
            <span class="text-xs font-normal text-muted-foreground">
              {move || {
                let cs = station.get().callsign.clone();
                if cs.is_empty() {
                  "未设置呼号（点击设置）".to_owned()
                } else {
                  cs
                }
              }}
            </span>
          </button>
          {move || {
            show_station.get().then(|| {
              view! {
                <div class="border-t p-4">
                  <p class="mb-3 text-xs text-muted-foreground">
                    "本台信息将写入 ADIF 的 STATION_CALLSIGN / OPERATOR / MY_GRIDSQUARE / MY_RIG / MY_ANTENNA 字段。"
                  </p>
                  <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
                    <label class="flex flex-col gap-1.5 text-sm">
                      <span class="text-xs text-muted-foreground">"本台呼号"</span>
                      <input
                        type="text"
                        placeholder="BG4XXX"
                        class=input_class("uppercase")
                        prop:value=move || station.get().callsign.clone()
                        on:input=move |e| station.update(|s| s.callsign = event_target_value(&e).to_uppercase())
                      />
                    </label>
                    <label class="flex flex-col gap-1.5 text-sm">
                      <span class="text-xs text-muted-foreground">"操作员"</span>
                      <input
                        type="text"
                        placeholder="同呼号"
                        class=input_class("")
                        prop:value=move || station.get().operator.clone()
                        on:input=move |e| station.update(|s| s.operator = event_target_value(&e))
                      />
                    </label>
                    <label class="flex flex-col gap-1.5 text-sm">
                      <span class="text-xs text-muted-foreground">"本台网格"</span>
                      <input
                        type="text"
                        placeholder="OM89EW"
                        class=input_class("uppercase")
                        prop:value=move || station.get().gridsquare.clone()
                        on:input=move |e| station.update(|s| s.gridsquare = event_target_value(&e).to_uppercase())
                      />
                    </label>
                    <label class="flex flex-col gap-1.5 text-sm">
                      <span class="text-xs text-muted-foreground">"设备"</span>
                      <input
                        type="text"
                        placeholder="FT-710"
                        class=input_class("")
                        prop:value=move || station.get().rig.clone()
                        on:input=move |e| station.update(|s| s.rig = event_target_value(&e))
                      />
                    </label>
                    <label class="flex flex-col gap-1.5 text-sm">
                      <span class="text-xs text-muted-foreground">"天线"</span>
                      <input
                        type="text"
                        placeholder="DP 20m"
                        class=input_class("")
                        prop:value=move || station.get().antenna.clone()
                        on:input=move |e| station.update(|s| s.antenna = event_target_value(&e))
                      />
                    </label>
                  </div>
                  <div class="mt-3">
                    <button
                      type="button"
                      class=button_class(Variant::Default, Size::Sm, "")
                      on:click=move |_| save_station_btn()
                    >
                      "保存本台信息"
                    </button>
                  </div>
                </div>
              }
            })
          }}
        </section>

        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label="通联总数" value=move || logbook.get().entries.len() />
        </div>

        {move || {
          let entries = logbook.get().entries;
          if entries.is_empty() {
            return view! { <div></div> }.into_any();
          }
          let mut by_mode: HashMap<String, usize> = HashMap::new();
          let mut by_band: HashMap<String, usize> = HashMap::new();
          let mut dxcc: Vec<String> = Vec::new();
          let mut grids: Vec<String> = Vec::new();
          for e in &entries {
            *by_mode.entry(e.mode.clone()).or_default() += 1;
            if let Ok(f) = e.freq.trim().parse::<f64>() {
              *by_band.entry(band_of(f).to_owned()).or_default() += 1;
            }
            if let Some(entity) = dxcc_entity(&e.callsign)
              && !dxcc.contains(&entity.to_owned())
            {
              dxcc.push(entity.to_owned());
            }
            if !e.gridsquare.is_empty() && !grids.contains(&e.gridsquare) {
              grids.push(e.gridsquare.clone());
            }
          }
          let mut modes: Vec<(String, usize)> = by_mode.into_iter().collect();
          let mut bands: Vec<(String, usize)> = by_band.into_iter().collect();
          modes.sort_by_key(|a| std::cmp::Reverse(a.1));
          bands.sort_by_key(|a| std::cmp::Reverse(a.1));
          dxcc.sort();
          grids.sort();
          view! {
            <section class="grid gap-4 sm:grid-cols-2">
              <div class="rounded-xl border bg-card p-4">
                <h3 class="mb-3 text-sm font-semibold">"按模式"</h3>
                <BarList items=modes />
              </div>
              <div class="rounded-xl border bg-card p-4">
                <h3 class="mb-3 text-sm font-semibold">"按波段"</h3>
                <BarList items=bands />
              </div>
            </section>
            <section class="rounded-xl border bg-card p-4">
              <h3 class="mb-2 text-sm font-semibold">
                "DXCC 实体"
                <span class="ml-2 text-xs font-normal text-muted-foreground">
                  "已通联 " {dxcc.len()} " 个"
                </span>
              </h3>
              {if dxcc.is_empty() {
                view! { <div class="text-xs text-muted-foreground">"暂无已识别实体，添加通联记录后自动统计。"</div> }.into_any()
              } else {
                view! {
                  <div class="flex flex-wrap gap-1.5">
                    {dxcc
                      .into_iter()
                      .map(|e| {
                        view! {
                          <span class="rounded-full border bg-muted/40 px-2.5 py-0.5 text-xs">{e}</span>
                        }
                      })
                      .collect_view()}
                  </div>
                }
                .into_any()
              }}
            </section>
            <section class="rounded-xl border bg-card p-4">
              <h3 class="mb-2 text-sm font-semibold">
                "已通联网格"
                <span class="ml-2 text-xs font-normal text-muted-foreground">
                  {grids.len()} " 个"
                </span>
              </h3>
              {if grids.is_empty() {
                view! { <div class="text-xs text-muted-foreground">"暂无网格记录，添加带网格的通联后自动统计。"</div> }.into_any()
              } else {
                view! {
                  <div class="space-y-3">
                    <GridMap grids=grids.clone() highlight=Signal::derive(|| None) />
                    <div class="flex flex-wrap gap-1.5">
                      {grids
                        .into_iter()
                        .map(|g| {
                          view! {
                            <span class="rounded-full border bg-muted/40 px-2.5 py-0.5 font-mono text-xs">{g}</span>
                          }
                        })
                        .collect_view()}
                    </div>
                  </div>
                }
                .into_any()
              }}
            </section>
          }
          .into_any()
        }}

        // 新增记录表单
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">
            {move || if editing.get().is_some() { "编辑通联记录" } else { "添加通联记录" }}
          </h2>
          <div class="grid gap-3 p-4 sm:grid-cols-2 lg:grid-cols-3">
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"日期（UTC）"</span>
              <input
                type="date"
                prop:value=move || date.get()
                on:input=move |e| date.set(event_target_value(&e))
                class=input_class("")
              />
            </label>
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"时间（UTC）"</span>
              <input
                type="time"
                prop:value=move || time.get()
                on:input=move |e| time.set(event_target_value(&e))
                class=input_class("")
              />
            </label>
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"频率（MHz）"</span>
              <input
                type="text"
                placeholder="14.074"
                prop:value=move || freq.get()
                on:input=move |e| freq.set(event_target_value(&e))
                class=input_class("")
              />
            </label>
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"模式"</span>
              <select
                prop:value=move || mode.get()
                on:change=move |e| mode.set(event_target_value(&e))
                class=input_class("")
              >
                {MODES
                  .iter()
                  .map(|&m| {
                    view! { <option value=m>{m}</option> }
                  })
                  .collect_view()}
              </select>
            </label>
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"对方呼号 *"</span>
              <input
                type="text"
                placeholder="BG4XXX"
                prop:value=move || callsign.get()
                on:input=move |e| callsign.set(event_target_value(&e))
                class=input_class("uppercase")
              />
            </label>
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"信号报告（发 RST_SENT）"</span>
              <input
                type="text"
                placeholder="59 / 599"
                prop:value=move || rst_sent.get()
                on:input=move |e| rst_sent.set(event_target_value(&e))
                class=input_class("")
              />
            </label>
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"信号报告（收 RST_RCVD）"</span>
              <input
                type="text"
                placeholder="59 / 599"
                prop:value=move || rst_rcvd.get()
                on:input=move |e| rst_rcvd.set(event_target_value(&e))
                class=input_class("")
              />
            </label>
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"对方网格"</span>
              <input
                type="text"
                placeholder="OM89EW"
                prop:value=move || gridsquare.get()
                on:input=move |e| gridsquare.set(event_target_value(&e))
                class=input_class("uppercase")
              />
            </label>
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"对方姓名"</span>
              <input
                type="text"
                prop:value=move || name.get()
                on:input=move |e| name.set(event_target_value(&e))
                class=input_class("")
              />
            </label>
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"对方 QTH"</span>
              <input
                type="text"
                prop:value=move || qth.get()
                on:input=move |e| qth.set(event_target_value(&e))
                class=input_class("")
              />
            </label>
            <label class="flex flex-col gap-1.5 text-sm sm:col-span-2 lg:col-span-2">
              <span class="text-xs text-muted-foreground">"备注"</span>
              <input
                type="text"
                prop:value=move || remark.get()
                on:input=move |e| remark.set(event_target_value(&e))
                class=input_class("")
              />
            </label>
            <label class="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                prop:checked=move || qsl_sent.get()
                on:change=move |e| qsl_sent.set(event_target_checked(&e))
                class="size-4 accent-primary"
              />
              <span class="text-xs text-muted-foreground">"QSL 已寄出"</span>
            </label>
            <label class="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                prop:checked=move || qsl_rcvd.get()
                on:change=move |e| qsl_rcvd.set(event_target_checked(&e))
                class="size-4 accent-primary"
              />
              <span class="text-xs text-muted-foreground">"QSL 已收到"</span>
            </label>
          </div>
          <div class="flex items-center gap-2 px-4 pb-4">
            <button type="button" class=button_class(Variant::Default, Size::Default, "") on:click=move |_| save()>
              {move || if editing.get().is_some() { "保存修改" } else { "添加记录" }}
            </button>
            {move || {
              editing.get().is_some().then(|| {
                view! {
                  <button
                    type="button"
                    class=button_class(Variant::Ghost, Size::Default, "")
                    on:click=move |_| reset_form()
                  >
                    "取消"
                  </button>
                }
              })
            }}
          </div>
        </section>

        // 日志列表
        <section class="rounded-xl border bg-card">
          <div class="flex items-center justify-between border-b px-4 py-3">
            <h2 class="text-sm font-semibold">"记录列表"</h2>
            <button
              type="button"
              class=button_class(Variant::Ghost, Size::Sm, "text-muted-foreground")
              on:click=move |_| clear()
            >
              "清空"
            </button>
          </div>

          {move || {
            let entries = logbook.get();
            if entries.entries.is_empty() {
              view! {
                <div class="px-4 py-10 text-center text-sm text-muted-foreground">
                  "暂无记录，添加第一条通联日志吧。"
                </div>
              }
              .into_any()
            } else {
              view! {
                <div class="overflow-x-auto">
                  <table class="w-full min-w-[880px] border-collapse text-sm">
                    <thead class="bg-muted/60 text-xs">
                      <tr>
                        <th class=CELL>"日期"</th>
                        <th class=CELL>"时间"</th>
                        <th class=CELL>"频率"</th>
                        <th class=CELL>"模式"</th>
                        <th class=CELL>"呼号"</th>
                        <th class=CELL>"RST 发/收"</th>
                        <th class=CELL>"网格"</th>
                        <th class=CELL>"备注"</th>
                        <th class=CELL>"QSL"</th>
                        <th class=CELL>"操作"</th>
                      </tr>
                    </thead>
                    <tbody>
                      {entries
                        .entries
                        .iter()
                        .map(|e| {
                          let entry = e.clone();
                          let id = e.id;
                          let rst = if e.rst_sent.is_empty() && e.rst_rcvd.is_empty() {
                            "—".to_owned()
                          } else {
                            format!("{} / {}", e.rst_sent, e.rst_rcvd)
                          };
                          view! {
                            <tr class="border-t transition-colors hover:bg-muted/40">
                              <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{e.date.clone()}</td>
                              <td class=format!("{CELL} whitespace-nowrap tabular-nums")>{e.time.clone()}</td>
                              <td class=format!("{CELL} whitespace-nowrap font-mono tabular-nums")>{e.freq.clone()}</td>
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
                              <td class=format!("{CELL} text-muted-foreground")>{e.remark.clone()}</td>
                              <td class=format!("{CELL} whitespace-nowrap")>
                                {if e.qsl_rcvd {
                                  view! {
                                    <span class="font-medium text-emerald-600 dark:text-emerald-400">
                                      "已确认"
                                    </span>
                                  }
                                  .into_any()
                                } else if e.qsl_sent {
                                  view! {
                                    <span class="text-amber-600 dark:text-amber-400">"已寄出"</span>
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
                                    on:click=move |_| edit(entry.clone())
                                  >
                                    "编辑"
                                  </button>
                                  <button
                                    type="button"
                                    class="text-xs text-muted-foreground transition-colors hover:text-destructive"
                                    on:click=move |_| remove(id)
                                  >
                                    "删除"
                                  </button>
                                </div>
                              </td>
                            </tr>
                          }
                        })
                        .collect_view()}
                    </tbody>
                  </table>
                </div>
              }
              .into_any()
            }
          }}
        </section>
      </div>
    </div>
  }
}

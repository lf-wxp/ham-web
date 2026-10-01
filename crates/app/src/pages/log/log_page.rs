use std::collections::{HashMap, HashSet};

use ham_web_core::adif::parse_adif;
use ham_web_core::dxcc::lookup;
use ham_web_core::logbook::{MODES, call_hint, export_adif, export_csv, path_to};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::components::cat_control::{CatControl, CatReading};
use crate::ui::{Size, Stat, Variant, button_class, input_class};
use crate::util::download_text;
use crate::util::set_title;

use super::awards_panel::AwardsPanel;
use super::bar_list::BarList;
use super::grid_cell::GridCell;
use super::grid_map::GridMap;
use super::{LogEntry, Logbook, use_log_store, utc_now_time, utc_today};

use super::log_helpers::{CELL, PAGE_SIZE, PROP_MODES, compass, confirm};

#[component]
pub fn LogPage() -> impl IntoView {
  set_title("通联日志");

  let store = use_log_store();
  let logbook = store.logbook;
  let station = store.station;
  let show_station = RwSignal::new(false);
  let show_more = RwSignal::new(false);
  let editing = RwSignal::new(None::<u64>);

  let date = RwSignal::new(utc_today());
  let time = RwSignal::new(utc_now_time());
  let time_off = RwSignal::new(String::new());
  let freq = RwSignal::new(String::new());
  let band = RwSignal::new(String::new());
  let mode = RwSignal::new("SSB".to_owned());
  let callsign = RwSignal::new(String::new());
  let rst_sent = RwSignal::new("59".to_owned());
  let rst_rcvd = RwSignal::new(String::new());
  let tx_pwr = RwSignal::new(String::new());
  let gridsquare = RwSignal::new(String::new());
  let name = RwSignal::new(String::new());
  let qth = RwSignal::new(String::new());
  let prop_mode = RwSignal::new(String::new());
  let sat_name = RwSignal::new(String::new());
  let sota_ref = RwSignal::new(String::new());
  let pota_ref = RwSignal::new(String::new());
  let cqz = RwSignal::new(String::new());
  let ituz = RwSignal::new(String::new());
  let remark = RwSignal::new(String::new());
  let qsl_sent = RwSignal::new(false);
  let qsl_rcvd = RwSignal::new(false);

  // 列表筛选与分页
  let query = RwSignal::new(String::new());
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

  let reset_form = move || {
    date.set(utc_today());
    time.set(utc_now_time());
    time_off.set(String::new());
    freq.set(String::new());
    band.set(String::new());
    mode.set("SSB".to_owned());
    callsign.set(String::new());
    rst_sent.set("59".to_owned());
    rst_rcvd.set(String::new());
    tx_pwr.set(String::new());
    gridsquare.set(String::new());
    name.set(String::new());
    qth.set(String::new());
    prop_mode.set(String::new());
    sat_name.set(String::new());
    sota_ref.set(String::new());
    pota_ref.set(String::new());
    cqz.set(String::new());
    ituz.set(String::new());
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
    // 编辑时呼号未变则保留原 DXCC（可能来自导入的 ADIF，比前缀推断更准）；呼号改了则
    // 重新推断 DXCC，未手动改过的分区也随之重新推断。
    let build = |id: u64, old: Option<&LogEntry>| {
      let same_call = old.filter(|o| o.callsign.eq_ignore_ascii_case(&call));
      let renamed = old.filter(|o| !o.callsign.eq_ignore_ascii_case(&call));
      let zone = |sig: RwSignal<String>, prev: Option<&String>| {
        let v = sig.get().trim().to_owned();
        if prev.is_some_and(|p| *p == v) {
          String::new()
        } else {
          v
        }
      };
      LogEntry {
        id,
        dxcc: same_call.map(|o| o.dxcc.clone()).unwrap_or_default(),
        cqz: zone(cqz, renamed.map(|o| &o.cqz)),
        ituz: zone(ituz, renamed.map(|o| &o.ituz)),
        date: date.get().trim().to_owned(),
        time: time.get().trim().to_owned(),
        time_off: time_off.get().trim().to_owned(),
        freq: freq.get().trim().to_owned(),
        band: band.get(),
        mode: mode.get(),
        callsign: call.clone(),
        rst_sent: rst_sent.get().trim().to_owned(),
        rst_rcvd: rst_rcvd.get().trim().to_owned(),
        tx_pwr: tx_pwr.get().trim().to_owned(),
        gridsquare: gridsquare.get().trim().to_uppercase(),
        name: name.get().trim().to_owned(),
        qth: qth.get().trim().to_owned(),
        prop_mode: prop_mode.get(),
        sat_name: sat_name.get().trim().to_uppercase(),
        sota_ref: sota_ref.get().trim().to_uppercase(),
        pota_ref: pota_ref.get().trim().to_uppercase(),
        remark: remark.get().trim().to_owned(),
        qsl_sent: qsl_sent.get(),
        qsl_rcvd: qsl_rcvd.get(),
        ..Default::default()
      }
    };
    if let Some(id) = editing.get() {
      logbook.update(|l| {
        if let Some(e) = l.entries.iter_mut().find(|e| e.id == id) {
          let mut next = build(id, Some(&*e));
          next.fill_location();
          *e = next;
        }
      });
    } else {
      logbook.update(|l| {
        let mut next = build(l.next_id(), None);
        next.fill_location();
        l.entries.push(next);
      });
    }
    store.persist();
    reset_form();
  };

  let edit = move |e: LogEntry| {
    let has_more = !(e.time_off.is_empty()
      && e.tx_pwr.is_empty()
      && e.prop_mode.is_empty()
      && e.sat_name.is_empty()
      && e.sota_ref.is_empty()
      && e.pota_ref.is_empty());
    date.set(e.date);
    time.set(e.time);
    time_off.set(e.time_off);
    freq.set(e.freq);
    band.set(e.band);
    mode.set(e.mode);
    callsign.set(e.callsign);
    rst_sent.set(e.rst_sent);
    rst_rcvd.set(e.rst_rcvd);
    tx_pwr.set(e.tx_pwr);
    gridsquare.set(e.gridsquare);
    name.set(e.name);
    qth.set(e.qth);
    prop_mode.set(e.prop_mode);
    sat_name.set(e.sat_name);
    sota_ref.set(e.sota_ref);
    pota_ref.set(e.pota_ref);
    cqz.set(e.cqz);
    ituz.set(e.ituz);
    remark.set(e.remark);
    qsl_sent.set(e.qsl_sent);
    qsl_rcvd.set(e.qsl_rcvd);
    if has_more {
      show_more.set(true);
    }
    editing.set(Some(e.id));
  };

  let remove = move |id: u64| {
    logbook.update(|l| l.entries.retain(|e| e.id != id));
    store.persist();
  };

  let clear = move || {
    let n = logbook.with_untracked(|l| l.entries.len());
    if confirm(&format!(
      "确定清空全部 {n} 条通联记录吗？建议先导出 ADIF 备份。此操作不可撤销。"
    )) {
      logbook.set(Logbook::default());
      store.persist();
    }
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
    store.persist_station();
    crate::util::alert("本台信息已保存");
  };

  let import_adif = move |e: web_sys::Event| {
    let Some(target) = e.target() else { return };
    let input: web_sys::HtmlInputElement = target.unchecked_into();
    let Some(file) = input.files().and_then(|f| f.get(0)) else {
      return;
    };
    input.set_value("");
    spawn_local(async move {
      let Some(text) = crate::util::read_file_text(&file).await else {
        return;
      };
      let (added, dupes) = store.import(parse_adif(&text));
      let msg = if dupes > 0 {
        format!("已导入 {added} 条记录，跳过重复 {dupes} 条")
      } else {
        format!("已导入 {added} 条记录")
      };
      crate::util::alert(&msg);
    });
  };

  let hint = Memo::new(move |_| {
    let call = callsign.get().trim().to_uppercase();
    if call.len() < 3 {
      return None;
    }
    let draft = LogEntry {
      freq: freq.get(),
      band: band.get(),
      ..Default::default()
    };
    let (m, skip) = (mode.get(), editing.get());
    let h = logbook.with(|lb| call_hint(&lb.entries, &call, &draft.band_label(), &m, skip));
    let path = station.with(|s| path_to(&s.gridsquare, &gridsquare.get(), &call));
    Some((h, path))
  });
  // 自动推断的分区，作为输入框占位提示。
  let auto_zones = Memo::new(move |_| lookup(callsign.get().trim()).map(|e| (e.cq, e.itu)));

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

  let text_input = move |label: &'static str,
                         placeholder: &'static str,
                         sig: RwSignal<String>,
                         extra: &'static str| {
    view! {
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{label}</span>
        <input
          type="text"
          placeholder=placeholder
          prop:value=move || sig.get()
          on:input=move |e| sig.set(event_target_value(&e))
          class=input_class(extra)
        />
      </label>
    }
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"通联日志"</h1>
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
          <a href="/qsl-labels" class=button_class(Variant::Outline, Size::Sm, "")>"打印 QSL 标签"</a>
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
                    "本台信息将写入每条 ADIF 记录的 STATION_CALLSIGN / OPERATOR / MY_GRIDSQUARE / MY_RIG / MY_ANTENNA 字段。"
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

        {move || {
          let entries = logbook.get().entries;
          if entries.is_empty() {
            return view! { <div></div> }.into_any();
          }
          let mut by_mode: HashMap<String, usize> = HashMap::new();
          let mut by_band: HashMap<String, usize> = HashMap::new();
          let mut dxcc: Vec<String> = Vec::new();
          let mut grids: Vec<String> = Vec::new();
          let mut calls: HashSet<String> = HashSet::new();
          let confirmed = entries.iter().filter(|e| e.qsl_rcvd).count();
          for e in &entries {
            *by_mode.entry(e.mode.clone()).or_default() += 1;
            let b = e.band_label();
            if !b.is_empty() {
              *by_band.entry(b).or_default() += 1;
            }
            if let Some(entity) = e.entity()
              && !dxcc.iter().any(|d| d == entity.name)
            {
              dxcc.push(entity.name.to_owned());
            }
            if !e.gridsquare.is_empty() && !grids.contains(&e.gridsquare) {
              grids.push(e.gridsquare.clone());
            }
            calls.insert(e.callsign.to_uppercase());
          }
          let mut modes: Vec<(String, usize)> = by_mode.into_iter().collect();
          let mut bands: Vec<(String, usize)> = by_band.into_iter().collect();
          modes.sort_by_key(|a| std::cmp::Reverse(a.1));
          bands.sort_by_key(|a| std::cmp::Reverse(a.1));
          dxcc.sort();
          grids.sort();
          let (total, n_calls, n_dxcc) = (entries.len(), calls.len(), dxcc.len());
          view! {
            <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
              <Stat label="通联总数" value=move || total />
              <Stat label="不同呼号" value=move || n_calls />
              <Stat label="DXCC 实体" value=move || n_dxcc />
              <Stat label="QSL 已确认" value=move || confirmed />
            </div>
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
                  "已通联 " {n_dxcc} " 个"
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
            <AwardsPanel entries=entries.clone() />
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
                    <GridMap entries=entries.clone() station_grid=station.get_untracked().gridsquare.clone() />
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
          <div class="px-4 pt-4">
            <CatControl on_reading=move |r: CatReading| {
              if let Some(f) = r.freq {
                freq.set(f);
              }
              if let Some(m) = r.mode {
                mode.set(m.to_owned());
              }
            } />
          </div>
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
            {text_input("频率（MHz）", "14.074", freq, "")}
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">"模式"</span>
              <select
                prop:value=move || mode.get()
                on:change=move |e| mode.set(event_target_value(&e))
                class=input_class("")
              >
                {move || {
                  let current = mode.get();
                  let mut list: Vec<String> = MODES.iter().map(|m| (*m).to_owned()).collect();
                  if !current.is_empty() && !list.contains(&current) {
                    list.push(current);
                  }
                  list
                    .into_iter()
                    .map(|m| view! { <option value=m.clone()>{m.clone()}</option> })
                    .collect_view()
                }}
              </select>
            </label>
            {text_input("对方呼号 *", "BG4XXX", callsign, "uppercase")}
            {text_input("信号报告（发 RST_SENT）", "59 / 599", rst_sent, "")}
            {move || {
              hint.get().map(|(h, path)| {
                let entity = h.entity.map_or_else(
                  || "未识别 DXCC 实体".to_owned(),
                  |e| format!("{}（{} · CQ {} · ITU {}）", e.name, e.continent, e.cq, e.itu),
                );
                view! {
                  <div class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg bg-muted/40 px-3 py-2 text-xs sm:col-span-2 lg:col-span-3">
                    <span class="text-muted-foreground">{entity}</span>
                    {path.map(|p| view! {
                      <span class="tabular-nums text-muted-foreground" title=if p.approximate { "未填对方网格，按 DXCC 实体中心估算" } else { "按双方网格计算" }>
                        {format!(
                          "{}{:.0} km · 方位 {:.0}°（{}）",
                          if p.approximate { "约 " } else { "" },
                          p.km,
                          p.bearing,
                          compass(p.bearing)
                        )}
                      </span>
                    })}
                    {h.new_dxcc.then(|| view! {
                      <span class="rounded-full bg-emerald-500/15 px-2 py-0.5 font-medium text-emerald-700 dark:text-emerald-300">"新 DXCC！"</span>
                    })}
                    {h.new_band.then(|| view! {
                      <span class="rounded-full bg-sky-500/15 px-2 py-0.5 font-medium text-sky-700 dark:text-sky-300">"该实体新波段"</span>
                    })}
                    {h.dupe.then(|| view! {
                      <span class="rounded-full bg-amber-500/15 px-2 py-0.5 font-medium text-amber-700 dark:text-amber-300">"重复：同波段同模式已联过"</span>
                    })}
                    <span class="text-muted-foreground">
                      {match (h.worked, h.last.clone()) {
                        (0, _) | (_, None) => "首次通联该呼号".to_owned(),
                        (n, Some(last)) => format!("已通联 {n} 次，最近 {last}"),
                      }}
                    </span>
                  </div>
                }
              })
            }}
            {text_input("信号报告（收 RST_RCVD）", "59 / 599", rst_rcvd, "")}
            {text_input("对方网格", "OM89EW", gridsquare, "uppercase")}
            {text_input("对方姓名", "", name, "")}
            {text_input("对方 QTH", "", qth, "")}
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
            <button
              type="button"
              class="text-left text-xs text-primary underline-offset-4 hover:underline sm:col-span-2 lg:col-span-3"
              on:click=move |_| show_more.update(|v| *v = !*v)
            >
              {move || if show_more.get() { "收起更多字段" } else { "更多字段：功率、结束时间、卫星 / 传播方式、SOTA / POTA、CQ / ITU 分区" }}
            </button>
            {move || {
              show_more.get().then(|| {
                view! {
                  {text_input("功率（W，TX_PWR）", "100", tx_pwr, "")}
                  <label class="flex flex-col gap-1.5 text-sm">
                    <span class="text-xs text-muted-foreground">"结束时间（UTC）"</span>
                    <input
                      type="time"
                      prop:value=move || time_off.get()
                      on:input=move |e| time_off.set(event_target_value(&e))
                      class=input_class("")
                    />
                  </label>
                  <label class="flex flex-col gap-1.5 text-sm">
                    <span class="text-xs text-muted-foreground">"传播方式（PROP_MODE）"</span>
                    <select
                      prop:value=move || prop_mode.get()
                      on:change=move |e| prop_mode.set(event_target_value(&e))
                      class=input_class("")
                    >
                      {PROP_MODES
                        .iter()
                        .map(|&(v, l)| view! { <option value=v>{l}</option> })
                        .collect_view()}
                    </select>
                  </label>
                  {text_input("卫星（SAT_NAME）", "SO-50 / RS-44", sat_name, "uppercase")}
                  {text_input("SOTA 编号", "BY/BJ-001", sota_ref, "uppercase")}
                  {text_input("POTA 编号", "CN-0001", pota_ref, "uppercase")}
                  <label class="flex flex-col gap-1.5 text-sm">
                    <span class="text-xs text-muted-foreground">"CQ 分区（留空自动）"</span>
                    <input
                      type="number"
                      min="1"
                      max="40"
                      placeholder=move || auto_zones.get().map(|(cq, _)| cq.to_string()).unwrap_or_default()
                      prop:value=move || cqz.get()
                      on:input=move |e| cqz.set(event_target_value(&e))
                      class=input_class("")
                    />
                  </label>
                  <label class="flex flex-col gap-1.5 text-sm">
                    <span class="text-xs text-muted-foreground">"ITU 分区（留空自动）"</span>
                    <input
                      type="number"
                      min="1"
                      max="90"
                      placeholder=move || auto_zones.get().map(|(_, itu)| itu.to_string()).unwrap_or_default()
                      prop:value=move || ituz.get()
                      on:input=move |e| ituz.set(event_target_value(&e))
                      class=input_class("")
                    />
                  </label>
                }
              })
            }}
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
            <h2 class="text-sm font-semibold">
              "记录列表"
              <span class="ml-2 text-xs font-normal text-muted-foreground">
                {move || {
                  let (n, total) = (filtered.with(Vec::len), logbook.with(|l| l.entries.len()));
                  if n == total { format!("共 {total} 条") } else { format!("筛选出 {n} / {total} 条") }
                }}
              </span>
            </h2>
            <button
              type="button"
              class=button_class(Variant::Ghost, Size::Sm, "text-muted-foreground")
              on:click=move |_| clear()
            >
              "清空"
            </button>
          </div>

          {move || {
            if logbook.with(|l| l.entries.is_empty()) {
              return view! {
                <div class="px-4 py-10 text-center text-sm text-muted-foreground">
                  "暂无记录，添加第一条通联日志吧。"
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
                  placeholder="搜索呼号 / 姓名 / QTH / 网格 / 备注"
                  prop:value=move || query.get()
                  on:input=move |e| query.set(event_target_value(&e))
                  class=input_class("min-w-48 flex-1")
                />
                <select prop:value=move || band_filter.get() on:change=move |e| band_filter.set(event_target_value(&e)) class=input_class("w-28")>
                  <option value="">"全部波段"</option>
                  {bands.into_iter().map(|b| view! { <option value=b.clone()>{b.clone()}</option> }).collect_view()}
                </select>
                <select prop:value=move || mode_filter.get() on:change=move |e| mode_filter.set(event_target_value(&e)) class=input_class("w-28")>
                  <option value="">"全部模式"</option>
                  {modes.into_iter().map(|m| view! { <option value=m.clone()>{m.clone()}</option> }).collect_view()}
                </select>
                <select prop:value=move || qsl_filter.get() on:change=move |e| qsl_filter.set(event_target_value(&e)) class=input_class("w-28")>
                  <option value="">"全部 QSL"</option>
                  <option value="pending">"未确认"</option>
                  <option value="rcvd">"已确认"</option>
                </select>
              </div>
              <div class="overflow-x-auto">
                <table class="w-full min-w-[880px] border-collapse text-sm">
                  <thead class="bg-muted/60 text-xs">
                    <tr>
                      <th class=CELL>"日期"</th>
                      <th class=CELL>"时间"</th>
                      <th class=CELL>"频率 / 波段"</th>
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
                        "上一页"
                      </button>
                      <span class="tabular-nums">{move || format!("第 {} / {} 页", page.get() + 1, page_count.get())}</span>
                      <button
                        type="button"
                        class=button_class(Variant::Outline, Size::Sm, "")
                        disabled=move || page.get() + 1 >= page_count.get()
                        on:click=move |_| page.update(|p| *p += 1)
                      >
                        "下一页"
                      </button>
                    </div>
                  }
                })
              }}
            }
            .into_any()
          }}
        </section>
      </div>
    </div>
  }
}

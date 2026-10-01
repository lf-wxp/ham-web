use std::collections::BTreeMap;

use ham_web_core::contest::{
  CONTESTS, CabrilloHeader, Exch, cabrillo, default_rst, is_dupe, score,
};
use ham_web_core::dxcc::lookup;
use leptos::html;
use leptos::prelude::*;

use crate::components::cat_control::{CatControl, CatReading};
use crate::pages::log::{LogEntry, use_log_store, utc_now_time, utc_today};
use crate::ui::{Size, Stat, Variant, button_class, input_class};
use crate::util::{download_text, set_title, storage};

use super::session::Session;

const SESSION_KEY: &str = "contest-session";
const MODES: &[&str] = &["CW", "SSB", "FT8", "RTTY"];

#[component]
pub fn ContestLogPage() -> impl IntoView {
  set_title("竞赛录入");
  let store = use_log_store();
  let session = RwSignal::new(storage::get_json::<Session>(SESSION_KEY).unwrap_or_default());
  let save = move || storage::set_json(SESSION_KEY, &session.get_untracked());
  let my_call = Memo::new(move |_| {
    store
      .station
      .with(|s| s.callsign.trim().to_ascii_uppercase())
  });

  // 首次使用：按本台呼号填入默认交换
  if session.with_untracked(|s| s.my_exch.is_empty()) {
    let zone = lookup(&my_call.get_untracked()).map(|e| e.cq.to_string());
    session.update(|s| {
      s.my_exch = match s.def().sent {
        Exch::CqZone => zone.unwrap_or_default(),
        Exch::Power => "100".to_owned(),
        _ => String::new(),
      };
    });
  }

  let entries = Memo::new(move |_| {
    let s = session.get();
    store.logbook.with(|lb| {
      lb.entries
        .iter()
        .filter(|e| s.includes(e))
        .cloned()
        .collect::<Vec<_>>()
    })
  });
  let tally = Memo::new(move |_| {
    let def = session.with(Session::def);
    entries.with(|list| score(def, list, &my_call.get()))
  });

  let call = RwSignal::new(String::new());
  let rcvd = RwSignal::new(String::new());
  let call_ref = NodeRef::<html::Input>::new();
  let rcvd_ref = NodeRef::<html::Input>::new();
  let message = RwSignal::new(None::<String>);
  let focus = |r: NodeRef<html::Input>| {
    if let Some(el) = r.get_untracked() {
      let _ = el.focus();
    }
  };

  let sent_exch = move || {
    let s = session.get();
    if s.def().sent == Exch::Serial {
      format!("{:03}", entries.with(|e| e.len()) + 1)
    } else {
      s.my_exch.clone()
    }
  };
  let draft = move || {
    let s = session.get_untracked();
    let mode = s.mode();
    LogEntry {
      id: store.logbook.with_untracked(|lb| lb.next_id()),
      date: utc_today(),
      time: utc_now_time(),
      freq: s.freq.trim().to_owned(),
      mode: mode.clone(),
      callsign: call.get_untracked().trim().to_ascii_uppercase(),
      rst_sent: default_rst(&mode).to_owned(),
      rst_rcvd: default_rst(&mode).to_owned(),
      contest_id: s.contest_id.clone(),
      stx: sent_exch(),
      srx: rcvd.get_untracked().trim().to_ascii_uppercase(),
      ..Default::default()
    }
  };
  let dupe = Memo::new(move |_| {
    let c = call.get();
    if c.trim().len() < 3 {
      return false;
    }
    let def = session.with(Session::def);
    let candidate = LogEntry {
      callsign: c,
      freq: session.with(|s| s.freq.clone()),
      mode: session.with(Session::mode),
      ..Default::default()
    };
    entries.with(|list| is_dupe(def, list, &candidate))
  });
  let hint = move || {
    let c = call.get();
    let en = lookup(c.trim())?;
    Some(format!("{} · {} · CQ {}", en.name, en.continent, en.cq))
  };
  let rcvd_placeholder = move || {
    let def = session.with(Session::def);
    if def.rcvd == Exch::CqZone
      && let Some(en) = lookup(call.get().trim())
    {
      return en.cq.to_string();
    }
    def.rcvd.placeholder().to_owned()
  };

  let log_it = move || {
    if call.with_untracked(|c| c.trim().len() < 3) {
      focus(call_ref);
      return;
    }
    let def = session.with_untracked(Session::def);
    if rcvd.with_untracked(|r| r.trim().is_empty()) {
      // CQ WW 未填分区时用对方实体的主分区
      if def.rcvd == Exch::CqZone
        && let Some(en) = lookup(call.get_untracked().trim())
      {
        rcvd.set(en.cq.to_string());
      } else if def.rcvd != Exch::Free {
        focus(rcvd_ref);
        return;
      }
    }
    let mut entry = draft();
    entry.fill_location();
    let was_dupe = dupe.get_untracked();
    let label = format!("{} {}", entry.callsign, entry.srx);
    store.logbook.update(|lb| lb.entries.push(entry));
    store.persist();
    message.set(Some(if was_dupe {
      format!("已记录 {label}（重复，不计分）")
    } else {
      format!("已记录 {label}")
    }));
    call.set(String::new());
    rcvd.set(String::new());
    focus(call_ref);
  };
  let remove = move |id: u64| {
    store.logbook.update(|lb| lb.entries.retain(|e| e.id != id));
    store.persist();
  };
  let restart = move || {
    session.update(|s| s.start = format!("{} {}", utc_today(), utc_now_time()));
    save();
    message.set(Some(
      "已开始新一场，之前的通联仍保留在通联日志里".to_owned(),
    ));
  };
  let export = move || {
    let s = session.get_untracked();
    let st = store.station.get_untracked();
    let header = CabrilloHeader {
      callsign: st.callsign.clone(),
      operators: st.operator.clone(),
      grid: st.gridsquare.clone(),
      power: s.power.clone(),
      ..Default::default()
    };
    let text = entries.with_untracked(|list| cabrillo(s.def(), &header, list));
    let name = format!(
      "{}-{}.cbr",
      s.contest_id,
      if st.callsign.is_empty() {
        "log"
      } else {
        st.callsign.trim()
      }
    );
    download_text(&name, &text, "text/plain");
  };

  let per_band = Memo::new(move |_| {
    let mut m: BTreeMap<String, usize> = BTreeMap::new();
    entries.with(|list| {
      for e in list {
        *m.entry(e.band_label()).or_default() += 1;
      }
    });
    m
  });

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"竞赛录入"</h1>
            <div class="text-xs text-muted-foreground">"键盘快速录入 · 实时查重 · 自报分数 · 导出 Cabrillo"</div>
          </div>
          <a href="/log" class=button_class(Variant::Outline, Size::Sm, "")>"通联日志"</a>
          <button type="button" class=button_class(Variant::Default, Size::Sm, "") on:click=move |_| export()>
            "导出 Cabrillo"
          </button>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-5 px-4 py-5">
        {move || my_call.get().is_empty().then(|| view! {
          <p class="rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm">
            "还没有设置本台呼号，记分与 Cabrillo 需要它。请先到 "
            <a href="/log" class="font-medium underline underline-offset-4">"通联日志 → 本台信息"</a>
            " 填写。"
          </p>
        })}

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"竞赛设置"</h2>
          <div class="px-4 pt-4">
            <CatControl on_reading=move |r: CatReading| {
              // 电台的数据模式在竞赛里通常就是 FT8
              let m = r.mode.map(|m| if m == "DATA" { "FT8" } else { m }).filter(|m| MODES.contains(m));
              session.update(|s| {
                if let Some(f) = r.freq {
                  s.freq = f;
                }
                if let Some(m) = m {
                  s.mode = m.to_owned();
                }
              });
              save();
            } />
          </div>
          <div class="grid gap-3 p-4 sm:grid-cols-2 lg:grid-cols-5">
            <label class="flex flex-col gap-1.5 text-xs text-muted-foreground lg:col-span-2">
              "竞赛"
              <select
                class=input_class("")
                prop:value=move || session.with(|s| s.contest_id.clone())
                on:change=move |e| {
                  let id = event_target_value(&e);
                  let zone = lookup(&my_call.get_untracked()).map(|e| e.cq.to_string());
                  session.update(|s| {
                    s.contest_id = id;
                    s.my_exch = match s.def().sent {
                      Exch::CqZone => zone.unwrap_or_default(),
                      Exch::Power => "100".to_owned(),
                      _ => String::new(),
                    };
                  });
                  save();
                }
              >
                {CONTESTS.iter().map(|c| view! { <option value=c.id>{c.name}</option> }).collect_view()}
              </select>
            </label>
            <label class="flex flex-col gap-1.5 text-xs text-muted-foreground">
              "频率（MHz）"
              <input
                class=input_class("font-mono")
                inputmode="decimal"
                prop:value=move || session.with(|s| s.freq.clone())
                on:change=move |e| {
                  let v = event_target_value(&e);
                  session.update(|s| s.freq = v);
                  save();
                }
              />
            </label>
            <label class="flex flex-col gap-1.5 text-xs text-muted-foreground">
              "模式"
              <select
                class=input_class("")
                prop:disabled=move || session.with(|s| s.def().mode.is_some())
                prop:value=move || session.with(Session::mode)
                on:change=move |e| {
                  let v = event_target_value(&e);
                  session.update(|s| s.mode = v);
                  save();
                }
              >
                {MODES.iter().map(|m| view! { <option value=*m>{*m}</option> }).collect_view()}
              </select>
            </label>
            <label class="flex flex-col gap-1.5 text-xs text-muted-foreground">
              {move || format!("我发出的{}", session.with(|s| s.def().sent.label()))}
              <input
                class=input_class("font-mono uppercase")
                prop:disabled=move || session.with(|s| s.def().sent == Exch::Serial)
                prop:value=move || {
                  if session.with(|s| s.def().sent == Exch::Serial) { "自动递增".to_owned() } else { session.with(|s| s.my_exch.clone()) }
                }
                on:change=move |e| {
                  let v = event_target_value(&e).trim().to_ascii_uppercase();
                  session.update(|s| s.my_exch = v);
                  save();
                }
              />
            </label>
          </div>
          <div class="flex flex-wrap items-center gap-3 border-t px-4 py-3 text-xs text-muted-foreground">
            <span>{move || format!("本场开始于 {} UTC", session.with(|s| s.start.clone()))}</span>
            <label class="inline-flex items-center gap-1.5">
              "功率类别"
              <select
                class="rounded border bg-background px-1.5 py-0.5"
                prop:value=move || session.with(|s| s.power.clone())
                on:change=move |e| {
                  let v = event_target_value(&e);
                  session.update(|s| s.power = v);
                  save();
                }
              >
                <option value="HIGH">"HIGH"</option>
                <option value="LOW">"LOW"</option>
                <option value="QRP">"QRP"</option>
              </select>
            </label>
            <button type="button" class=button_class(Variant::Ghost, Size::Sm, "ml-auto") on:click=move |_| restart()>
              "开始新一场"
            </button>
          </div>
        </section>

        <section class="rounded-xl border bg-card p-4">
          <form
            class="grid gap-3 sm:grid-cols-[1fr_auto_auto_auto] sm:items-end"
            on:submit=move |e| {
              e.prevent_default();
              log_it();
            }
          >
            <label class="flex flex-col gap-1.5 text-xs text-muted-foreground">
              "对方呼号"
              <input
                node_ref=call_ref
                autofocus
                autocomplete="off"
                autocapitalize="characters"
                spellcheck="false"
                aria-describedby="contest-call-hint"
                prop:value=move || call.get()
                on:input=move |e| {
                  call.set(event_target_value(&e).to_ascii_uppercase());
                  message.set(None);
                }
                on:keydown=move |e| {
                  match e.key().as_str() {
                    "Enter" | " " if rcvd.with_untracked(String::is_empty) && call.with_untracked(|c| c.trim().len() >= 3) => {
                      e.prevent_default();
                      focus(rcvd_ref);
                    }
                    "Escape" => {
                      call.set(String::new());
                      rcvd.set(String::new());
                    }
                    _ => {}
                  }
                }
                class=move || {
                  let base = "h-12 rounded-lg border bg-background px-3 font-mono text-2xl font-semibold uppercase tracking-wider outline-none focus-visible:ring-2";
                  if dupe.get() {
                    format!("{base} border-red-500 text-red-600 focus-visible:ring-red-500/40 dark:text-red-400")
                  } else {
                    format!("{base} focus-visible:ring-ring/50")
                  }
                }
              />
            </label>
            <div class="flex flex-col gap-1.5 text-xs text-muted-foreground">
              "发出"
              <div class="flex h-12 items-center rounded-lg border bg-muted/40 px-3 font-mono text-lg tabular-nums text-foreground">
                {move || format!("{} {}", default_rst(&session.with(Session::mode)), sent_exch())}
              </div>
            </div>
            <label class="flex flex-col gap-1.5 text-xs text-muted-foreground">
              {move || format!("收到的{}", session.with(|s| s.def().rcvd.label()))}
              <input
                node_ref=rcvd_ref
                autocomplete="off"
                spellcheck="false"
                placeholder=rcvd_placeholder
                prop:value=move || rcvd.get()
                on:input=move |e| rcvd.set(event_target_value(&e).to_ascii_uppercase())
                class="h-12 w-28 rounded-lg border bg-background px-3 font-mono text-2xl font-semibold uppercase outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
              />
            </label>
            <button type="submit" class=button_class(Variant::Default, Size::Default, "h-12 px-6")>"记录"</button>
          </form>
          <div id="contest-call-hint" aria-live="polite" class="mt-2 min-h-5 text-sm">
            {move || {
              if dupe.get() {
                view! { <span class="font-medium text-red-600 dark:text-red-400">"重复：本波段已通联过"</span> }.into_any()
              } else if let Some(m) = message.get() {
                view! { <span class="text-emerald-600 dark:text-emerald-400">{m}</span> }.into_any()
              } else {
                view! { <span class="text-muted-foreground">{hint}</span> }.into_any()
              }
            }}
          </div>
          <p class="mt-1 text-xs text-muted-foreground">
            "呼号框按回车或空格跳到交换，再按回车记录；Esc 清空。时间按当前 UTC 自动填写，信号报告默认 599 / 59。"
          </p>
        </section>

        <div class="grid grid-cols-2 gap-3 sm:grid-cols-5">
          <Stat label="有效通联" value=Signal::derive(move || tally.get().qsos as usize) />
          <Stat label="重复" value=Signal::derive(move || tally.get().dupes as usize) />
          <Stat label="点数" value=Signal::derive(move || tally.get().points as usize) />
          <Stat label="乘数" value=Signal::derive(move || tally.get().mults as usize) />
          <Stat label="自报总分" value=Signal::derive(move || tally.get().total as usize) />
        </div>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">"本场通联"</h2>
            {move || per_band.get().into_iter().map(|(b, n)| view! {
              <span class="rounded-full border px-2 py-0.5 text-xs text-muted-foreground">{format!("{b} {n}")}</span>
            }).collect_view()}
          </div>
          <div class="overflow-x-auto">
            <table class="w-full text-left text-sm">
              <thead class="text-xs text-muted-foreground">
                <tr class="border-b">
                  <th class="px-4 py-2 font-normal">"时间"</th>
                  <th class="px-2 py-2 font-normal">"频率"</th>
                  <th class="px-2 py-2 font-normal">"呼号"</th>
                  <th class="px-2 py-2 font-normal">"发出"</th>
                  <th class="px-2 py-2 font-normal">"收到"</th>
                  <th class="px-2 py-2 font-normal">"实体"</th>
                  <th class="px-4 py-2 font-normal"><span class="sr-only">"操作"</span></th>
                </tr>
              </thead>
              <tbody class="font-mono">
                {move || {
                  let list = entries.get();
                  if list.is_empty() {
                    return view! {
                      <tr><td colspan="7" class="px-4 py-6 text-center font-sans text-sm text-muted-foreground">"还没有通联，输入呼号开始吧"</td></tr>
                    }.into_any();
                  }
                  list
                    .into_iter()
                    .rev()
                    .take(50)
                    .map(|e| {
                      let id = e.id;
                      let entity = e.entity().map(|x| x.name).unwrap_or("—");
                      view! {
                        <tr class="border-b last:border-0">
                          <td class="px-4 py-1.5 tabular-nums">{e.time.clone()}</td>
                          <td class="px-2 py-1.5 tabular-nums">{e.freq.clone()}</td>
                          <td class="px-2 py-1.5 font-semibold">{e.callsign.clone()}</td>
                          <td class="px-2 py-1.5">{format!("{} {}", e.rst_sent, e.stx)}</td>
                          <td class="px-2 py-1.5">{format!("{} {}", e.rst_rcvd, e.srx)}</td>
                          <td class="px-2 py-1.5 font-sans text-xs text-muted-foreground">{entity}</td>
                          <td class="px-4 py-1.5 text-right">
                            <button
                              type="button"
                              class="font-sans text-xs text-muted-foreground hover:text-red-600"
                              aria-label=format!("删除 {}", e.callsign)
                              on:click=move |_| remove(id)
                            >
                              "删除"
                            </button>
                          </td>
                        </tr>
                      }
                    })
                    .collect_view()
                    .into_any()
                }}
              </tbody>
            </table>
          </div>
        </section>

        <p class="text-xs text-muted-foreground">
          "竞赛通联同时保存在通联日志中（带 CONTEST_ID 与交换信息，ADIF 导出时一并写出）。分数按中国台站视角简化计算，仅供参考，以主办方核对为准。"
        </p>
      </div>
    </div>
  }
}

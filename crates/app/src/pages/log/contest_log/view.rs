use std::collections::BTreeMap;

use ham_web_core::contest::{
  CONTESTS, CabrilloHeader, Exch, cabrillo, default_rst, is_dupe, score,
};
use ham_web_core::dxcc::lookup;
use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::components::cat_control::{CatControl, CatReading};
use crate::pages::log::qsl_image;
use crate::pages::log::{LogEntry, use_log_store, utc_now_time, utc_today};
use crate::ui::{
  Button, ButtonKind, ButtonLink, ControlSize, Field, Input, NativeSelect, SelectOption, Size,
  Stat, Variant,
};
use crate::util::unique_id as ui_id;
use crate::util::{download_text, set_title, storage};

use super::session::Session;
use crate::i18n::{t, tf};

const SESSION_KEY: &str = "contest-session";
const MODES: &[&str] = &["CW", "SSB", "FT8", "RTTY"];

#[component]
pub fn ContestLogPage() -> impl IntoView {
  set_title("shell.contest-log");
  let store = use_log_store();
  let session = RwSignal::new(storage::get_json::<Session>(SESSION_KEY).unwrap_or_default());
  // 支持 `/contest-log?contest=ID` 预选竞赛（如从竞赛日历「开新场次」进入）。
  let query = use_query_map();
  if let Some(cid) = query.with(|q| q.get("contest"))
    && ham_web_core::contest::contest(&cid).is_some()
  {
    session.update(|s| {
      s.contest_id = cid;
      s.my_exch.clear();
    });
  }
  let save = move || storage::set_json(SESSION_KEY, &session.get_untracked());

  let contest_options: Vec<SelectOption> = CONTESTS
    .iter()
    .map(|c| SelectOption::new(c.id, Signal::derive(move || t(c.name))))
    .collect();
  let mode_options: Vec<SelectOption> = MODES.iter().map(|m| SelectOption::new(*m, *m)).collect();
  let power_options = vec![
    SelectOption::new("HIGH", "HIGH"),
    SelectOption::new("LOW", "LOW"),
    SelectOption::new("QRP", "QRP"),
  ];
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

  // `Field` 的标签与控件是兄弟节点，`r#for` / `id` 必须配对才能点击标签聚焦输入框。
  let contest_id_field = ui_id("contest-contest");
  let freq_id = ui_id("contest-freq");
  let mode_id = ui_id("contest-mode");
  let my_exch_id = ui_id("contest-my-exch");
  let power_id = ui_id("contest-power");
  let call_id = ui_id("contest-call");
  let rcvd_id = ui_id("contest-rcvd");
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
      tf("contest.logged-dupe-no-points", &[&(label).to_string()])
    } else {
      tf("contest.logged", &[&(label).to_string()])
    }));
    call.set(String::new());
    rcvd.set(String::new());
    focus(call_ref);
  };
  let remove = move |id: u64| {
    store.logbook.update(|lb| lb.entries.retain(|e| e.id != id));
    // 卡片影像按 id 绑定，而 `next_id()` 会复用 id —— 竞赛日志与普通日志共用同一份
    // `logbook` 与同一批 `qsl-image:<id>` 键，这里漏清就会让下一个新通联直接挂上
    // 已删记录的扫描件（`log_page` 的删除路径就是这么做的，两处必须一致）。
    qsl_image::purge(store, &[id]);
    store.persist();
  };
  let restart = move || {
    session.update(|s| s.start = format!("{} {}", utc_today(), utc_now_time()));
    save();
    message.set(Some(t("contest.new-session-started-earlier")));
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
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.contest-log")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("contest.fast-keyboard-entry-live")}</div>
          </div>
          <ButtonLink
            href="/log"
            variant=Variant::Outline
            size=Size::Sm
          >{move || t("shell.logbook")}</ButtonLink>
          <Button
            variant=Variant::Default
            size=Size::Sm
            on_click=Callback::new(move |_| export())
          >
            {move || t("contest.export-cabrillo")}
          </Button>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-5 px-4 py-5">
        {move || my_call.get().is_empty().then(|| view! {
          <p class="rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm">
            {move || t("contest.station-callsign-not-set")}
            " "
            <a href="/log" class="font-medium underline underline-offset-4">{move || t("contest.qso-log-station-info")}</a>
            " "
            {move || t("contest.to-fill-it-in")}
          </p>
        })}

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("contest.contest-setup")}</h2>
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
            <Field
              label=Signal::derive(move || t("contest.contest"))
              r#for=contest_id_field.clone()
              class="lg:col-span-2"
            >
              <NativeSelect
                id=contest_id_field.clone()
                value=Signal::derive(move || session.with(|s| s.contest_id.clone()))
                on_change=Callback::new(move |id: String| {
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
                })
                options=contest_options
                aria_label=Signal::derive(move || t("contest.contest"))
              />
            </Field>
            <Field label=Signal::derive(move || t("log.frequency-mhz")) r#for=freq_id.clone()>
              <Input
                id=freq_id.clone()
                value=Signal::derive(move || session.with(|s| s.freq.clone()))
                on_change=Callback::new(move |v: String| {
                  session.update(|s| s.freq = v);
                  save();
                })
                inputmode="decimal"
                class="font-mono"
              />
            </Field>
            <Field label=Signal::derive(move || t("log.mode")) r#for=mode_id.clone()>
              <NativeSelect
                id=mode_id.clone()
                value=Signal::derive(move || session.with(Session::mode))
                on_change=Callback::new(move |v: String| {
                  session.update(|s| s.mode = v);
                  save();
                })
                options=mode_options
                disabled=Signal::derive(move || session.with(|s| s.def().mode.is_some()))
                aria_label=Signal::derive(move || t("log.mode"))
              />
            </Field>
            <Field
              label=Signal::derive(move || tf("contest.i-send", &[(session.with(|s| s.def().sent.label()))]))
              r#for=my_exch_id.clone()
            >
              <Input
                id=my_exch_id.clone()
                value=Signal::derive(move || {
                  if session.with(|s| s.def().sent == Exch::Serial) { t("contest.auto-increment") } else { session.with(|s| s.my_exch.clone()) }
                })
                on_change=Callback::new(move |v: String| {
                  session.update(|s| s.my_exch = v.trim().to_ascii_uppercase());
                  save();
                })
                disabled=Signal::derive(move || session.with(|s| s.def().sent == Exch::Serial))
                class="font-mono uppercase"
              />
            </Field>
          </div>
          <div class="flex flex-wrap items-end gap-3 border-t px-4 py-3 text-xs text-muted-foreground">
            <span>{move || tf("contest.this-session-started-utc", &[&(session.with(|s| s.start.clone())).to_string()])}</span>
            <Field
              label=Signal::derive(move || t("contest.power-category"))
              r#for=power_id.clone()
            >
              <NativeSelect
                id=power_id.clone()
                value=Signal::derive(move || session.with(|s| s.power.clone()))
                on_change=Callback::new(move |v: String| {
                  session.update(|s| s.power = v);
                  save();
                })
                options=power_options
                size=ControlSize::Sm
                aria_label=Signal::derive(move || t("contest.power-category"))
                class="w-auto"
              />
            </Field>
            <Button
              variant=Variant::Ghost
              size=Size::Sm
              class="ml-auto"
              on_click=Callback::new(move |_| restart())
            >
              {move || t("contest.start-new-session")}
            </Button>
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
            <Field label=Signal::derive(move || t("contest.their-callsign")) r#for=call_id.clone()>
              <Input
                id=call_id.clone()
                node_ref=call_ref
                value=call
                on_change=Callback::new(move |v: String| {
                  call.set(v.to_ascii_uppercase());
                  message.set(None);
                })
                on_keydown=Callback::new(move |e: web_sys::KeyboardEvent| {
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
                })
                invalid=dupe
                autofocus=true
                autocomplete="off"
                autocapitalize="characters"
                spellcheck="false"
                aria_describedby="contest-call-hint"
                aria_label=Signal::derive(move || t("contest.their-callsign"))
                size=ControlSize::Lg
                class="h-12 px-3 font-mono text-2xl md:text-2xl font-semibold tracking-wider uppercase"
              />
            </Field>
            <Field label=Signal::derive(move || t("contest.sent"))>
              <div class="flex h-12 items-center rounded-md border bg-muted/40 px-3 font-mono text-lg tabular-nums text-foreground">
                {move || format!("{} {}", default_rst(&session.with(Session::mode)), sent_exch())}
              </div>
            </Field>
            <Field
              label=Signal::derive(move || tf("contest.received-2", &[(session.with(|s| s.def().rcvd.label()))]))
              r#for=rcvd_id.clone()
            >
              <Input
                id=rcvd_id.clone()
                node_ref=rcvd_ref
                value=rcvd
                on_change=Callback::new(move |v: String| rcvd.set(v.to_ascii_uppercase()))
                placeholder=Signal::derive(rcvd_placeholder)
                autocomplete="off"
                spellcheck="false"
                aria_label=Signal::derive(move || tf("contest.received-2", &[(session.with(|s| s.def().rcvd.label()))]))
                size=ControlSize::Lg
                class="h-12 w-28 px-3 font-mono text-2xl md:text-2xl font-semibold uppercase"
              />
            </Field>
            <Button kind=ButtonKind::Submit class="h-12 px-6">
              {move || t("contest.log-it")}
            </Button>
          </form>
          <div id="contest-call-hint" aria-live="polite" class="mt-2 min-h-5 text-sm">
            {move || {
              if dupe.get() {
                view! { <span class="font-medium text-red-600 dark:text-red-400">{move || t("contest.dupe-already-worked-on")}</span> }.into_any()
              } else if let Some(m) = message.get() {
                view! { <span class="text-emerald-600 dark:text-emerald-400">{m}</span> }.into_any()
              } else {
                view! { <span class="text-muted-foreground">{hint}</span> }.into_any()
              }
            }}
          </div>
          <p class="mt-1 text-xs text-muted-foreground">
            {move || t("contest.press-enter-or-space")}
          </p>
        </section>

        <div class="grid grid-cols-2 gap-3 sm:grid-cols-5">
          <Stat label=t("contest.valid-qsos") value=Signal::derive(move || tally.get().qsos as usize) />
          <Stat label=t("contest.dupes") value=Signal::derive(move || tally.get().dupes as usize) />
          <Stat label=t("contest.points") value=Signal::derive(move || tally.get().points as usize) />
          <Stat label=t("contest.multipliers") value=Signal::derive(move || tally.get().mults as usize) />
          <Stat label=t("contest.claimed-score") value=Signal::derive(move || tally.get().total as usize) />
        </div>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("contest.session-qsos")}</h2>
            {move || per_band.get().into_iter().map(|(b, n)| view! {
              <span class="rounded-full border px-2 py-0.5 text-xs text-muted-foreground">{format!("{b} {n}")}</span>
            }).collect_view()}
          </div>
          <div class="overflow-x-auto">
            <table class="w-full text-left text-sm">
              <thead class="text-xs text-muted-foreground">
                <tr class="border-b">
                  <th class="px-4 py-2 font-normal">{move || t("log.time")}</th>
                  <th class="px-2 py-2 font-normal">{move || t("contest.freq")}</th>
                  <th class="px-2 py-2 font-normal">{move || t("log.callsign")}</th>
                  <th class="px-2 py-2 font-normal">{move || t("contest.sent")}</th>
                  <th class="px-2 py-2 font-normal">{move || t("contest.received")}</th>
                  <th class="px-2 py-2 font-normal">{move || t("contest.entity")}</th>
                  <th class="px-4 py-2 font-normal"><span class="sr-only">{move || t("log.actions")}</span></th>
                </tr>
              </thead>
              <tbody class="font-mono">
                {move || {
                  let list = entries.get();
                  if list.is_empty() {
                    return view! {
                      <tr><td colspan="7" class="px-4 py-6 text-center font-sans text-sm text-muted-foreground">{move || t("contest.no-qsos-yet-type")}</td></tr>
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
                              aria-label=tf("contest.delete", &[&(e.callsign).to_string()])
                              on:click=move |_| remove(id)
                            >
                              {move || t("log.delete")}
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
          {move || t("contest.contest-qsos-are-also")}
        </p>
      </div>
    </div>
  }
}

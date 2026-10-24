use ham_web_core::contest::{
  CONTESTS, CabrilloHeader, Exch, cabrillo, default_rst, is_dupe, score,
};
use ham_web_core::dxcc::lookup;
use ham_web_core::scp::{Index, ScpFile, parse_scp};
use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::components::cat_control::{CatControl, CatReading};
use crate::components::common::{PageContainer, PageHeader};
use crate::pages::log::qsl_image;
use crate::pages::log::{LogEntry, use_log_store, utc_now_time, utc_today};
use crate::ui::{
  Button, ButtonKind, ButtonLink, ControlSize, Field, Input, NativeSelect, SelectOption, Size,
  Stat, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Variant, card_title_class,
};
use crate::util::unique_id as ui_id;
use crate::util::{download_text, set_title, storage};

use super::analysis::ContestAnalysis;
use super::cabrillo_import::{CabrilloImportButton, ImportReport};
use super::import_report::ImportReportPanel;
use super::pace::ContestPace;
use super::scp_source::{STORAGE_KEY as SCP_KEY, ScpSource};
use super::scp_suggest::ScpSuggest;
use super::session::Session;
use crate::i18n::{localized_name, t, tf};

pub(super) const SESSION_KEY: &str = "contest-session";
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
      s.start = format!("{} {}", utc_today(), utc_now_time());
      s.end.clear();
      s.station_id = store.station.with_untracked(|book| book.active().id);
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
  let session_station = Memo::new(move |_| {
    let id = session.with(|s| s.station_id);
    store.station.with(|book| book.resolve(id).info())
  });
  let my_call = Memo::new(move |_| session_station.get().callsign.trim().to_ascii_uppercase());

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
    store.station.with(|book| {
      store.logbook.with(|lb| {
        lb.entries
          .iter()
          .filter(|e| s.includes(e, book))
          .cloned()
          .collect::<Vec<_>>()
      })
    })
  });
  let def = Memo::new(move |_| session.with(Session::def));
  let import_report = RwSignal::new(None::<ImportReport>);

  // 呼号补全：候选 = 自己的通联日志里出现过的呼号 + 用户自己载入的 SCP 文件（本站不带 SCP 数据）。
  let scp_file = RwSignal::new(None::<ScpFile>);
  leptos::task::spawn_local(async move {
    if let Some(text) = crate::kv::load_store_only(SCP_KEY).await
      && let Ok(f) = parse_scp(&text)
    {
      scp_file.set(Some(f));
    }
  });
  let log_calls = Memo::new(move |_| {
    store.logbook.with(|lb| {
      let mut v: Vec<String> = lb.entries.iter().map(|e| e.callsign.clone()).collect();
      v.sort_unstable();
      v.dedup();
      v
    })
  });
  let scp_index = Memo::new(move |_| {
    scp_file.with(|f| {
      Index::build(
        log_calls
          .with(|c| c.clone())
          .into_iter()
          .chain(f.iter().flat_map(|f| f.calls.iter().cloned())),
      )
    })
  });
  let from_log = Signal::derive(move || log_calls.with(Vec::len));
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
      format!(
        "{:03}",
        entries.with(|e| ham_web_core::contest::next_serial(e))
      )
    } else {
      s.my_exch.clone()
    }
  };
  let draft = move || {
    let s = session.get_untracked();
    let mode = s.mode();
    LogEntry {
      id: store.logbook.with_untracked(|lb| lb.next_id()),
      station_id: store
        .station
        .with_untracked(|book| book.resolve(s.station_id).id),
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
    Some(format!(
      "{} · {} · CQ {}",
      localized_name(en.name, en.name_en),
      en.continent,
      en.cq
    ))
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
    let end = session.with_untracked(|s| {
      entry
        .minutes_on()
        .and_then(|now| ham_web_core::contest::continuation_end(&s.end, now))
    });
    let Some(end) = end else {
      crate::ui::push_error(t("contest.session-history-closed"));
      return;
    };
    session.update(|s| s.end = end);
    save();
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
    session.update(|s| {
      s.start = format!("{} {}", utc_today(), utc_now_time());
      s.end.clear();
      s.station_id = store.station.with_untracked(|book| book.active().id);
    });
    save();
    message.set(Some(t("contest.new-session-started-earlier")));
  };
  let export = move || {
    let s = session.get_untracked();
    let st = session_station.get_untracked();
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

  let per_band =
    Memo::new(move |_| entries.with(|list| ham_web_core::log_stats::band_counts(list)));

  view! {
    <div class="min-h-svh animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      <PageHeader
        title=move || t("shell.contest-log")
        subtitle=move || t("contest.fast-keyboard-entry-live")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink
              href="/log"
              variant=Variant::Outline
              size=Size::Sm
            >{move || t("shell.logbook")}</ButtonLink>
            <CabrilloImportButton session=session report=import_report />
            <Button
              variant=Variant::Default
              size=Size::Sm
              on_click=Callback::new(move |_| export())
            >
              {move || t("contest.export-cabrillo")}
            </Button>
          }
        })
      />

      <PageContainer class="space-y-5">
        {move || my_call.get().is_empty().then(|| view! {
          <p class="border border-gold/40 bg-gold/10 px-3 py-2 text-sm">
            {move || t("contest.station-callsign-not-set")}
            " "
            <a href="/log" class="underline underline-offset-4">{move || t("contest.qso-log-station-info")}</a>
            " "
            {move || t("contest.to-fill-it-in")}
          </p>
        })}

        <section class="pxl-card border bg-card">
          <h2 class="pxl-section-title">{move || t("contest.contest-setup")}</h2>
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
                    s.end.clear();
                    s.start = format!("{} {}", utc_today(), utc_now_time());
                    s.station_id = store.station.with_untracked(|book| book.active().id);
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
              label=Signal::derive(move || {
                // 交换名先翻一遍再套模板：`Exch::label()` 给的是中文原文，直接当实参填进去
                // 会让英文界面出现「I send 序号」。
                tf("contest.i-send", &[&t(session.with(|s| s.def().sent.label()))])
              })
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

        <section class="pxl-card border bg-card p-4">
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
                class="h-12 px-3 font-mono text-2xl md:text-2xl tracking-wider uppercase"
              />
            </Field>
            <Field label=Signal::derive(move || t("contest.sent"))>
              <div class="flex h-12 items-center border bg-inset px-3 font-mono text-lg tabular-nums text-foreground">
                {move || format!("{} {}", default_rst(&session.with(Session::mode)), sent_exch())}
              </div>
            </Field>
            <Field
              label=Signal::derive(move || {
                // 同「我发出的」：交换名先翻一遍再套模板。
                tf("contest.received-field", &[&t(session.with(|s| s.def().rcvd.label()))])
              })
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
                aria_label=Signal::derive(move || {
                  tf("contest.received-field", &[&t(session.with(|s| s.def().rcvd.label()))])
                })
                size=ControlSize::Lg
                class="h-12 w-28 px-3 font-mono text-2xl md:text-2xl uppercase"
              />
            </Field>
            <Button kind=ButtonKind::Submit class="h-12 px-6">
              {move || t("contest.log-it")}
            </Button>
          </form>
          <div id="contest-call-hint" aria-live="polite" class="mt-2 min-h-5 text-sm">
            {move || {
              if dupe.get() {
                view! { <span class="text-hp-text">{move || t("contest.dupe-already-worked-on")}</span> }.into_any()
              } else if let Some(m) = message.get() {
                view! { <span class="text-win-text">{m}</span> }.into_any()
              } else {
                view! { <span class="text-muted-foreground">{hint}</span> }.into_any()
              }
            }}
          </div>
          <ScpSuggest
            call=call
            index=scp_index
            on_pick=Callback::new(move |c: String| {
              call.set(c);
              message.set(None);
              focus(rcvd_ref);
            })
          />
          <p class="mt-1 text-xs text-muted-foreground">
            {move || t("contest.press-enter-or-space")}
          </p>
          <ScpSource file=scp_file from_log=from_log />
        </section>

        <div class="grid grid-cols-2 gap-3 sm:grid-cols-5">
          <Stat label=t("contest.valid-qsos") value=Signal::derive(move || tally.get().qsos as usize) />
          <Stat label=t("contest.dupes") value=Signal::derive(move || tally.get().dupes as usize) />
          <Stat label=t("contest.points") value=Signal::derive(move || tally.get().points as usize) />
          <Stat label=t("contest.multipliers") value=Signal::derive(move || tally.get().mults as usize) />
          <Stat label=t("contest.claimed-score") value=Signal::derive(move || tally.get().total as usize) />
        </div>

        <section class="pxl-card border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class=card_title_class("mr-auto")>{move || t("contest.session-qsos")}</h2>
            {move || per_band.get().into_iter().map(|(b, n)| view! {
              <span class="border px-2 py-0.5 text-xs text-muted-foreground">{format!("{b} {n}")}</span>
            }).collect_view()}
          </div>
          // `relative`：表头里的 `sr-only`（`position: absolute` 的 1px 盒子）需要一个在
          // 滚动容器内部的包含块。否则它以卡片为包含块，那 1px 会被算进**文档**的可滚动
          // 溢出区 —— 手机宽度下整页多出二十来像素的横向滚动（表格本身横向滚是正常的）。
          <Table class="relative text-sm">
            <TableHead>
              <TableRow>
                <TableHeader class="px-4 py-2 font-normal">{move || t("log.time")}</TableHeader>
                <TableHeader class="px-2 py-2 font-normal">{move || t("contest.freq")}</TableHeader>
                <TableHeader class="px-2 py-2 font-normal">{move || t("log.callsign")}</TableHeader>
                <TableHeader class="px-2 py-2 font-normal">{move || t("contest.sent")}</TableHeader>
                <TableHeader class="px-2 py-2 font-normal">{move || t("contest.received")}</TableHeader>
                <TableHeader class="px-2 py-2 font-normal">{move || t("contest.entity")}</TableHeader>
                <TableHeader class="px-4 py-2 font-normal"><span class="sr-only">{move || t("log.actions")}</span></TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {move || {
                let list = entries.get();
                if list.is_empty() {
                  return view! {
                    <TableRow><TableCell colspan=7 class="px-4 py-6 text-center font-sans text-sm text-muted-foreground">{move || t("contest.no-qsos-yet-type")}</TableCell></TableRow>
                  }.into_any();
                }
                list
                  .into_iter()
                  .rev()
                  .take(50)
                  .map(|e| {
                    let id = e.id;
                    let entity = e
                      .entity()
                      .map(|x| localized_name(x.name, x.name_en))
                      .unwrap_or_else(|| "—".to_owned());
                    let time = e.time.clone();
                    let freq = e.freq.clone();
                    let callsign = e.callsign.clone();
                    let sent = format!("{} {}", e.rst_sent, e.stx);
                    let rcvd = format!("{} {}", e.rst_rcvd, e.srx);
                    let delete_label = callsign.clone();
                    view! {
                      <TableRow class="border-b font-mono last:border-0">
                        <TableCell class="px-4 py-1.5 tabular-nums">{time}</TableCell>
                        <TableCell class="px-2 py-1.5 tabular-nums">{freq}</TableCell>
                        <TableCell class="px-2 py-1.5">{callsign}</TableCell>
                        <TableCell class="px-2 py-1.5">{sent}</TableCell>
                        <TableCell class="px-2 py-1.5">{rcvd}</TableCell>
                        <TableCell class="px-2 py-1.5 font-sans text-xs text-muted-foreground">{entity}</TableCell>
                        <TableCell class="px-4 py-1.5 text-right">
                          <Button
                            variant=Variant::Text
                            size=Size::Xs
                            class="font-sans hover:text-hp-text"
                            aria_label=tf("contest.delete", &[&delete_label])
                            on_click=Callback::new(move |_| remove(id))
                          >
                            {move || t("log.delete")}
                          </Button>
                        </TableCell>
                      </TableRow>
                    }
                  })
                  .collect_view()
                  .into_any()
              }}
            </TableBody>
          </Table>
        </section>

        <ImportReportPanel report=import_report />

        <ContestPace
          session=session
          done=Signal::derive(move || u32::try_from(entries.with(Vec::len)).unwrap_or(0))
          save=Callback::new(move |()| save())
        />

        <ContestAnalysis entries=entries def=def my_call=my_call />

        <p class="text-xs text-muted-foreground">
          {move || t("contest.contest-qsos-are-also")}
        </p>
      </PageContainer>
    </div>
  }
}

use ham_web_core::adif::parse_adif;
use ham_web_core::logbook::{export_adif, export_csv};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;
use wasm_bindgen::JsCast;

use crate::ui::{Size, Variant, button_class};
use crate::util::download_text;
use crate::util::set_title;

use super::entry_form::EntryForm;
use super::entry_list::EntryList;
use super::form_state::LogFormState;
use super::log_helpers::confirm;
use super::log_stats_panel::LogStatsPanel;
use super::qsl_sync_dialog::QslSyncDialog;
use super::station_panel::StationPanel;
use super::{LogEntry, Logbook, use_log_store, utc_today};
use crate::i18n::{t, tf};

#[component]
pub fn LogPage() -> impl IntoView {
  set_title(&t("通联日志"));

  let store = use_log_store();
  let logbook = store.logbook;
  let station = store.station;
  // 从 URL 读取初始搜索词（如 DXCC 地图点击实体后按实体名过滤）。
  let initial_query = use_query_map().get_untracked().get("q");
  let editing = RwSignal::new(None::<u64>);
  let form = LogFormState::new();
  let qsl_open = RwSignal::new(false);

  let reset_form = move || {
    form.reset();
    editing.set(None);
  };

  let save = move || {
    if form.callsign.get().trim().is_empty() {
      return;
    }
    if let Some(id) = editing.get() {
      logbook.update(|l| {
        if let Some(e) = l.entries.iter_mut().find(|e| e.id == id) {
          let mut next = form.build(id, Some(&*e));
          next.fill_location();
          *e = next;
        }
      });
    } else {
      logbook.update(|l| {
        let mut next = form.build(l.next_id(), None);
        next.fill_location();
        l.entries.push(next);
      });
    }
    store.persist();
    reset_form();
  };

  let edit = move |e: LogEntry| {
    if form.load(&e) {
      form.show_more.set(true);
    }
    editing.set(Some(e.id));
  };

  let remove = move |id: u64| {
    logbook.update(|l| l.entries.retain(|e| e.id != id));
    store.persist();
  };

  let clear = move || {
    let n = logbook.with_untracked(|l| l.entries.len());
    if confirm(&tf(
      "确定清空全部 {} 条通联记录吗？建议先导出 ADIF 备份。此操作不可撤销。",
      &[&n.to_string()],
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
    crate::util::alert(&t(
      "ADIF 已导出（含 LoTW / eQSL 状态）。可到 LoTW（TQSL）或 eQSL 官网登录后上传此文件，确认后回到日志勾选对应状态。",
    ));
  };

  let export_csv_btn = move || {
    let content = export_csv(&logbook.get_untracked().entries, &station.get_untracked());
    download_text(
      &format!("logbook-{}.csv", utc_today()),
      &content,
      "text/csv",
    );
  };

  let save_station_btn = Callback::new(move |()| {
    store.persist_station();
    crate::util::alert(&t("本台信息已保存"));
  });

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
        tf(
          "已导入 {} 条记录，跳过重复 {} 条",
          &[&added.to_string(), &dupes.to_string()],
        )
      } else {
        tf("已导入 {} 条记录", &[&added.to_string()])
      };
      crate::util::alert(&msg);
    });
  };

  let on_save = Callback::new(move |()| save());
  let on_edit = Callback::new(edit);
  let on_remove = Callback::new(remove);
  let on_clear = Callback::new(move |()| clear());

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("通联日志")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("在线记录 · 本地保存 · 导出 ADIF / CSV")}</div>
          </div>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Sm, "")
            on:click=move |_| export()
          >
            {move || t("导出 ADIF")}
          </button>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Sm, "")
            on:click=move |_| export_csv_btn()
          >
            {move || t("导出 CSV")}
          </button>
          <a href="/qsl-labels" class=button_class(Variant::Outline, Size::Sm, "")>{move || t("打印 QSL 标签")}</a>
          <label class=button_class(Variant::Outline, Size::Sm, "cursor-pointer")>
            {move || t("导入 ADIF")}
            <input
              type="file"
              accept=".adi,.adif,.txt"
              class="hidden"
              on:change=import_adif
            />
          </label>
          <button
            type="button"
            class=button_class(Variant::Outline, Size::Sm, "")
            on:click=move |_| qsl_open.set(true)
          >
            {move || t("同步 QSL")}
          </button>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <StationPanel station=station on_save=save_station_btn />
        <LogStatsPanel logbook=logbook station=station />
        <EntryForm form=form logbook=logbook station=station editing=editing on_save=on_save />
        <EntryList logbook=logbook on_edit=on_edit on_remove=on_remove on_clear=on_clear initial_query=initial_query.unwrap_or_default() />
      </div>
      <QslSyncDialog open=qsl_open />
    </div>
  }
}

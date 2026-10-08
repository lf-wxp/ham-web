use crate::ui::{Button, ButtonLink, FileInput, Size, Variant};
use crate::util::download_text;
use crate::util::set_title;
use ham_web_core::adif::parse_adif;
use ham_web_core::logbook::{export_adif, export_csv};
use ham_web_core::qsl_sync::QslFlag;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_query_map;

use super::entry_form::EntryForm;
use super::entry_list::EntryList;
use super::form_state::LogFormState;
use super::log_health_panel::LogHealthPanel;
use super::log_helpers::confirm;
use super::log_stats_panel::LogStatsPanel;
use super::qsl_image;
use super::qsl_sync_dialog::QslSyncDialog;
use super::station_panel::StationPanel;
use super::{LogEntry, Logbook, use_log_store, utc_today};
use crate::i18n::{t, tf, tp};

#[component]
pub fn LogPage() -> impl IntoView {
  set_title("shell.logbook");

  let store = use_log_store();
  let logbook = store.logbook;
  let book = store.station;
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
      // 表单里选了归属台站就用它，没选（0）落到当前台站。
      let active = book.with_untracked(|b| b.active().id);
      logbook.update(|l| {
        let mut next = form.build(l.next_id(), None);
        next.fill_location();
        if next.station_id == 0 {
          next.station_id = active;
        }
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
    // 影像与 id 绑定，而 `next_id()` 会复用 id —— 一并清掉，别让新通联挂上旧扫描件。
    qsl_image::purge(store, &[id]);
    store.persist();
  };

  let clear = move || {
    let n = logbook.with_untracked(|l| l.entries.len());
    if confirm(&tp("log.clear-all-qso-records", n, &[&n.to_string()])) {
      let ids: Vec<u64> = logbook.with_untracked(|l| l.entries.iter().map(|e| e.id).collect());
      logbook.set(Logbook::default());
      qsl_image::purge(store, &ids);
      store.persist();
    }
  };

  let export = move || {
    let content = export_adif(&logbook.get_untracked().entries, &book.get_untracked());
    download_text(
      &format!("logbook-{}.adi", utc_today()),
      &content,
      "text/plain",
    );
    crate::util::alert(&t("log.adif-exported-with-lotw"));
  };

  let export_csv_btn = move || {
    let content = export_csv(&logbook.get_untracked().entries, &book.get_untracked());
    download_text(
      &format!("logbook-{}.csv", utc_today()),
      &content,
      "text/csv",
    );
  };

  let save_station_btn = Callback::new(move |()| {
    store.persist_station();
    crate::util::alert(&t("log.station-info-saved"));
  });

  // 导入 ADIF：`FileInput` 直接把选中的文件交过来（一次一个，多选忽略其余）；
  // 「读完清空 input」由组件负责，选同一个文件能重复触发。
  let import_adif = Callback::new(move |files: Vec<web_sys::File>| {
    let Some(file) = files.into_iter().next() else {
      return;
    };
    spawn_local(async move {
      let Some(text) = crate::util::read_file_text(&file).await else {
        // 读失败必须出声：静默返回会被当成「导入了 0 条」。
        crate::util::alert(&t("common.file-read-failed"));
        return;
      };
      let (added, dupes) = store.import(parse_adif(&text));
      let msg = if dupes > 0 {
        // 两个可数名词，且西语的过去分词要跟着各自变形 —— 整段放进内层片段各自 `tp`。
        // 内层「已导入 N 条记录」直接复用外层那条 key：语义完全相同，多一条同义 key
        // 只会让中文反向索引撞车（同一句中文只能映射到一个 key）。
        tf(
          "log.imported-records-skipped-duplicates",
          &[
            &tp("log.imported-records", added, &[&added.to_string()]),
            &tp("log.skipped-n-duplicates", dupes, &[&dupes.to_string()]),
          ],
        )
      } else {
        tp("log.imported-records", added, &[&added.to_string()])
      };
      crate::util::alert(&msg);
    });
  });

  // 还没标成「已上传 LoTW」的条数：按钮上的计数，也就是这次批量标记的作用范围。
  let pending_lotw =
    Memo::new(move |_| logbook.with(|lb| lb.entries.iter().filter(|e| !e.lotw_sent).count()));

  // LoTW 状态回写的「已上传」那一半：确认由「同步 QSL」按报告回写，这一半只能由用户在
  // 上传成功后自己宣告 —— 我们无法探测 LoTW 端到底收没收到。
  let mark_lotw = Callback::new(move |()| {
    let n = pending_lotw.get_untracked();
    if n == 0 || !confirm(&tf("log.mark-these-qsos-as-uploaded", &[&n.to_string()])) {
      return;
    }
    logbook.update(|lb| {
      for e in lb.entries.iter_mut().filter(|e| !e.lotw_sent) {
        // 复用同步模块的标志位读写：`LOTW_QSL_SENT` 的语义只在一处定义。
        QslFlag::LotwSent.set(e, true);
      }
    });
    store.persist();
    crate::util::alert(&tf("log.marked-as-uploaded-lotw", &[&n.to_string()]));
  });

  let on_save = Callback::new(move |()| save());
  let on_edit = Callback::new(edit);
  let on_remove = Callback::new(remove);
  let on_clear = Callback::new(move |()| clear());

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.logbook")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("log.log-online-saved-locally")}</div>
          </div>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            on_click=Callback::new(move |_| export())
          >
            {move || t("log.export-adif")}
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            on_click=Callback::new(move |_| export_csv_btn())
          >
            {move || t("log.export-csv")}
          </Button>
          <ButtonLink
            href="/qsl-labels"
            variant=Variant::Outline
            size=Size::Sm
          >{move || t("log.print-qsl-labels")}</ButtonLink>
          <FileInput
            accept=".adi,.adif,.txt"
            label=t("log.import-adif")
            variant=Variant::Outline
            size=Size::Sm
            on_files=import_adif
          />
          <Button
            variant=Variant::Outline
            size=Size::Sm
            on_click=Callback::new(move |_| qsl_open.set(true))
          >
            {move || t("log.sync-qsl")}
          </Button>
          <Button
            variant=Variant::Outline
            size=Size::Sm
            disabled=Signal::derive(move || pending_lotw.get() == 0)
            on_click=Callback::new(move |_| mark_lotw.run(()))
          >
            {move || tf("log.mark-lotw-uploaded", &[&pending_lotw.get().to_string()])}
          </Button>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <StationPanel book=book on_save=save_station_btn />
        <LogStatsPanel logbook=logbook book=book />
        <LogHealthPanel />
        <EntryForm form=form logbook=logbook book=book editing=editing on_save=on_save />
        <EntryList logbook=logbook on_edit=on_edit on_remove=on_remove on_clear=on_clear initial_query=initial_query.unwrap_or_default() />
      </div>
      <QslSyncDialog open=qsl_open />
    </div>
  }
}

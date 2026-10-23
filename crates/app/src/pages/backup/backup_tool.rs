use ham_web_core::text::format_units;
use ham_web_core::usage::sum_by_group;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::i18n::{t, tf, tp};
use crate::ui::{Button, Checkbox, FileInput, Variant};
use crate::util::{BackupModules, storage};

/// 本地存储 key 的中文说明。
fn key_label(key: &str) -> &'static str {
  match key {
    "logbook" => "通联日志",
    "mistake-book" => "错题本",
    "study-stats" => "累计答题统计",
    "exam-history" => "考试成绩历史",
    "bookmarks" => "收藏题目",
    "station-info" => "本台信息",
    "station-book" => "台站档案",
    "log-stats-scope" => "统计范围",
    "daily-checkin" => "每日打卡",
    "countdowns" => "倒计时",
    "morse-stats" | "morse-koch" => "摩尔斯练习",
    "dxcc_wanted_done" => "DXCC 稀有度",
    "dx-alerts" => "DX 热点提醒",
    "learning-plan" => "学习计划",
    "theme" => "主题",
    k if k.starts_with("practice:") => "练习进度 / 偏好",
    k if k.starts_with("exam:") => "考试存档 / 偏好",
    k if k.starts_with("ui:") || k.starts_with("mistake-book:") => "界面状态",
    _ => "其他",
  }
}

/// 数据备份：按模块勾选导出，导入 / 合并导入全部本地数据，并展示各项占用。
#[component]
pub(super) fn BackupTool() -> impl IntoView {
  let usage = RwSignal::new(storage::usage());
  let exporting = RwSignal::new(false);
  let importing = RwSignal::new(false);
  let alive = StoredValue::new(crate::util::mount_guard());
  let log_store = crate::pages::log::use_log_store();
  let export_log = RwSignal::new(true);
  let export_qsl = RwSignal::new(true);
  let export_other = RwSignal::new(true);

  let run_import = move |file: web_sys::File, merge: bool| {
    let Some(task) = crate::util::begin_backup_task(log_store.backup_busy) else {
      return;
    };
    importing.set(true);
    let alive = alive.get_value();
    spawn_local(async move {
      let text = crate::util::read_file_text(&file).await;
      if !alive() {
        return;
      }
      let Some(text) = text else {
        importing.set(false);
        crate::ui::push_error(t("common.file-read-failed"));
        return;
      };
      let result = if merge {
        crate::util::import_backup_merge(&text).await
      } else {
        crate::util::import_backup(&text).await
      };
      log_store.refresh_from_storage().await;
      drop(task);
      if !alive() {
        return;
      }
      importing.set(false);
      match result {
        Ok(n) => {
          // 这两条走的是「中文原文 → key」反向索引，静态扫描看不见，改用语义 key + `tp`
          let msg = if merge {
            tp("settings.merged-items", n as u32, &[&n.to_string()])
          } else {
            tp("tools.restored-records", n as u32, &[&n.to_string()])
          };
          crate::ui::push_success(msg);
        }
        Err(e) => crate::ui::push_error(tf("tools.import-failed", &[&e.to_string()])),
      }
      usage.set(storage::usage());
    });
  };

  let run_export = move |_| {
    let Some(task) = crate::util::begin_backup_task(log_store.backup_busy) else {
      return;
    };
    let modules = BackupModules {
      log: export_log.get_untracked(),
      qsl_images: export_qsl.get_untracked(),
      other: export_other.get_untracked(),
    };
    exporting.set(true);
    let alive = alive.get_value();
    spawn_local(async move {
      let result = crate::util::export_backup(modules).await;
      drop(task);
      if !alive() {
        return;
      }
      match result {
        Ok(n) => crate::ui::push_success(tp("tools.exported-records", n as u32, &[&n.to_string()])),
        Err(e) => crate::ui::push_error(e),
      }
      exporting.set(false);
    });
  };

  // 词条的字面量写在调用点（静态扫描才认得出）；这里只负责排版一行勾选。
  let module_row = |checked: RwSignal<bool>, label: Signal<String>| {
    view! {
      <li class="flex items-center gap-2">
        <Checkbox
          checked=Signal::derive(move || checked.get())
          on_change=Callback::new(move |v| checked.set(v))
          aria_label=label
        />
        <span class="text-sm">{move || label.get()}</span>
      </li>
    }
  };
  // 三项全不选只会导出一个空 JSON，直接禁用按钮。
  let nothing_selected = move || !(export_log.get() || export_qsl.get() || export_other.get());

  view! {
    <div class="space-y-4">
      <div class="flex flex-wrap items-center gap-2">
        <FileInput
          accept=".json"
          label=t("tools.import-backup")
          variant=Variant::Outline
          disabled=move || importing.get() || exporting.get()
          on_files=Callback::new(move |files: Vec<web_sys::File>| {
            if let Some(f) = files.into_iter().next() {
              run_import(f, false);
            }
          })
        />
        <FileInput
          accept=".json"
          label=t("settings.merge-import")
          variant=Variant::Outline
          disabled=move || importing.get() || exporting.get()
          on_files=Callback::new(move |files: Vec<web_sys::File>| {
            if let Some(f) = files.into_iter().next() {
              run_import(f, true);
            }
          })
        />
      </div>

      <div class="border bg-muted/20 p-3">
        <p class="mb-2 text-xs font-semibold text-muted-foreground">
          {move || t("tools.backup-modules")}
        </p>
        <ul class="space-y-1.5 text-sm">
          {module_row(
            export_log,
            Signal::derive(move || t("tools.backup-module-log")),
          )}
          {module_row(
            export_qsl,
            Signal::derive(move || t("tools.backup-module-qsl-images")),
          )}
          {module_row(
            export_other,
            Signal::derive(move || t("tools.backup-module-other")),
          )}
        </ul>
        <div class="mt-3">
          <Button
            variant=Variant::Default
            disabled=move || importing.get() || exporting.get() || nothing_selected()
            on_click=Callback::new(run_export)
          >
            {move || {
              if exporting.get() {
                t("tools.exporting")
              } else {
                t("tools.export-backup")
              }
            }}
          </Button>
        </div>
      </div>

      {move || {
        let list = usage.get();
        let total: usize = list.iter().map(|(_, n)| n).sum();
        let pct = total as f64 / storage::QUOTA_UNITS as f64 * 100.0;
        let bar = if pct >= 80.0 {
          "bg-red-500"
        } else if pct >= 50.0 {
          "bg-amber-500"
        } else {
          "bg-primary"
        };
        // 归并与排序在 core（`usage::sum_by_group`）：这里只把分组标签翻成文案。
        let groups = sum_by_group(list.iter().map(|(k, n)| (k.as_str(), *n)), key_label);
        view! {
          <div class="space-y-2">
            <div class="flex items-center justify-between text-xs text-muted-foreground">
              {move || t("tools.local-storage-used-approx")}
              <span class="tabular-nums">
                {tf(
                  "tools.characters",
                  &[
                    &format_units(total),
                    &format_units(storage::QUOTA_UNITS),
                    &format!("{pct:.1}"),
                  ],
                )}
              </span>
            </div>
            <div class="h-2 w-full overflow-hidden bg-muted">
              <div class=format!("h-full {bar}") style=format!("width: {:.1}%", pct.min(100.0))></div>
            </div>
            <ul class="grid gap-x-6 gap-y-1 text-xs sm:grid-cols-2">
              {groups
                .into_iter()
                .take(10)
                .map(|(label, n)| {
                  view! {
                    <li class="flex justify-between">
                      <span>{move || t(label)}</span>
                      <span class="tabular-nums text-muted-foreground">{format_units(n)}</span>
                    </li>
                  }
                })
                .collect_view()}
            </ul>
            {(pct >= 80.0).then(|| view! {
              <p class="text-xs text-red-600 dark:text-red-400">
                {move || t("tools.storage-is-nearly-full")}
              </p>
            })}
          </div>
        }
      }}
    </div>
  }
}

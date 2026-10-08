use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::i18n::{t, tf, tp};
use crate::ui::{Button, FileInput, Variant};
use crate::util::storage;

/// 本地存储 key 的中文说明。
fn key_label(key: &str) -> &'static str {
  match key {
    "logbook" => "通联日志",
    "mistake-book" => "错题本",
    "study-stats" => "累计答题统计",
    "exam-history" => "考试成绩历史",
    "bookmarks" => "收藏题目",
    "station-info" => "本台信息",
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

fn fmt_size(units: usize) -> String {
  if units >= 1024 * 1024 {
    format!("{:.1} M", units as f64 / 1024.0 / 1024.0)
  } else if units >= 1024 {
    format!("{:.1} K", units as f64 / 1024.0)
  } else {
    units.to_string()
  }
}

/// 数据备份：导出 / 导入全部本地数据，并展示各项占用。
#[component]
pub(super) fn BackupTool() -> impl IntoView {
  let usage = RwSignal::new(storage::usage());
  let run_import = move |file: web_sys::File, merge: bool| {
    spawn_local(async move {
      let Some(text) = crate::util::read_file_text(&file).await else {
        // 读失败必须出声：静默返回会被当成「备份里没有数据」。
        crate::util::alert(&t("common.file-read-failed"));
        return;
      };
      let result = if merge {
        crate::util::import_backup_merge(&text)
      } else {
        crate::util::import_backup(&text)
      };
      match result {
        Ok(n) => {
          // 这两条走的是「中文原文 → key」反向索引，静态扫描看不见，改用语义 key + `tp`
          let msg = if merge {
            tp("settings.merged-items", n as u32, &[&n.to_string()])
          } else {
            tp("tools.restored-records", n as u32, &[&n.to_string()])
          };
          crate::util::alert(&msg);
        }
        Err(e) => crate::util::alert(&tf("tools.import-failed", &[&e.to_string()])),
      }
      usage.set(storage::usage());
    });
  };

  view! {
    <div class="space-y-4">
      <div class="flex flex-wrap items-center gap-2">
        <Button variant=Variant::Default on_click=Callback::new(move |_| crate::util::export_backup())>
          {move || t("tools.export-backup")}
        </Button>
        <FileInput
          accept=".json"
          label=t("tools.import-backup")
          variant=Variant::Outline
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
          on_files=Callback::new(move |files: Vec<web_sys::File>| {
            if let Some(f) = files.into_iter().next() {
              run_import(f, true);
            }
          })
        />
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
        let mut groups: Vec<(&'static str, usize)> = Vec::new();
        for (k, n) in &list {
          let label = key_label(k);
          match groups.iter_mut().find(|(l, _)| *l == label) {
            Some(g) => g.1 += n,
            None => groups.push((label, *n)),
          }
        }
        groups.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        view! {
          <div class="space-y-2">
            <div class="flex items-center justify-between text-xs text-muted-foreground">
              {move || t("tools.local-storage-used-approx")}
              <span class="tabular-nums">
                {tf(
                  "tools.characters",
                  &[
                    &fmt_size(total),
                    &fmt_size(storage::QUOTA_UNITS),
                    &format!("{pct:.1}"),
                  ],
                )}
              </span>
            </div>
            <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
              <div class=format!("h-full rounded-full {bar}") style=format!("width: {:.1}%", pct.min(100.0))></div>
            </div>
            <ul class="grid gap-x-6 gap-y-1 text-xs sm:grid-cols-2">
              {groups
                .into_iter()
                .take(10)
                .map(|(label, n)| {
                  view! {
                    <li class="flex justify-between">
                      <span>{move || t(label)}</span>
                      <span class="tabular-nums text-muted-foreground">{fmt_size(n)}</span>
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

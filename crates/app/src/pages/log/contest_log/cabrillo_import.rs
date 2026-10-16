use ham_web_core::cabrillo_import::{MAX_BYTES, Skipped, drop_logged, parse_cabrillo};
use ham_web_core::contest::{Exch, contest};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::i18n::{t, tf, tp};
use crate::pages::log::use_log_store;
use crate::ui::{FileInput, Size, Variant};
use crate::util::storage;

use super::session::Session;
use super::view::SESSION_KEY;

/// 一次导入的结果：只存原始数据，文案在面板里按当前语言现渲染。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ImportReport {
  /// `CONTEST` 头（可能为空）。
  pub(super) contest: String,
  /// 内置模板的竞赛名词条 key；模板外为 `None`。
  pub(super) contest_name_key: Option<&'static str>,
  pub(super) added: usize,
  /// 日志里已有而跳过的条数（宽松键 + 精确键两道合计）。
  pub(super) dupes: usize,
  pub(super) skipped_total: usize,
  pub(super) samples: Vec<Skipped>,
  pub(super) x_qso: usize,
  /// 竞赛不在内置模板里（列布局是猜的）。
  pub(super) assumed_layout: bool,
  /// 导入后切换了竞赛录入的场次：新的开始时间（`YYYY-MM-DD HH:MM`，UTC）。
  pub(super) switched_start: Option<String>,
}

/// 「导入 Cabrillo」按钮：读文件 → 解析 → 查重 → 写入日志，并把结果交给面板。
#[component]
pub(super) fn CabrilloImportButton(
  session: RwSignal<Session>,
  report: RwSignal<Option<ImportReport>>,
) -> impl IntoView {
  let store = use_log_store();
  let mounted = crate::util::mount_guard();

  let on_files = Callback::new(move |files: Vec<web_sys::File>| {
    let Some(file) = files.into_iter().next() else {
      return;
    };
    // 先看大小：真实的竞赛日志在 MB 以内，太大多半选错了文件，不必读进内存。
    if file.size() > MAX_BYTES as f64 {
      report.set(None);
      crate::ui::push_error(tf(
        "contest.cabrillo-too-large",
        &[&(MAX_BYTES / (1024 * 1024)).to_string()],
      ));
      return;
    }
    let mounted = mounted.clone();
    spawn_local(async move {
      let text = crate::util::read_file_text(&file).await;
      if !mounted() {
        return;
      }
      let Some(text) = text else {
        crate::ui::push_error(t("common.file-read-failed"));
        return;
      };
      let cab = parse_cabrillo(&text);
      if !cab.recognized() {
        report.set(None);
        crate::ui::push_error(t("contest.cabrillo-not-cabrillo"));
        return;
      }
      let scope = cab.scope();
      let (fresh, known) = store.station.with_untracked(|book| {
        store
          .logbook
          .with_untracked(|lb| drop_logged(cab.records.clone(), &lb.entries, book))
      });
      // `None` = 备份 / 恢复进行中（`import` 已提示用户）。
      let Some((added, dupes)) = store.import(fresh) else {
        return;
      };

      // 模板里有这个竞赛才切场次：竞赛录入页只显示「竞赛 ID 一致、且不早于本场开始时间」的通联，
      // 导入的通联多半是过去的比赛，不切就会导入成功却一条也看不到。
      let def = contest(&cab.contest);
      let mut switched_start = None;
      if added > 0
        && let (Some(def), Some(scope)) = (def, scope.as_ref())
      {
        let station_id = store.station.with_untracked(|book| {
          let id = book.clone().claim(&scope.callsign, &scope.gridsquare);
          book.by_id(id).map(|p| p.id)
        });
        if let Some(station_id) = station_id {
          session.update(|s| {
            // 只有台站、竞赛及时间窗重叠时扩展快照，不能把历史年份混进当前场次。
            let same = s.contest_id == def.id
              && s.station_id == station_id
              && !s.end.is_empty()
              && s.start < scope.end
              && scope.start < s.end;
            s.contest_id = def.id.to_owned();
            s.station_id = station_id;
            s.start = if same {
              s.start.clone().min(scope.start.clone())
            } else {
              scope.start.clone()
            };
            s.end = if same {
              s.end.clone().max(scope.end.clone())
            } else {
              scope.end.clone()
            };
            if !same {
              s.my_exch = if def.sent == Exch::Serial {
                String::new()
              } else {
                scope.exchange.clone()
              };
            }
          });
          storage::set_json(SESSION_KEY, &session.get_untracked());
          switched_start = Some(session.with_untracked(|s| s.start.clone()));
        }
      } else if added > 0 && def.is_some() && scope.is_none() {
        crate::ui::push_error(t("contest.cabrillo-split-session"));
      }

      let r = ImportReport {
        contest: cab.contest.clone(),
        contest_name_key: def.map(|d| d.name),
        added,
        dupes: known + dupes,
        skipped_total: cab.skipped_total,
        samples: cab.skipped.clone(),
        x_qso: cab.x_qso,
        assumed_layout: cab.assumed_layout,
        switched_start,
      };
      let label = contest_label(&r);
      crate::ui::push_success(tp(
        "contest.cabrillo-imported",
        added,
        &[&added.to_string(), &label],
      ));
      report.set(Some(r));
    });
  });

  view! {
    <FileInput
      on_files=on_files
      accept=".cbr,.log,.txt,text/plain"
      variant=Variant::Outline
      size=Size::Sm
      label=Signal::derive(move || t("contest.import-cabrillo"))
    />
  }
}

/// 面板和提示里显示的竞赛名：有模板用译名，否则原样显示头信息，都没有就说「未标明」。
pub(super) fn contest_label(r: &ImportReport) -> String {
  match (r.contest_name_key, r.contest.as_str()) {
    (Some(key), _) => t(key),
    (None, "") => t("contest.cabrillo-no-contest-name"),
    (None, id) => id.to_owned(),
  }
}

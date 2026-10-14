//! 统计口径选择器（全部 / 某个台站 / 某个操作员）：日志页统计面板、`/stats`、
//! `/dxcc-map`、`/report`、`/grid-map` 共用同一个控件与同一份持久化值 —— 在一处选了「野外台站」，
//! 别的统计页也按它算，不会出现「日志页说 12 个实体、地图上是 30 个」。

use leptos::prelude::*;

use ham_web_core::log_stats::{StatsScope, operators, scoped_entries};

use super::LogEntry;
use super::log_helpers::station_title;
use super::use_log_store;
use crate::i18n::{t, tp};
use crate::ui::{NativeSelect, SelectOption};

/// 统计口径的持久化 key（视图偏好，不进备份正文）。
const SCOPE_KEY: &str = "log-stats-scope";

/// 读出持久化的统计口径（短串，见 [`StatsScope::key`]）。
pub fn use_stats_scope() -> RwSignal<String> {
  RwSignal::new(crate::util::storage::get(SCOPE_KEY).unwrap_or_default())
}

/// 按口径过滤后的通联（跟随日志、台站档案与口径变化）。
///
/// 上层统计（`AwardProgress::from_entries`、`overview`…）都吃 `&[LogEntry]`，
/// 这里克隆一份子集交给它们，见 [`scoped_entries`]。
pub fn scoped_log_entries(scope: RwSignal<String>) -> Memo<Vec<LogEntry>> {
  let store = use_log_store();
  Memo::new(move |_| {
    let raw = StatsScope::from_key(&scope.get());
    store.logbook.with(|lb| {
      store.station.with(|b| {
        // 持久化的口径可能指向已删的台站 / 已不存在的操作员，先剔掉失效片段（见 `live`）。
        let parsed = raw.live(&lb.entries, b);
        scoped_entries(&lb.entries, b, &parsed)
      })
    })
  })
}

/// 口径下拉：只有一个台站且只有一个操作员时不渲染（没得选）。
///
/// `count` 是当前口径下的通联条数；口径不是「全部」时在下拉旁给出。
#[component]
pub fn StatsScopeSelect(
  scope: RwSignal<String>,
  #[prop(into)] count: Signal<usize>,
) -> impl IntoView {
  let store = use_log_store();
  let set_scope = Callback::new(move |key: String| {
    crate::util::storage::set(SCOPE_KEY, &key);
    scope.set(key);
  });
  move || {
    let profiles = store.station.with(|b| {
      b.profiles
        .iter()
        .map(|p| (p.id, station_title(p)))
        .collect::<Vec<_>>()
    });
    let ops = store
      .logbook
      .with(|lb| store.station.with(|b| operators(&lb.entries, b)));
    (profiles.len() > 1 || ops.len() > 1).then(|| {
      // 选项带分组（台站 / 操作员两个面），与原生 `optgroup` 对应。
      let mut options: Vec<SelectOption> = Vec::new();
      for (id, title) in &profiles {
        options.push(SelectOption::grouped(
          t("log.stats-scope-by-station"),
          StatsScope::of_station(*id).key(),
          title.clone(),
        ));
      }
      for op in &ops {
        options.push(SelectOption::grouped(
          t("log.stats-scope-by-operator"),
          StatsScope::of_operator(op).key(),
          op.clone(),
        ));
      }
      view! {
        <div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <span>{move || t("log.stats-scope")}</span>
          <NativeSelect
            value=Signal::derive(move || scope.get())
            on_change=set_scope
            options=options
            // 「全部」这个说法词典里已经有了（`exam.all`）：中文释义全库唯一，
            // 同义的 key 只能复用一个，不能各造一份。
            placeholder=Signal::derive(move || t("exam.all"))
            aria_label=Signal::derive(move || t("log.stats-scope"))
            class="w-40"
          />
          {move || {
            // 与 `scoped_log_entries` 同口径：失效的口径按「全部」算，也就不显示条数。
            let active = store.logbook.with(|lb| {
              store
                .station
                .with(|b| !StatsScope::from_key(&scope.get()).live(&lb.entries, b).is_all())
            });
            active
              .then(|| {
                let n = count.get();
                view! { <span>{tp("log.stats-scope-count", n, &[&n.to_string()])}</span> }
              })
          }}
        </div>
      }
    })
  }
}

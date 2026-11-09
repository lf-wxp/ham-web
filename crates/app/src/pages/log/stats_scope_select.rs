//! 统计口径选择器（全部 / 某个台站 / 某个操作员）：日志页统计面板、`/stats`、
//! `/dxcc-map`、`/report`、`/grid-map` 共用同一个控件与同一份持久化值 —— 在一处选了「野外台站」，
//! 别的统计页也按它算，不会出现「日志页说 12 个实体、地图上是 30 个」。

use leptos::prelude::*;

use ham_web_core::log_stats::{StatsScope, operators};

use super::LogEntry;
use super::derived::{SCOPE_KEY, build_scoped};
use super::log_helpers::station_title;
use super::use_log_store;
use crate::i18n::{t, tp};
use crate::ui::{SelectOption, SimpleSelect};

/// 当前的统计口径（短串，见 [`StatsScope::key`]）。
///
/// 全站共用 `LogStore::derived` 上的同一个信号：在一处改了口径，别处立刻跟着变。
///
/// 每次取用时再对一次持久化的值（一次 `localStorage` 读，与改前「每进一次页面读一次」同口径）：
/// 另一个标签页改了口径，换到本页就跟上；值没变不写信号，不惊动订阅者。
pub fn use_stats_scope() -> RwSignal<String> {
  let scope = use_log_store().derived.scope;
  let stored = crate::util::storage::get(SCOPE_KEY).unwrap_or_default();
  crate::debounced::set_if_changed(scope, stored);
  scope
}

/// 按口径过滤后的通联（跟随日志、台站档案与口径变化）。
///
/// 上层统计（`AwardProgress::from_entries`、`overview`…）都吃 `&[LogEntry]`，
/// 这里克隆一份子集交给它们，见 [`ham_web_core::log_stats::scoped_entries`]。
/// 传入的是共享口径（[`use_stats_scope`] 的返回值）时直接给出会话级缓存的那一份。
pub fn scoped_log_entries(scope: RwSignal<String>) -> Memo<Vec<LogEntry>> {
  let store = use_log_store();
  if scope == store.derived.scope {
    return store.derived.scoped;
  }
  build_scoped(store.logbook, store.station, scope)
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
          <SimpleSelect
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

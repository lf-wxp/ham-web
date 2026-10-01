//! 学习进度仪表盘：练习 / 考试 / 错题 / 收藏 / 日志 / 打卡 / DXCC 的统计总览。

use std::collections::HashSet;

use ham_web_core::Bank;
use ham_web_core::categories::top_category;
use ham_web_core::mistake_book::StudyStats;
use ham_web_core::most_wanted::{WANTED_ENTITIES, wanted_prefix};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::components::study_plan_card::StudyPlanCard;
use crate::exam_history::ExamTrend;
use crate::pages::log::use_log_store;
use crate::ui::Stat;
use crate::util::{now_ms, set_title, storage};
use crate::{data, store, study};

/// 打卡状态（精简）。
#[derive(Deserialize, Clone, Default)]
struct CheckinLite {
  #[serde(default)]
  streak: usize,
}

/// 分类正确率统计。
#[derive(Clone, PartialEq)]
struct CategoryStat {
  key: &'static str,
  name: String,
  correct: usize,
  total: usize,
}

#[component]
pub fn ProgressPage() -> impl IntoView {
  set_title("学习进度");

  // 同步统计。
  let checkin: CheckinLite = storage::get_json("daily-checkin").unwrap_or_default();
  let bookmarks = store::load_bookmarks().len();
  let entries = use_log_store().logbook.get_untracked().entries;
  let log_count = entries.len();
  let logged_dxcc: HashSet<String> = entries
    .iter()
    .filter_map(|e| wanted_prefix(&e.callsign).map(str::to_owned))
    .collect();
  let done: Vec<String> = storage::get_json("dxcc_wanted_done").unwrap_or_default();
  let dxcc_done = WANTED_ENTITIES
    .iter()
    .filter(|(p, _, _)| done.iter().any(|x| x.as_str() == *p) || logged_dxcc.contains(*p))
    .count();
  let dxcc_total = WANTED_ENTITIES.len();
  let grids: HashSet<String> = entries
    .iter()
    .filter_map(|e| {
      if e.gridsquare.is_empty() {
        None
      } else {
        Some(e.gridsquare.clone())
      }
    })
    .collect();

  // 错题与累计答题统计。
  let book = study::load_book();
  let mistakes = book.records.len();
  let due = book.due_count(now_ms());
  let study_stats = StoredValue::new(study::load_stats());
  let answered_total = RwSignal::new(study_stats.with_value(|s| s.total.answered) as usize);
  // 当前查看的题库（`None` 为全部），默认作答最多的题库。
  let main_bank = study_stats.with_value(StudyStats::main_bank);
  let bank_tab = RwSignal::new(main_bank);
  let exam_bank = RwSignal::new(main_bank.unwrap_or_default());
  let exam_records = StoredValue::new(crate::exam_history::load());

  // 分类正确率：按正确率升序（薄弱在前）。
  let category_stats = Memo::new(move |_| {
    let bank = bank_tab.get();
    let mut stats: Vec<CategoryStat> = study_stats.with_value(|s| {
      s.categories_of(bank)
        .map(|c| {
          c.categories
            .iter()
            .filter_map(|(key, t)| {
              let top = top_category(key)?;
              Some(CategoryStat {
                key: top.key,
                name: top.name.to_owned(),
                correct: t.correct as usize,
                total: t.answered as usize,
              })
            })
            .filter(|s| s.total > 0)
            .collect()
        })
        .unwrap_or_default()
    });
    stats.sort_by(|a, b| {
      let ra = a.correct as f64 / a.total as f64;
      let rb = b.correct as f64 / b.total as f64;
      ra.total_cmp(&rb)
    });
    stats
  });

  // 题库覆盖率：最新版题库中做过的题数 / 总题数。
  let coverage = RwSignal::new(Vec::<(Bank, usize, usize)>::new());
  spawn_local(async move {
    let mut out = Vec::new();
    for b in Bank::ALL {
      if let Ok(qs) = data::load_bank(None, b, false).await {
        let covered = study_stats
          .try_with_value(|s| s.bank(b).map_or(0, |bs| bs.covered(qs.iter())))
          .unwrap_or(0);
        out.push((b, covered, qs.len()));
      }
    }
    coverage.try_set(out);
  });

  let tab_class = |on: bool| {
    if on {
      "rounded-md bg-primary px-2.5 py-1 text-xs font-medium text-primary-foreground"
    } else {
      "rounded-md px-2.5 py-1 text-xs font-medium text-muted-foreground hover:bg-accent"
    }
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">"学习进度"</h1>
            <div class="text-xs text-muted-foreground">"练习 · 考试 · 错题 · 收藏 · 日志 · DXCC"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label="连续打卡" value=move || checkin.streak />
          <Stat label="错题数" value=move || mistakes />
          <Stat label="今日待复习" value=move || due />
          <Stat label="累计作答" value=move || answered_total.get() />
          <Stat label="收藏题目" value=move || bookmarks />
          <Stat label="通联日志" value=move || log_count />
          <Stat label="已通联网格" value=move || grids.len() />
          <Stat label="DXCC 稀有度" value=move || dxcc_done />
        </div>

        <section class="rounded-xl border bg-card p-4">
          <h2 class="mb-2 text-sm font-semibold">"DXCC 稀有度进度"</h2>
          <div class="mb-1 flex items-center justify-between text-xs text-muted-foreground">
            <span>"已通联（含日志自动识别）"</span>
            <span class="tabular-nums">{format!("{dxcc_done} / {dxcc_total}")}</span>
          </div>
          <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
            <div
              class="h-full rounded-full bg-primary transition-all"
              style=format!("width: {:.1}%", dxcc_done as f64 / dxcc_total as f64 * 100.0)
            ></div>
          </div>
          <div class="mt-3 flex flex-wrap gap-x-4 gap-y-1">
            <a href="/most-wanted" class="text-xs text-muted-foreground underline-offset-4 hover:underline">
              "前往 DXCC 稀有度追踪 →"
            </a>
            <a href="/log#awards" class="text-xs text-muted-foreground underline-offset-4 hover:underline">
              "DXCC / WAZ / WAC / VUCC 奖状进度 →"
            </a>
          </div>
        </section>

        <StudyPlanCard editable=true />

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">"备考状态"</h2>
            {Bank::ALL
              .into_iter()
              .map(|b| view! {
                <button type="button" class=move || tab_class(exam_bank.get() == b) on:click=move |_| exam_bank.set(b)>
                  {format!("{b} 类")}
                </button>
              })
              .collect_view()}
          </div>
          <div class="p-4">
            {move || {
              let bank = exam_bank.get();
              view! { <ExamTrend history=exam_records.get_value() bank=bank /> }
            }}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"题库覆盖率"</h2>
          <div class="space-y-3 p-4">
            {move || {
              let list = coverage.get();
              if list.is_empty() {
                return view! { <p class="text-sm text-muted-foreground">"加载题库中..."</p> }.into_any();
              }
              list
                .into_iter()
                .map(|(b, covered, total)| {
                  let pct = covered as f64 / total.max(1) as f64 * 100.0;
                  let left = total.saturating_sub(covered);
                  view! {
                    <div>
                      <div class="mb-1 flex items-center justify-between text-xs">
                        <span class="font-medium">
                          {format!("{b} 类")}
                          {(left > 0 && covered > 0).then(|| view! {
                            <a
                              href=format!("/practice?bank={b}&unseen=1")
                              class="ml-1.5 rounded border px-1.5 py-0.5 text-[10px] text-primary transition-colors hover:bg-primary/10"
                            >
                              {format!("练没做过的 {left} 题")}
                            </a>
                          })}
                        </span>
                        <span class="tabular-nums text-muted-foreground">
                          {format!("{covered} / {total} · {pct:.0}%")}
                        </span>
                      </div>
                      <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
                        <div class="h-full rounded-full bg-primary transition-all" style=format!("width: {pct:.1}%")></div>
                      </div>
                    </div>
                  }
                })
                .collect_view()
                .into_any()
            }}
            <p class="text-xs text-muted-foreground">"统计练习、模拟考试与闪卡中做过的题（按当前最新题库计算）。"</p>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">"薄弱知识点分析"</h2>
            <button type="button" class=move || tab_class(bank_tab.get().is_none()) on:click=move |_| bank_tab.set(None)>
              "全部"
            </button>
            {Bank::ALL
              .into_iter()
              .map(|b| view! {
                <button type="button" class=move || tab_class(bank_tab.get() == Some(b)) on:click=move |_| bank_tab.set(Some(b))>
                  {format!("{b} 类")}
                </button>
              })
              .collect_view()}
          </div>
          <div class="space-y-3 p-4">
            {move || {
              let stats = category_stats.get();
              if stats.is_empty() {
                return view! {
                  <p class="text-sm text-muted-foreground">
                    "完成练习或模拟考试后，这里会按分类展示正确率，帮你定位薄弱知识点。"
                  </p>
                }
                .into_any();
              }
              view! {
                <div class="space-y-2.5">
                  {stats
                    .iter()
                    .map(|s| {
                      let rate = s.correct as f64 / s.total.max(1) as f64 * 100.0;
                      let bar = if rate < 50.0 {
                        "bg-red-500"
                      } else if rate < 80.0 {
                        "bg-amber-500"
                      } else {
                        "bg-emerald-500"
                      };
                      let name = s.name.clone();
                      let href = format!(
                        "/practice?bank={}&topic={}",
                        bank_tab.get_untracked().or(main_bank).unwrap_or_default(),
                        js_sys::encode_uri_component(s.key)
                      );
                      view! {
                        <div>
                          <div class="mb-1 flex items-center justify-between text-xs">
                            <span class="font-medium">
                              {name}
                              <a
                                href=href
                                class="ml-1.5 rounded border px-1.5 py-0.5 text-[10px] text-primary transition-colors hover:bg-primary/10"
                              >
                                "练习"
                              </a>
                            </span>
                            <span class="tabular-nums text-muted-foreground">
                              {format!("{} / {} · {:.0}%", s.correct, s.total, rate)}
                            </span>
                          </div>
                          <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
                            <div
                              class=format!("h-full rounded-full {bar}")
                              style=format!("width: {rate:.1}%")
                            ></div>
                          </div>
                        </div>
                      }
                    })
                    .collect_view()}
                </div>
                <p class="text-xs text-muted-foreground">
                  "按正确率升序排列，越靠前越薄弱。数据来自练习与模拟考试的答题记录。"
                </p>
              }
              .into_any()
            }}
          </div>
        </section>

        <p class="text-xs text-muted-foreground">
          "数据来自本地记录（练习 / 考试 / 闪卡 / 收藏 / 日志 / 打卡），无需联网。"
        </p>
      </div>
    </div>
  }
}

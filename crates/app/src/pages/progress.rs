//! 学习进度仪表盘：练习 / 考试 / 错题 / 收藏 / 日志 / 打卡 / DXCC 的统计总览。

use std::collections::{HashMap, HashSet};

use ham_web_core::Bank;
use ham_web_core::ExamRule;
use ham_web_core::achievements::{ACHIEVEMENTS, AchStats, unlocked};
use ham_web_core::categories::{sub_category, top_category, top_pages};
use ham_web_core::mistake_book::StudyStats;
use ham_web_core::most_wanted::{WANTED_ENTITIES, wanted_prefix};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::components::common::StudyHeatmap;
use crate::components::study_plan_card::StudyPlanCard;
use crate::exam_history::ExamTrend;
use crate::i18n::{t, tf};
use crate::pages::log::use_log_store;
use crate::ui::Stat;
use crate::util::{local_today, now_ms, set_title, storage};
use crate::{data, store, study};

/// 打卡状态（精简）。
#[derive(Deserialize, Clone, Default)]
struct CheckinLite {
  #[serde(default)]
  streak: usize,
}

/// 分类码（P 码）正确率，用于下钻。
#[derive(Clone, PartialEq)]
struct SubStat {
  code: String,
  name: String,
  correct: usize,
  total: usize,
}

/// 分类正确率统计。
#[derive(Clone, PartialEq)]
struct CategoryStat {
  key: &'static str,
  name: String,
  correct: usize,
  total: usize,
  subs: Vec<SubStat>,
}

#[component]
pub fn ProgressPage() -> impl IntoView {
  set_title("shell.progress");

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
  let grid_count = grids.len();

  // 错题与累计答题统计。
  let book = study::load_book();
  let mistakes = book.records.len();
  let due = book.due_count(now_ms());
  let study_stats = StoredValue::new(study::load_stats());
  // 累计学习时长（分钟，取整展示）与每日作答热力图数据。
  let daily = study::load_daily();
  let total_minutes = (daily.days.values().map(|t| t.duration_ms).sum::<u64>() / 60_000) as usize;
  let heatmap_days: HashMap<String, u32> = daily
    .days
    .iter()
    .map(|(k, t)| (k.clone(), t.answered))
    .collect();
  let answered_total = RwSignal::new(study_stats.with_value(|s| s.total.answered) as usize);
  // 当前查看的题库（`None` 为全部），默认作答最多的题库。
  let main_bank = study_stats.with_value(StudyStats::main_bank);
  let bank_tab = RwSignal::new(main_bank);
  let exam_bank = RwSignal::new(main_bank.unwrap_or_default());
  let exam_records = StoredValue::new(crate::exam_history::load());

  // 分类正确率：按正确率升序（薄弱在前），含二级分类（P 码）下钻。
  let category_stats = Memo::new(move |_| {
    let bank = bank_tab.get();
    let mut stats: Vec<CategoryStat> = study_stats.with_value(|s| {
      s.categories_of(bank)
        .map(|c| {
          c.categories
            .iter()
            .filter_map(|(key, t)| {
              let top = top_category(key)?;
              let mut subs: Vec<SubStat> = c
                .subs_of(top.key)
                .into_iter()
                .filter(|(_, t)| t.answered > 0)
                .map(|(code, t)| SubStat {
                  code: code.to_owned(),
                  name: sub_category(code).map_or_else(|| code.to_owned(), |s| s.name.to_owned()),
                  correct: t.correct as usize,
                  total: t.answered as usize,
                })
                .collect();
              subs.sort_by(|a, b| {
                let ra = a.correct as f64 / a.total.max(1) as f64;
                let rb = b.correct as f64 / b.total.max(1) as f64;
                ra.total_cmp(&rb)
              });
              Some(CategoryStat {
                key: top.key,
                name: top.name.to_owned(),
                correct: t.correct as usize,
                total: t.answered as usize,
                subs,
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

  // 成就判定。
  let unlocked_ids = Memo::new(move |_| {
    let cov = coverage.get();
    let covs: Vec<f64> = cov
      .iter()
      .map(|(_, c, t)| *c as f64 / (*t).max(1) as f64)
      .collect();
    let passed = exam_records.with_value(|h| {
      h.iter()
        .any(|r| !r.weak && r.correct >= ExamRule::of(Bank::from_param(Some(&r.bank))).pass)
    });
    let perfect = exam_records.with_value(|h| {
      h.iter()
        .any(|r| !r.weak && r.total > 0 && r.correct == r.total)
    });
    let challenge_days =
      storage::get_json::<ham_web_core::daily_challenge::DailyResults>("daily-challenge")
        .map_or(0, |r| r.days.len());
    let contest_count = entries.iter().filter(|e| !e.contest_id.is_empty()).count();
    let s = AchStats {
      total_correct: study_stats.with_value(|s| s.total.correct),
      coverages: &covs,
      streak: checkin.streak,
      exam_passed: passed,
      exam_perfect: perfect,
      challenge_count: challenge_days,
      dxcc_count: dxcc_done,
      log_count,
      grid_count,
      contest_count,
      bookmarks,
    };
    unlocked(&s)
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
            <h1 class="text-base font-semibold leading-tight">{move || t("shell.progress")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("learning.practice-exam-mistakes-bookmarks")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label=t("learning.day-streak") value=move || checkin.streak />
          <Stat label=t("learning.mistakes") value=move || mistakes />
          <Stat label=t("home.due-for-review") value=move || due />
          <Stat label=t("learning.answered-total") value=move || answered_total.get() />
          <Stat label=t("learning.bookmarks") value=move || bookmarks />
          <Stat label=t("shell.logbook") value=move || log_count />
          <Stat label=t("learning.grids-worked") value=move || grid_count />
          <Stat label=t("learning.dxcc-rarity") value=move || dxcc_done />
          <Stat label=t("learning.total-study-min") value=move || total_minutes />
        </div>

        <section class="rounded-xl border bg-card p-4">
          <h2 class="mb-3 text-sm font-semibold">{move || t("learning.daily-study-check-in")}</h2>
          <StudyHeatmap days=heatmap_days today=local_today() />
        </section>

        <section class="rounded-xl border bg-card p-4">
          <h2 class="mb-2 text-sm font-semibold">{move || t("learning.dxcc-rarity-progress")}</h2>
          <div class="mb-1 flex items-center justify-between text-xs text-muted-foreground">
            <span>{move || t("learning.worked-incl-auto-detected")}</span>
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
              {move || t("learning.go-to-dxcc-rarity")}
            </a>
            <a href="/log#awards" class="text-xs text-muted-foreground underline-offset-4 hover:underline">
              {move || t("learning.dxcc-waz-wac-vucc")}
            </a>
          </div>
        </section>

        <StudyPlanCard editable=true />

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("learning.prep-status")}</h2>
            {Bank::ALL
              .into_iter()
              .map(|b| view! {
                <button type="button" class=move || tab_class(exam_bank.get() == b) on:click=move |_| exam_bank.set(b)>
                  {tf("learning.class", &[&b.to_string()])}
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
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("learning.bank-coverage")}</h2>
          <div class="space-y-3 p-4">
            {move || {
              let list = coverage.get();
              if list.is_empty() {
                return view! { <p class="text-sm text-muted-foreground">{move || t("exam.loading-questions-u-2026")}</p> }.into_any();
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
                          {tf("learning.class", &[&b.to_string()])}
                          {(left > 0 && covered > 0).then(|| view! {
                            <a
                              href=format!("/practice?bank={b}&unseen=1")
                              class="ml-1.5 rounded border px-1.5 py-0.5 text-[10px] text-primary transition-colors hover:bg-primary/10"
                            >
                              {tf("learning.practice-the-unseen", &[&left.to_string()])}
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
            <p class="text-xs text-muted-foreground">{move || t("learning.counts-questions-done-in")}</p>
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("learning.achievements")}</h2>
          <div class="grid grid-cols-2 gap-2 p-4 sm:grid-cols-4">
            {ACHIEVEMENTS
              .iter()
              .map(|a| {
                let is_unlocked = move || unlocked_ids.with(|ids| ids.contains(&a.id));
                view! {
                  <div class=move || if is_unlocked() {
                    "rounded-lg border p-3 text-center"
                  } else {
                    "rounded-lg border border-dashed p-3 text-center text-muted-foreground"
                  }>
                    <div class="text-2xl">{a.icon}</div>
                    <div class="mt-1 text-xs font-medium">{move || t(a.name)}</div>
                    <div class="mt-0.5 text-[10px] text-muted-foreground">{move || t(a.desc)}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
            <h2 class="mr-auto text-sm font-semibold">{move || t("learning.weak-area-analysis")}</h2>
            <button type="button" class=move || tab_class(bank_tab.get().is_none()) on:click=move |_| bank_tab.set(None)>
              {move || t("exam.all")}
            </button>
            {Bank::ALL
              .into_iter()
              .map(|b| view! {
                <button type="button" class=move || tab_class(bank_tab.get() == Some(b)) on:click=move |_| bank_tab.set(Some(b))>
                  {tf("learning.class", &[&b.to_string()])}
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
                    {move || t("learning.after-practice-or-a")}
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
                      let bank = bank_tab.get_untracked().or(main_bank).unwrap_or_default();
                      let name = s.name.clone();
                      let href = format!(
                        "/practice?bank={bank}&topic={}",
                        js_sys::encode_uri_component(s.key)
                      );
                      let topics = top_pages(s.key);
                      let subs = s.subs.clone();
                      view! {
                        <div class="rounded-lg border p-2.5">
                          <div class="mb-1 flex items-center justify-between gap-2 text-xs">
                            <span class="flex min-w-0 flex-wrap items-center gap-1.5 font-medium">
                              <span>{name}</span>
                              <a
                                href=href
                                class="rounded border px-1.5 py-0.5 text-[10px] text-primary transition-colors hover:bg-primary/10"
                              >
                                {move || t("shell.practice")}
                              </a>
                              {topics
                                .iter()
                                .map(|(route, label)| {
                                  view! {
                                    <a
                                      href=*route
                                      class="rounded border px-1.5 py-0.5 text-[10px] text-muted-foreground transition-colors hover:bg-accent"
                                    >
                                      {*label}
                                    </a>
                                  }
                                })
                                .collect_view()}
                            </span>
                            <span class="shrink-0 tabular-nums text-muted-foreground">
                              {format!("{} / {} · {:.0}%", s.correct, s.total, rate)}
                            </span>
                          </div>
                          <div class="h-2 w-full overflow-hidden rounded-full bg-muted">
                            <div
                              class=format!("h-full rounded-full {bar}")
                              style=format!("width: {rate:.1}%")
                            ></div>
                          </div>
                          {(!subs.is_empty()).then(|| {
                            view! {
                              <div class="mt-2 space-y-1.5 border-t pt-2">
                                {subs
                                  .iter()
                                  .map(|sub| {
                                    let sub_rate = sub.correct as f64 / sub.total.max(1) as f64 * 100.0;
                                    let sub_bar = if sub_rate < 50.0 {
                                      "bg-red-400"
                                    } else if sub_rate < 80.0 {
                                      "bg-amber-400"
                                    } else {
                                      "bg-emerald-400"
                                    };
                                    let sub_href = format!(
                                      "/practice?bank={bank}&sub={}",
                                      js_sys::encode_uri_component(&sub.code)
                                    );
                                    view! {
                                      <div>
                                        <div class="flex items-center justify-between gap-2 text-[11px]">
                                          <span class="flex min-w-0 items-center gap-1 text-muted-foreground">
                                            <span class="shrink-0 font-mono text-[10px] text-muted-foreground">
                                              {sub.code.clone()}
                                            </span>
                                            <span class="truncate" title=sub.name.clone()>
                                              {sub.name.clone()}
                                            </span>
                                            <a
                                              href=sub_href
                                              class="shrink-0 rounded border px-1 py-px text-[10px] text-primary transition-colors hover:bg-primary/10"
                                            >
                                              {move || t("learning.practice")}
                                            </a>
                                          </span>
                                          <span class="shrink-0 tabular-nums text-muted-foreground">
                                            {format!("{}/{} · {:.0}%", sub.correct, sub.total, sub_rate)}
                                          </span>
                                        </div>
                                        <div class="ml-4 mt-0.5 h-1 w-[calc(100%-1rem)] overflow-hidden rounded-full bg-muted">
                                          <div
                                            class=format!("h-full rounded-full {sub_bar}")
                                            style=format!("width: {sub_rate:.1}%")
                                          ></div>
                                        </div>
                                      </div>
                                    }
                                  })
                                  .collect_view()}
                              </div>
                            }
                          })}
                        </div>
                      }
                    })
                    .collect_view()}
                </div>
                <p class="text-xs text-muted-foreground">
                  {move || t("learning.sorted-by-accuracy-ascending")}
                </p>
              }
              .into_any()
            }}
          </div>
        </section>

        <p class="text-xs text-muted-foreground">
          {move || t("learning.data-comes-from-local-2")}
        </p>
      </div>
    </div>
  }
}

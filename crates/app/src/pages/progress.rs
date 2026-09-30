//! 学习进度仪表盘：练习 / 考试 / 错题 / 收藏 / 日志 / 打卡 / DXCC 的统计总览。

use std::collections::{HashMap, HashSet};

use ham_web_core::Bank;
use ham_web_core::categories::{top_category, top_of};
use ham_web_core::mistakes::{Mistake, mistakes_from_state};
use ham_web_core::most_wanted::{WANTED_ENTITIES, wanted_prefix};
use ham_web_core::question::QuestionItem;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::data;
use crate::store;
use crate::ui::{Size, Stat, Variant, button_class, input_class};
use crate::util::set_title;
use crate::util::storage;

/// 打卡状态（精简）。
#[derive(Deserialize, Clone, Default)]
struct CheckinLite {
  #[serde(default)]
  streak: usize,
}

/// 日志精简结构。
#[derive(Deserialize, Default)]
struct LogbookLite {
  #[serde(default)]
  entries: Vec<LogEntryLite>,
}

#[derive(Deserialize, Default)]
struct LogEntryLite {
  #[serde(default)]
  callsign: String,
  #[serde(default)]
  gridsquare: String,
}

/// 分类正确率统计。
#[derive(Clone)]
struct CategoryStat {
  key: &'static str,
  name: String,
  correct: usize,
  total: usize,
}

/// 按一级分类累计答题正确率（key = 一级分类 key）。
fn accumulate_category(
  map: &mut HashMap<&'static str, (usize, usize)>,
  qs: &[QuestionItem],
  order_indices: &[usize],
  answers_by_position: &[Option<Vec<String>>],
) {
  for (pos, ans) in answers_by_position.iter().enumerate() {
    let Some(ans) = ans.as_ref().filter(|a| !a.is_empty()) else {
      continue;
    };
    let Some(qi) = order_indices.get(pos).and_then(|&i| qs.get(i)) else {
      continue;
    };
    let Some(top) = qi.p_code().and_then(top_of) else {
      continue;
    };
    let entry = map.entry(top.key).or_default();
    entry.1 += 1;
    if qi.is_answer_correct(ans) {
      entry.0 += 1;
    }
  }
}

/// 学习计划：目标日期与目标题数。
#[derive(Serialize, Deserialize, Clone, Default)]
struct LearningPlan {
  #[serde(default)]
  target_date: String,
  #[serde(default)]
  target_count: usize,
}

const PLAN_KEY: &str = "learning-plan";

/// 剩余天数（从今天到目标日期，向上取整）。
fn days_remaining(target_date: &str) -> Option<i64> {
  if target_date.is_empty() {
    return None;
  }
  let target = js_sys::Date::parse(&format!("{target_date}T00:00:00"));
  if target.is_nan() {
    return None;
  }
  let now = js_sys::Date::now();
  let diff = target - now;
  Some((diff / 86_400_000.0).ceil() as i64)
}

#[component]
pub fn ProgressPage() -> impl IntoView {
  set_title("学习进度");

  // 同步统计。
  let checkin: CheckinLite = storage::get_json("daily-checkin").unwrap_or_default();
  let bookmarks = store::load_bookmarks().len();
  let logbook: LogbookLite = storage::get_json("logbook").unwrap_or_default();
  let log_count = logbook.entries.len();
  let logged_dxcc: HashSet<String> = logbook
    .entries
    .iter()
    .filter_map(|e| wanted_prefix(&e.callsign).map(str::to_owned))
    .collect();
  let done: Vec<String> = storage::get_json("dxcc_wanted_done").unwrap_or_default();
  let dxcc_done = WANTED_ENTITIES
    .iter()
    .filter(|(p, _, _)| done.iter().any(|x| x.as_str() == *p) || logged_dxcc.contains(*p))
    .count();
  let dxcc_total = WANTED_ENTITIES.len();
  let grids: HashSet<String> = logbook
    .entries
    .iter()
    .filter_map(|e| {
      if e.gridsquare.is_empty() {
        None
      } else {
        Some(e.gridsquare.clone())
      }
    })
    .collect();

  // 错题数（异步汇总练习与考试）。
  let mistakes = RwSignal::new(0usize);
  let mistakes_loading = RwSignal::new(true);
  let category_stats = RwSignal::new(Vec::<CategoryStat>::new());
  let answered_total = RwSignal::new(0usize);
  let plan: LearningPlan = storage::get_json(PLAN_KEY).unwrap_or_default();
  let plan_date = RwSignal::new(plan.target_date.clone());
  let plan_count = RwSignal::new(plan.target_count);
  spawn_local(async move {
    let mut all: Vec<Mistake> = Vec::new();
    let mut cat_map: HashMap<&'static str, (usize, usize)> = HashMap::new();
    let versions = data::all_versions(false).await.unwrap_or_default();
    let mut version_ids: Vec<Option<String>> = vec![None];
    version_ids.extend(versions.iter().map(|v| Some(v.id.clone())));
    for vid in version_ids {
      let vid_ref = vid.as_deref();
      for bank in Bank::ALL {
        if let Some(state) = store::load_practice(bank, vid_ref)
          && let Ok(qs) = data::load_bank(vid_ref, bank, false).await
        {
          all.extend(mistakes_from_state(
            &qs,
            &state.order_indices,
            &state.answers_by_position,
          ));
          accumulate_category(
            &mut cat_map,
            &qs,
            &state.order_indices,
            &state.answers_by_position,
          );
        }
        if let Some(state) = store::load_exam(bank, vid_ref)
          && let Ok(qs) = data::load_bank(vid_ref, bank, false).await
        {
          let rebuilt = state.reconstruct(&qs);
          let order: Vec<usize> = (0..rebuilt.len()).collect();
          all.extend(mistakes_from_state(
            &rebuilt,
            &order,
            &state.answers_by_position,
          ));
          accumulate_category(&mut cat_map, &rebuilt, &order, &state.answers_by_position);
        }
      }
    }
    let mut seen: HashSet<String> = HashSet::new();
    all.retain(|m| {
      let key = m
        .question
        .stable_id()
        .unwrap_or_else(|| m.question.question.clone());
      seen.insert(key)
    });
    mistakes.set(all.len());
    mistakes_loading.set(false);

    // 分类正确率：按正确率升序（薄弱在前）。
    let mut stats: Vec<CategoryStat> = cat_map
      .into_iter()
      .map(|(key, (correct, total))| CategoryStat {
        key,
        name: top_category(key)
          .map(|t| t.name.to_owned())
          .unwrap_or_else(|| key.to_owned()),
        correct,
        total,
      })
      .collect();
    stats.sort_by(|a, b| {
      let ra = a.correct as f64 / a.total.max(1) as f64;
      let rb = b.correct as f64 / b.total.max(1) as f64;
      ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
    });
    answered_total.set(stats.iter().map(|s| s.total).sum());
    category_stats.set(stats);
  });

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"学习进度"</div>
            <div class="text-xs text-muted-foreground">"练习 · 考试 · 错题 · 收藏 · 日志 · DXCC"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <Stat label="连续打卡" value=move || checkin.streak />
          <Stat label="错题数" value=move || mistakes.get() />
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
          <a href="/most-wanted" class="mt-3 inline-block text-xs text-muted-foreground underline-offset-4 hover:underline">
            "前往 DXCC 稀有度追踪 →"
          </a>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"薄弱知识点分析"</h2>
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
                        "/practice?bank=B&topic={}",
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

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"每日学习计划"</h2>
          <div class="space-y-3 p-4">
            <div class="grid gap-3 sm:grid-cols-2">
              <label class="flex flex-col gap-1.5 text-sm">
                <span class="text-xs text-muted-foreground">"目标日期（如考试日）"</span>
                <input
                  type="date"
                  prop:value=move || plan_date.get()
                  on:input=move |e| plan_date.set(event_target_value(&e))
                  class=input_class("")
                />
              </label>
              <label class="flex flex-col gap-1.5 text-sm">
                <span class="text-xs text-muted-foreground">"目标题数"</span>
                <input
                  type="number"
                  prop:value=move || plan_count.get().to_string()
                  on:input=move |e| {
                    if let Ok(v) = event_target_value(&e).parse::<usize>() {
                      plan_count.set(v);
                    }
                  }
                  class=input_class("")
                />
              </label>
            </div>
            <button
              type="button"
              class=button_class(Variant::Default, Size::Sm, "")
              on:click=move |_| {
                let p = LearningPlan {
                  target_date: plan_date.get(),
                  target_count: plan_count.get(),
                };
                storage::set_json(PLAN_KEY, &p);
              }
            >
              "保存计划"
            </button>
            {move || {
              let target = plan_count.get();
              let done = answered_total.get();
              match days_remaining(&plan_date.get()) {
                Some(d) if d > 0 && target > 0 => {
                  let per_day = (target as f64 / d as f64).ceil() as usize;
                  let pct = (done as f64 / target as f64 * 100.0).min(100.0);
                  view! {
                    <div class="rounded-lg bg-muted/40 p-3 text-sm">
                      <div>
                        "剩余 " <span class="font-semibold">{d}</span> " 天，建议每天刷 "
                        <span class="font-semibold">{per_day}</span> " 题"
                      </div>
                      <div class="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
                        <div class="h-2 flex-1 overflow-hidden rounded-full bg-muted">
                          <div class="h-full rounded-full bg-primary" style=format!("width: {pct:.1}%")></div>
                        </div>
                        <span class="tabular-nums">
                          {format!("已答 {} / {}", done, target)}
                        </span>
                      </div>
                    </div>
                  }
                  .into_any()
                }
                _ => view! {
                  <p class="text-xs text-muted-foreground">
                    "设定目标日期与题数后，自动计算每日建议题量与完成进度。"
                  </p>
                }
                .into_any(),
              }
            }}
          </div>
        </section>

        <p class="text-xs text-muted-foreground">
          {move || {
            if mistakes_loading.get() {
              "错题数正在汇总…".to_owned()
            } else {
              "数据来自本地进度（练习 / 考试 / 收藏 / 日志 / 打卡），无需联网。".to_owned()
            }
          }}
        </p>
      </div>
    </div>
  }
}

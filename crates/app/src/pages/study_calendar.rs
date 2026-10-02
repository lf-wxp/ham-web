//! 备考日历：把考试日期、每日待复习错题、临考模拟考节奏与打卡整合到「未来 14 天」视图。

use std::time::Duration;

use ham_web_core::study_plan::{MOCK_EXAM_DAYS, day_number, format_day};
use leptos::prelude::*;

use crate::components::study_plan_card::StudyPlanCard;
use crate::i18n::{t, tf};
use crate::study;
use crate::util::{local_today, now_ms, set_title};

const WEEKDAYS: [&str; 7] = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
const DAYS: usize = 14;

/// 星期几（0 = 周日）。1970-01-01 是周四，故 `day_number ≡ -4 (mod 7)` 对应周日。
fn weekday(n: i64) -> usize {
  ((n + 4) % 7 + 7) as usize % 7
}

#[component]
pub fn StudyCalendarPage() -> impl IntoView {
  set_title(&t("备考日历"));

  let today = local_today();
  let plan = RwSignal::new(study::load_plan());
  // 备考计划（考试日期 / 类别）在顶部卡片里可改，这里每 2 秒轮询一次 storage 以同步日历。
  set_interval(
    move || {
      let p = study::load_plan();
      if p != plan.get_untracked() {
        plan.set(p);
      }
    },
    Duration::from_secs(2),
  );

  // 错题到期与打卡为页面进入时的快照（复习进度通常不会在本页内变化）。
  let timeline = study::load_book().due_timeline(now_ms(), DAYS);
  let daily = study::load_daily();

  let today_n = day_number(&today).unwrap_or(0);

  view! {
    <div class="animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-3xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("备考日历")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("未来 14 天的复习节奏与压力，一眼看清")}</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-3xl space-y-4 px-4 py-5">
        <StudyPlanCard editable=true />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("未来 14 天复习日历")}</h2>
          <ul class="divide-y">
            {{
              let target_n = plan.get().days_left(&today).map(|_| {
                day_number(&plan.get().target_date).unwrap_or(0)
              });
              (0..DAYS)
                .map(|i| {
                  let day_n = today_n + i as i64;
                  let day = format_day(day_n);
                  let wd = WEEKDAYS[weekday(day_n)];
                  let due = timeline.get(i).copied().unwrap_or(0);
                  let days_left = target_n.map(|t| t - day_n);
                  let is_exam = days_left == Some(0);
                  let is_over = days_left.is_some_and(|d| d < 0);
                  let mock = days_left.is_some_and(|d| d > 0 && d <= MOCK_EXAM_DAYS);
                  let checked = daily.on(&day).answered > 0;
                  let is_today = i == 0;
                  view! {
                    <li class=if is_exam {
                      "flex items-center gap-3 px-4 py-3 bg-primary/10"
                    } else {
                      "flex items-center gap-3 px-4 py-3"
                    }>
                      <div class="flex w-20 shrink-0 flex-col">
                        <span class=if is_today { "text-sm font-semibold text-primary" } else { "text-sm font-semibold" }>
                          {day[5..].to_owned()}
                        </span>
                        <span class="text-[10px] text-muted-foreground">
                          {if is_today { t("今天") } else { t(wd) }}
                        </span>
                      </div>
                      <div class="flex min-w-0 flex-1 flex-wrap items-center gap-x-3 gap-y-1 text-xs">
                        {is_exam.then(|| view! {
                          <span class="rounded-full bg-primary px-2 py-0.5 font-medium text-primary-foreground">{move || t("考试日")}</span>
                        })}
                        {(!is_exam && days_left.is_some()).then(|| view! {
                          <span class="text-muted-foreground">
                            {if is_over {
                              t("已过")
                            } else {
                              tf("距考试 {} 天", &[&days_left.unwrap_or(0).to_string()])
                            }}
                          </span>
                        })}
                        {mock.then(|| view! {
                          <span class="rounded-full border px-2 py-0.5 text-amber-700 dark:text-amber-400">{move || t("建议模拟考")}</span>
                        })}
                        {checked.then(|| view! {
                          <span class="text-emerald-700 dark:text-emerald-400">{move || t("已打卡")}</span>
                        })}
                      </div>
                      <div class="shrink-0 text-right">
                        {if due > 0 {
                          view! {
                            <div class="text-sm font-semibold text-primary tabular-nums">
                              {tf("{} 道", &[&due.to_string()])}
                            </div>
                            <div class="text-[10px] text-muted-foreground">{move || t("待复习")}</div>
                          }
                          .into_any()
                        } else {
                          view! {
                            <div class="text-sm text-muted-foreground">{"—"}</div>
                          }
                          .into_any()
                        }}
                      </div>
                    </li>
                  }
                })
                .collect_view()
            }}
          </ul>
          <p class="border-t px-4 py-3 text-xs text-muted-foreground">
            {move || t("「建议模拟考」出现在考试前 7 天内；「待复习」为该天到期的错题数。")}
          </p>
        </section>
      </div>
    </div>
  }
}

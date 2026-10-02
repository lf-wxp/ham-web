//! 备考计划：设定考试日期与类别，按剩余题量、待复习错题与临考阶段给出今日任务。

use ham_web_core::Bank;
use ham_web_core::study_plan::{DailyGoal, Phase, StudyPlan, daily_goal};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::i18n::{t, tf};
use crate::ui::{
  CARD_HEADER, Size, Variant, button_class, card_class, card_content_class, card_title_class,
  input_class,
};
use crate::util::{local_day, local_today, now_ms};
use crate::{data, exam_history, study};

/// 一条今日任务。
fn task_row(
  done: bool,
  label: String,
  detail: String,
  href: String,
  action: String,
) -> impl IntoView {
  view! {
    <li class="flex items-center gap-3 py-2">
      <span
        aria-hidden="true"
        class=if done {
          "flex size-5 shrink-0 items-center justify-center rounded-full bg-emerald-500 text-xs text-white"
        } else {
          "size-5 shrink-0 rounded-full border-2 border-muted-foreground/40"
        }
      >
        {done.then_some("✓")}
      </span>
      <div class="min-w-0 flex-1">
        <div class=if done { "text-sm text-muted-foreground line-through" } else { "text-sm font-medium" }>
          {label}
          {done.then(|| view! { <span class="sr-only">{move || t("（已完成）")}</span> })}
        </div>
        <div class="text-xs text-muted-foreground">{detail}</div>
      </div>
      {(!done).then(|| view! {
        <a href=href class=button_class(Variant::Outline, Size::Sm, "shrink-0")>{action}</a>
      })}
    </li>
  }
}

/// 最近 14 天每日作答量柱状图。
fn history_bars(today: &str) -> impl IntoView {
  let days = study::load_daily().recent(today, 14);
  let max = days
    .iter()
    .map(|(_, t)| t.answered)
    .max()
    .unwrap_or(0)
    .max(1);
  let total: u32 = days.iter().map(|(_, t)| t.answered).sum();
  view! {
    <div>
      <div class="mb-1.5 flex items-center justify-between text-xs text-muted-foreground">
        <span>{move || t("最近 14 天作答")}</span>
        <span class="tabular-nums">{tf("共 {} 题", &[&total.to_string()])}</span>
      </div>
      <div class="flex h-16 items-end gap-1" role="img" aria-label=tf("最近 14 天共作答 {} 题", &[&total.to_string()])>
        {days
          .into_iter()
          .map(|(day, t)| {
            let h = f64::from(t.answered) / f64::from(max) * 100.0;
            view! {
              <div
                class="flex-1 rounded-t bg-primary/70"
                style=format!("height: max({h:.1}%, 2px)")
                title=tf(
                  "{}：{} 题（新题 {}）",
                  &[&day[5..], &t.answered.to_string(), &t.new.to_string()],
                )
              ></div>
            }
          })
          .collect_view()}
      </div>
    </div>
  }
}

#[component]
pub fn StudyPlanCard(
  /// 显示考试日期 / 类别设置与近 14 天统计（学习进度页）；否则只在已设定计划时显示今日任务（首页）。
  #[prop(optional)]
  editable: bool,
) -> impl IntoView {
  let stats = StoredValue::new(study::load_stats());
  let plan = RwSignal::new(study::load_plan());
  let today = local_today();
  let bank = Memo::new(move |_| {
    plan
      .with(|p| p.bank)
      .or_else(|| stats.with_value(|s| s.main_bank()))
      .unwrap_or_default()
  });
  let unseen = RwSignal::new(None::<usize>);
  Effect::new(move |_| {
    let b = bank.get();
    spawn_local(async move {
      if let Ok(qs) = data::load_bank(None, b, false).await {
        let covered = stats.with_value(|s| s.bank(b).map_or(0, |bs| bs.covered(qs.iter())));
        unseen.try_set(Some(qs.len().saturating_sub(covered)));
      }
    });
  });

  let update = move |f: &dyn Fn(&mut StudyPlan)| {
    plan.update(|p| f(p));
    study::save_plan(&plan.get_untracked());
  };

  let today_for_goal = today.clone();
  let goal = Memo::new(move |_| {
    let days = plan.with(|p| p.days_left(&today_for_goal))?;
    let b = bank.get();
    let now = now_ms();
    let due = study::load_book()
      .records
      .iter()
      .filter(|r| r.is_due(now) && r.in_bank(b))
      .count();
    let new_today = study::load_daily().on(&today_for_goal).new as usize;
    Some(daily_goal(days, unseen.get()?, new_today, due))
  });

  let today_for_tasks = today.clone();
  let tasks = move || {
    let g: DailyGoal = goal.get()?;
    let b = bank.get();
    let today = &today_for_tasks;
    let log = study::load_daily().on(today);
    let headline = match g.phase {
      Phase::Over => t("考试日期已过，可以设定下一次考试。"),
      Phase::ExamDay => t("今天考试，祝顺利通过！考前只看看错题，别再刷新题了。"),
      Phase::Sprint => tf(
        "距 {} 类考试还有 {} 天 · 冲刺阶段：错题 + 模拟考试",
        &[&b.to_string(), &g.days_left.to_string()],
      ),
      Phase::Learn => tf(
        "距 {} 类考试还有 {} 天",
        &[&b.to_string(), &g.days_left.to_string()],
      ),
    };
    let exam_today = exam_history::load()
      .iter()
      .any(|r| r.bank() == Some(b) && local_day(r.timestamp as f64) == *today);
    let rows = view! {
      <ul class="divide-y">
        {(g.new_target > 0).then(|| {
          let done_new = log.new as usize;
          task_row(
            done_new >= g.new_target,
            tf(
              "做新题 {} / {}",
              &[
                &done_new.min(g.new_target).to_string(),
                &g.new_target.to_string(),
              ],
            ),
            tf(
              "{} 类还有 {} 题没做过",
              &[&b.to_string(), &unseen.get().unwrap_or(0).to_string()],
            ),
            format!("/practice?bank={b}&unseen=1"),
            t("去做"),
          )
        })}
        {(g.phase != Phase::Over).then(|| task_row(
          g.review == 0,
          if g.review == 0 { t("错题已复习完") } else { tf("复习错题 {} 题", &[&g.review.to_string()]) },
          t("按间隔复习到期的错题"),
          format!("/mistakes?review=1&bank={b}"),
          t("去复习"),
        ))}
        {g.mock_exam.then(|| task_row(
          exam_today,
          tf("{} 类模拟考试 1 套", &[&b.to_string()]),
          t("临考每天一套，熟悉节奏、检验成绩"),
          format!("/exam?bank={b}"),
          t("去考"),
        ))}
        {(g.phase != Phase::Over).then(|| task_row(
          false,
          t("必背考点速查"),
          t("法规条款、公式与高频考点，每天抽空过一遍"),
          "/cheat-sheet".to_owned(),
          t("去背"),
        ))}
      </ul>
    };
    Some(view! {
      <div class="space-y-1">
        <div class="text-sm font-medium">{headline}</div>
        <div class="text-xs text-muted-foreground">{tf("今天已作答 {} 题，其中新题 {} 题", &[&log.answered.to_string(), &log.new.to_string()])}</div>
        {rows}
      </div>
    })
  };

  let has_plan = move || plan.with(|p| !p.target_date.is_empty());
  if !editable && !has_plan() {
    return ().into_any();
  }

  let seg_class = move |on: bool| {
    if on {
      "rounded-md bg-primary px-2.5 py-1 text-xs font-medium text-primary-foreground"
    } else {
      "rounded-md px-2.5 py-1 text-xs text-muted-foreground hover:text-foreground"
    }
  };

  view! {
    <section id="plan" data-slot="card" class=card_class("scroll-mt-24")>
      <div data-slot="card-header" class=CARD_HEADER>
        <h2 data-slot="card-title" class=card_title_class("flex items-center justify-between gap-2")>
          <span>{move || t("备考计划")}</span>
          {(!editable).then(|| view! {
            <a href="/progress#plan" class="text-xs font-normal text-muted-foreground hover:text-foreground">{move || t("调整 →")}</a>
          })}
        </h2>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-4")>
        {editable.then(|| view! {
          <div class="flex flex-wrap items-end gap-4">
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">{move || t("考试日期")}</span>
              <input
                type="date"
                prop:value=move || plan.with(|p| p.target_date.clone())
                on:change=move |e| {
                  let v = event_target_value(&e);
                  update(&|p| p.target_date = v.clone());
                }
                class=input_class("w-44")
              />
            </label>
            <div class="flex flex-col gap-1.5">
              <span class="text-xs text-muted-foreground">{move || t("报考类别")}</span>
              <div class="inline-flex gap-0.5 rounded-lg border p-0.5" role="group" aria-label=move || t("报考类别")>
                {Bank::ALL
                  .into_iter()
                  .map(|b| view! {
                    <button
                      type="button"
                      class=move || seg_class(bank.get() == b)
                      aria-pressed=move || (bank.get() == b).to_string()
                      on:click=move |_| update(&|p| p.bank = Some(b))
                    >
                      {tf("{} 类", &[&b.to_string()])}
                    </button>
                  })
                  .collect_view()}
              </div>
            </div>
            <label class="flex flex-col gap-1.5 text-sm">
              <span class="text-xs text-muted-foreground">{move || t("每日提醒时间")}</span>
              <input
                type="time"
                prop:value=move || plan.with(|p| p.reminder_time.clone())
                on:change=move |e| {
                  let v = event_target_value(&e);
                  update(&|p| p.reminder_time = v.clone());
                }
                class=input_class("w-32")
              />
            </label>
          </div>
        })}
        {move || {
          if !has_plan() {
            return view! {
              <p class="text-sm text-muted-foreground">
                {move || t("设定考试日期后，会按还没做过的题量、待复习错题和临考阶段，每天安排新题、复习与模拟考试任务，并显示在首页。")}
              </p>
            }
            .into_any();
          }
          match tasks() {
            Some(v) => v.into_any(),
            None => view! { <p class="text-sm text-muted-foreground">{move || t("加载题库中...")}</p> }.into_any(),
          }
        }}
        {editable.then(|| history_bars(&today))}
      </div>
    </section>
  }
  .into_any()
}

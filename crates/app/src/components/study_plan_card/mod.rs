//! 备考计划：设定考试日期与类别，按剩余题量、待复习错题与临考阶段给出今日任务。

mod task_row;
use task_row::task_row;
mod history_bars;
use history_bars::history_bars;

use ham_web_core::Bank;
use ham_web_core::study_plan::{DailyGoal, Phase, StudyPlan, daily_goal};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::common::SectionCard;
use crate::i18n::{t, tf, tp};
use crate::ui::{
  CARD_HEADER, Chip, ChipGroup, DatePicker, Field, TimePicker, card_class, card_content_class,
  card_title_class,
};
use crate::util::{local_day, local_today, now_ms};
use crate::{data, exam_history, study};

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
      Phase::Over => t("common.the-exam-date-has"),
      Phase::ExamDay => t("common.exam-today-good-luck"),
      Phase::Sprint => tp(
        "common.days-until-the-class-sprint",
        g.days_left,
        &[&b.to_string(), &g.days_left.to_string()],
      ),
      Phase::Learn => tp(
        "common.days-until-the-class",
        g.days_left,
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
            tp(
              "common.do-new-questions",
              g.new_target as u32,
              &[
                &done_new.min(g.new_target).to_string(),
                &g.new_target.to_string(),
              ],
            ),
            tp(
              "common.questions-left-unseen-in",
              unseen.get().unwrap_or(0),
              &[&b.to_string(), &unseen.get().unwrap_or(0).to_string()],
            ),
            format!("/practice?bank={b}&unseen=1"),
            t("common.go"),
          )
        })}
        {(g.phase != Phase::Over).then(|| task_row(
          g.review == 0,
          if g.review == 0 { t("common.all-mistakes-reviewed") } else { tp("common.review-mistakes", g.review, &[&g.review.to_string()]) },
          t("common.review-mistakes-due-by"),
          format!("/mistakes?review=1&bank={b}"),
          t("common.review"),
        ))}
        {g.mock_exam.then(|| task_row(
          exam_today,
          tf("common.one-class-mock-exam", &[&b.to_string()]),
          t("common.one-mock-exam-a"),
          format!("/exam?bank={b}"),
          t("common.take-one"),
        ))}
        {(g.phase != Phase::Over).then(|| task_row(
          false,
          t("common.must-know-exam-points"),
          t("common.rules-formulas-and-high"),
          "/cheat-sheet".to_owned(),
          t("common.remove-background"),
        ))}
      </ul>
    };
    Some(view! {
      <div class="space-y-1">
        <div class="text-sm">{headline}</div>
        <div class="text-xs text-muted-foreground">{tp("common.answered-questions-today-of", log.answered, &[&log.answered.to_string(), &log.new.to_string()])}</div>
        {rows}
      </div>
    })
  };

  let has_plan = move || plan.with(|p| !p.target_date.is_empty());
  // 组件体里的一次性判断：只决定「只读且没有计划时不渲染」，不订阅。
  if !editable && plan.with_untracked(|p| p.target_date.is_empty()) {
    return ().into_any();
  }

  // 正文只写一份，按场景套不同的外壳：
  // - 首页（只读摘要）：和其它首页卡片一样用 `SectionCard`（L1 平卡 + 像素标题栏）；
  // - 学习进度页（可编辑）：它就是这一页的主角，保持窗口级（`card_class`，L2）。
  let body = view! {
    {editable.then(|| view! {
      // 标签统一走 `Field`（`<label for>` ↔ 控件 `id` 配对）：以前这里用裸 `<label>` 包住整个
      // 选择器，点标签文字会把点击转发给面板里的第一个可聚焦元素 —— 意外开面板。
      <div class="flex flex-wrap items-end gap-4">
        <Field
          label=Signal::derive(move || t("common.exam-date"))
          r#for="plan-date"
          class="w-44"
        >
          <DatePicker
            id="plan-date"
            value=Signal::derive(move || plan.with(|p| p.target_date.clone()))
            on_change=Callback::new(move |v: String| update(&|p| p.target_date = v.clone()))
            class="w-full"
          />
        </Field>
        <div class="flex flex-col gap-1.5">
          <span class="text-xs text-muted-foreground">{move || t("common.target-class")}</span>
          <ChipGroup
            value=Signal::derive(move || bank.get().as_str().to_string())
            on_change=Callback::new(move |v: String| {
              if let Ok(b) = v.parse::<Bank>() {
                update(&|p| p.bank = Some(b));
              }
            })
            aria_label=Signal::derive(move || t("common.target-class"))
          >
            {Bank::ALL
              .into_iter()
              .map(|b| {
                let label = view! { {move || tf("learning.class", &[&b.to_string()])} }.into_any();
                view! { <Chip value=b.as_str().to_string()>{label}</Chip> }
              })
              .collect_view()}
          </ChipGroup>
        </div>
        <Field
          label=Signal::derive(move || t("settings.daily-reminder-time"))
          r#for="plan-reminder"
          class="w-32"
        >
          <TimePicker
            id="plan-reminder"
            value=Signal::derive(move || plan.with(|p| p.reminder_time.clone()))
            on_change=Callback::new(move |v: String| update(&|p| p.reminder_time = v.clone()))
            class="w-full"
          />
        </Field>
      </div>
    })}
    {move || {
      if !has_plan() {
        return view! {
          <p class="text-sm text-muted-foreground">
            {move || t("common.once-you-set-an")}
          </p>
        }
        .into_any();
      }
      match tasks() {
        Some(v) => v.into_any(),
        None => view! { <p class="text-sm text-muted-foreground">{move || t("exam.loading-questions-u-2026")}</p> }.into_any(),
      }
    }}
    {editable.then(|| history_bars(&today))}
  };

  if editable {
    view! {
      <section id="plan" data-slot="card" class=card_class("scroll-mt-24")>
        <div data-slot="card-header" class=CARD_HEADER>
          <h2 data-slot="card-title" class=card_title_class("flex items-center justify-between gap-2")>
            <span>{move || t("common.study-plan")}</span>
          </h2>
        </div>
        <div data-slot="card-content" class=card_content_class("space-y-4")>{body}</div>
      </section>
    }
    .into_any()
  } else {
    view! {
      <SectionCard
        id="plan"
        class="scroll-mt-24"
        title=Signal::derive(move || t("common.study-plan"))
        meta=ViewFn::from(move || {
          view! {
            <a href="/progress#plan" class="underline underline-offset-4 hover:text-foreground">
              {move || t("common.adjust")}
            </a>
          }
        })
      >
        <div class="space-y-4 p-4">{body}</div>
      </SectionCard>
    }
    .into_any()
  }
}

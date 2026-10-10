//! 学习周报：近两周作答趋势、新题 / 复习占比、连续打卡与薄弱分类回顾。

use ham_web_core::ExamRule;
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf, tp};
use crate::share_score;
use crate::study;
use crate::util::{local_today, set_title};

#[component]
pub fn WeeklyPage() -> impl IntoView {
  set_title("shell.weekly-report");

  let today = local_today();
  let daily = study::load_daily();
  let stats = study::load_stats();
  let book = study::load_book();
  let history = crate::exam_history::load();

  let recent14 = daily.recent(&today, 14);
  // 本周（最近 7 天，含今天）
  let week: Vec<_> = recent14.iter().rev().take(7).collect();
  let week_answered: u32 = week.iter().map(|(_, t)| t.answered).sum();
  let week_new: u32 = week.iter().map(|(_, t)| t.new).sum();
  let week_review = week_answered.saturating_sub(week_new);
  // 本周累计学习时长（毫秒），格式化为「X 分钟 / X.X 小时」。
  let week_duration_ms: u64 = week.iter().map(|(_, t)| t.duration_ms).sum();
  let week_duration_text = if week_duration_ms >= 3_600_000 {
    tf(
      "learning.hours",
      &[&format!("{:.1}", week_duration_ms as f64 / 3_600_000.0)],
    )
  } else {
    tf(
      "radio.min",
      &[&format!("{:.0}", week_duration_ms as f64 / 60_000.0)],
    )
  };

  // 连续打卡天数：从今天往回数连续有作答的天数。
  let mut streak = 0usize;
  for (_, t) in recent14.iter().rev() {
    if t.answered > 0 {
      streak += 1;
    } else {
      break;
    }
  }

  // 趋势图数据（owned，供 view 闭包消费）。
  let max_day = recent14
    .iter()
    .map(|(_, t)| t.answered)
    .max()
    .unwrap_or(1)
    .max(1);
  let trend: Vec<(String, u32, u32)> = recent14
    .iter()
    .map(|(d, t)| (d.clone(), t.answered, t.new))
    .collect();

  // 薄弱分类（owned 数据）。
  let weakest: Option<(String, String, f64, u32)> = stats.total.weakest(5).map(|(key, t)| {
    let name = ham_web_core::categories::top_category(key)
      .map_or_else(|| key.to_owned(), |c| c.name.to_owned());
    (
      key.to_owned(),
      name,
      t.rate().map_or(0.0, |r| r * 100.0),
      t.answered,
    )
  });
  let main_bank = stats.main_bank();

  // 分享卡片所需的数据（owned，供分享按钮使用）。
  let mistakes = book.records.len();
  let weakest_label: Option<String> = weakest.as_ref().map(|(_, name, rate, answered)| {
    tf(
      "common.correct-answered",
      &[
        &name.to_string(),
        &format!("{rate:.0}"),
        &answered.to_string(),
      ],
    )
  });
  let weakest_label_sv = StoredValue::new(weakest_label.clone());
  let card = move || {
    share_score::render_weekly_card(
      week_answered,
      week_new,
      streak,
      mistakes,
      weakest_label_sv.get_value().as_deref(),
    )
    .ok()
  };
  // 页头 actions 由 `ViewFn`（要求 `Fn`）承载，闭包需可重复调用，故用 `Callback`（`Copy`）。
  let share = Callback::new({
    let today = today.clone();
    move |_: ()| {
      if let Some(data_url) = card() {
        share_score::download(&data_url, &format!("ham-weekly-{today}.png"));
      }
    }
  });
  let copy = Callback::new(move |_: ()| {
    if let Some(data_url) = card() {
      leptos::task::spawn_local(async move {
        match share_score::copy_image(&data_url).await {
          Ok(()) => crate::util::alert(&t("learning.copied-to-clipboard")),
          Err(e) => crate::util::alert(&tf("learning.copy-failed", &[&e])),
        }
      });
    }
  });
  let sys_share = Callback::new({
    let today = today.clone();
    move |_: ()| {
      if let Some(data_url) = card() {
        let filename = format!("ham-weekly-{today}.png");
        let title = t("shell.weekly-report");
        leptos::task::spawn_local(async move {
          if let Err(e) = share_score::share_image(&data_url, &filename, &title).await {
            crate::util::alert(&tf("learning.share-failed", &[&e]));
          }
        });
      }
    }
  });

  // 最近 5 次常规（非薄弱项）模拟考成绩（owned）。
  let mut exams: Vec<_> = history
    .into_iter()
    .rev()
    .filter(|r| !r.weak)
    .take(5)
    .collect();
  exams.reverse();

  let new_pct = if week_answered > 0 {
    week_new as f64 / week_answered as f64 * 100.0
  } else {
    0.0
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.weekly-report")
        subtitle=move || t("learning.two-week-answering-trend")
        actions=ViewFn::from(move || {
          view! {
            <button
              type="button"
              class="rounded-md border px-3 py-1.5 text-xs transition-colors hover:bg-accent"
              on:click=move |_| share.run(())
            >
              {move || t("learning.download-card")}
            </button>
            <button
              type="button"
              class="rounded-md border px-3 py-1.5 text-xs transition-colors hover:bg-accent"
              on:click=move |_| copy.run(())
            >
              {move || t("learning.copy-image")}
            </button>
            <button
              type="button"
              class="rounded-md border px-3 py-1.5 text-xs transition-colors hover:bg-accent"
              on:click=move |_| sys_share.run(())
            >
              {move || t("learning.system-share")}
            </button>
            <a href="/progress" class="text-xs text-primary underline-offset-4 hover:underline">
              {move || t("learning.progress")}
            </a>
          }
        })
      />

      <PageContainer class="space-y-4">
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-5">
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">{week_answered}</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("exam.answered-this-week")}</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">{week_new}</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("exam.new-questions-this-week")}</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">{streak}</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("learning.check-in-streak-days")}</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">{mistakes}</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("exam.current-mistakes")}</div>
          </div>
          <div class="rounded-xl border bg-card p-4 text-center">
            <div class="text-2xl font-semibold tabular-nums">{week_duration_text}</div>
            <div class="mt-1 text-xs text-muted-foreground">{move || t("learning.study-time-this-week")}</div>
          </div>
        </div>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("learning.answering-trend-over-the")}</h2>
          <div class="p-4">
            <div class="flex h-28 items-end gap-1">
              {move || {
                trend
                  .clone()
                  .into_iter()
                  .map(|(day, answered, new)| {
                    let h = (answered as f64 / max_day as f64 * 100.0)
                      .max(if answered > 0 { 4.0 } else { 0.0 });
                    let label = day.get(8..).unwrap_or(&day).to_owned();
                    view! {
                      <div class="group flex min-w-0 flex-1 flex-col items-center gap-1">
                        <div
                          class="w-full rounded-t bg-primary/70 transition-all group-hover:bg-primary"
                          style=format!("height: {h}%")
                          title=tp("common.questions-new", answered, &[&(day).to_string(), &(answered).to_string(), &(new).to_string()])
                        ></div>
                        <span class="text-[9px] text-muted-foreground">{label}</span>
                      </div>
                    }
                  })
                  .collect_view()
              }}
            </div>
            <div class="mt-3 flex items-center gap-4 text-xs text-muted-foreground">
              <span>
                {t("learning.new")} <span class="font-semibold text-foreground tabular-nums">{week_new}</span>
              </span>
              <span>
                {t("learning.reviews")} <span class="font-semibold text-foreground tabular-nums">{week_review}</span>
              </span>
              <span class="ml-auto tabular-nums">{move || t("learning.share-of-new-questions")} {format!("{new_pct:.0}%")}</span>
            </div>
          </div>
        </section>

        {weakest.map(|(key, name, rate, answered)| {
          let href = match main_bank {
            Some(b) => format!("/practice?bank={b}&topic={key}"),
            None => format!("/practice?topic={key}"),
          };
          view! {
            <section class="rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("exam.weakest-category-this-week")}</h2>
              <div class="flex items-center gap-3 p-4">
                <div class="min-w-0 flex-1">
                  <div class="text-sm font-medium">{name}</div>
                  <div class="mt-0.5 text-xs text-muted-foreground tabular-nums">
                    {tp("common.questions-answered-accuracy", answered, &[&(answered).to_string(), &(rate).to_string()])}
                  </div>
                </div>
                <a href=href class="rounded-md border px-3 py-1.5 text-xs transition-colors hover:bg-accent">
                  {move || t("learning.focused-practice")}
                </a>
              </div>
            </section>
          }
        })}

        {move || {
          if exams.is_empty() {
            return view! { <div></div> }.into_any();
          }
          view! {
            <section class="rounded-xl border bg-card">
              <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("learning.recent-mock-exams")}</h2>
              <div class="divide-y">
                {exams
                  .iter()
                  .map(|r| {
                    let rule = ExamRule::of(ham_web_core::Bank::from_param(Some(&r.bank)));
                    let passed = r.correct >= rule.pass;
                    view! {
                      <div class="flex items-center gap-3 px-4 py-2.5 text-sm">
                        <span class="w-8 shrink-0 font-mono text-xs text-muted-foreground">{r.bank.clone()}</span>
                        <span class="tabular-nums text-muted-foreground">{r.correct} " / " {r.total}</span>
                        <span class=if passed { "ml-auto text-xs font-medium text-emerald-700 dark:text-emerald-400" } else { "ml-auto text-xs font-medium text-red-700 dark:text-red-400" }>
                          {if passed { t("exam.passed") } else { t("learning.not-passed") }}
                        </span>
                      </div>
                    }
                  })
                  .collect_view()}
              </div>
            </section>
          }
          .into_any()
        }}

        <p class="text-xs text-muted-foreground">
          {move || t("learning.data-comes-from-local")}
        </p>
      </PageContainer>
    </div>
  }
}

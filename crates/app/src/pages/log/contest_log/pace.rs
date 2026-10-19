//! 赛前目标与赛中配速：按线性进度显示「此刻该在哪儿」。
//!
//! 目标随场次持久化（`Session.target_qsos` / `planned_minutes`），时间从场次开始
//! 时刻算起。只做算术，不预测传播，也不参与记分。

use std::time::Duration;

use ham_web_core::contest_pace::{self, Goal, hours_text, parse_hours, parse_target_qsos};
use leptos::prelude::*;

use super::session::Session;
use crate::i18n::{t, tf, tp};
use crate::ui::{Field, NumberField, NumberKind, Stat};
use crate::util::unique_id;

/// 刷新周期：配速只关心分钟级变化，逐秒更新只是白白重绘。
const TICK_SECS: u64 = 60;

#[component]
pub(super) fn ContestPace(
  session: RwSignal<Session>,
  done: Signal<u32>,
  save: Callback<()>,
) -> impl IntoView {
  let target_id = unique_id("contest-target");
  let hours_id = unique_id("contest-plan");

  let now = RwSignal::new(js_sys::Date::now());
  let handle = set_interval_with_handle(
    move || now.set(js_sys::Date::now()),
    Duration::from_secs(TICK_SECS),
  )
  .ok();
  on_cleanup(move || {
    if let Some(h) = handle {
      h.clear();
    }
  });

  let elapsed_min = Signal::derive(move || {
    let start = session.with(|s| s.start.clone());
    // 场次时间是 UTC 字符串（`YYYY-MM-DD HH:MM`），按 UTC 解析，避免本地时区把已用时间
    // 算歪。必须换成 ISO 的 `T` 分隔：带空格的写法不在 ECMAScript 规定的格式里，
    // 部分引擎会返回 NaN，而下面 `is_nan → 0` 是静默的，整块配速会全是 0。
    let iso = start.replacen(' ', "T", 1);
    let started = js_sys::Date::parse(&format!("{iso}:00Z"));
    if started.is_nan() {
      0.0
    } else {
      (now.get() - started).max(0.0) / 60_000.0
    }
  });

  let goal = Signal::derive(move || Goal {
    target_qsos: session.with(|s| s.target_qsos),
    planned_minutes: f64::from(session.with(|s| s.planned_minutes)),
  });

  let report = Signal::derive(move || {
    let g = goal.get();
    g.is_set()
      .then(|| (contest_pace::pace(done.get(), elapsed_min.get(), &g), g))
  });

  let target_value = Signal::derive(move || {
    let n = session.with(|s| s.target_qsos);
    if n == 0 { String::new() } else { n.to_string() }
  });
  // 输入框的显示源是这份草稿，而不是「session 取整后再反推」：后者每敲一个键都把
  // 用户原文换成格式化后的值，小数点 / 负号这类中间态会被当场吞掉。
  let hours_draft = RwSignal::new(hours_text(session.with_untracked(|s| s.planned_minutes)));
  // 场次被重置（清空 / 换场次）时把草稿跟回去。只在**分钟数变了**的时候才动草稿：
  // session 里别的字段（目标 QSO 数等）变化也会触发这个 effect，而那时草稿里可能正躺着
  // 一个还没敲完的非法中间态，不该被无关的更新覆盖。
  Effect::new(move |prev: Option<u32>| {
    let minutes = session.with(|s| s.planned_minutes);
    if prev != Some(minutes) && parse_hours(&hours_draft.get_untracked()) != Some(minutes) {
      hours_draft.set(hours_text(minutes));
    }
    minutes
  });

  #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
  let whole = |value: f64| value.max(0.0).round() as usize;

  view! {
    <section class="pxl-card border bg-card">
      <h2 class="border-b px-4 py-3 text-sm font-semibold">
        {move || t("contest.pace-title")}
      </h2>
      <div class="space-y-4 p-4">
        <div class="grid gap-3 sm:grid-cols-2">
          <Field label=Signal::derive(|| t("contest.pace-target")) r#for=target_id.clone()>
            <NumberField
              id=target_id
              value=target_value
              on_change=Callback::new(move |v: String| {
                // 空串＝清除目标；`1.5`、负号这类非法输入不写回——以前一律 `unwrap_or(0)`，
                // 手滑敲错一个字符目标就被静默清零并持久化。
                let Some(n) = parse_target_qsos(&v) else {
                  return;
                };
                session.update(|s| s.target_qsos = n);
                save.run(());
              })
              step=10.0
              min=0.0
              controls=false
            />
          </Field>
          <Field label=Signal::derive(|| t("contest.pace-planned-hours")) r#for=hours_id.clone()>
            <NumberField
              id=hours_id
              value=Signal::derive(move || hours_draft.get())
              on_change=Callback::new(move |v: String| {
                hours_draft.set(v.clone());
                let Some(minutes) = parse_hours(&v) else {
                  return;
                };
                session.update(|s| s.planned_minutes = minutes);
                save.run(());
              })
              kind=NumberKind::Decimal
              step=0.5
              min=0.0
              controls=false
            />
          </Field>
        </div>
        <p class="text-xs text-muted-foreground">{move || t("contest.pace-hint")}</p>

        {move || {
          match report.get() {
            None => {
              view! {
                <p class="text-sm text-muted-foreground">{move || t("contest.pace-unset")}</p>
              }
                .into_any()
            }
            Some((pace, goal)) => {
              let reached = f64::from(pace.done) >= f64::from(goal.target_qsos);
              let delta = if pace.delta >= 0.0 {
                tp(
                  "contest.pace-ahead",
                  pace.delta.round(),
                  &[&format!("{:.0}", pace.delta)],
                )
              } else {
                tp(
                  "contest.pace-behind",
                  (-pace.delta).round(),
                  &[&format!("{:.0}", -pace.delta)],
                )
              };
              // 已达标时 core 仍会给 `eta_min = Some(0.0)`：不能同时显示「已达到目标」
              // 与「按当前速率还需 0 分钟」。
              let required = (!reached)
                .then(|| {
                  pace.required_rate
                    .map(|r| tf("contest.pace-required", &[&format!("{r:.1}")]))
                })
                .flatten();
              let eta = (!reached)
                .then(|| pace.eta_min.map(|m| tf("contest.pace-eta", &[&format!("{m:.0}")])))
                .flatten();
              let missing = f64::from(goal.target_qsos) - f64::from(pace.done);
              view! {
                <div class="space-y-3">
                  <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
                    <Stat
                      label=t("contest.pace-done")
                      value=Signal::derive(move || pace.done as usize)
                    />
                    <Stat
                      label=t("contest.pace-expected")
                      value=Signal::derive(move || whole(pace.expected))
                    />
                    <Stat
                      label=t("contest.pace-rate")
                      value=Signal::derive(move || whole(pace.current_rate))
                    />
                    <Stat
                      label=t("contest.pace-projected")
                      value=Signal::derive(move || whole(pace.projected))
                    />
                  </div>
                  <p class="text-sm">
                    {if reached {
                      t("contest.pace-reached").to_string()
                    } else if pace.over_time {
                      tp("contest.pace-overtime", missing.round(), &[&format!("{missing:.0}")])
                    } else {
                      delta
                    }}
                  </p>
                  {required
                    .map(|text| view! { <p class="text-sm text-muted-foreground">{text}</p> })}
                  {eta.map(|text| view! { <p class="text-sm text-muted-foreground">{text}</p> })}
                </div>
              }
                .into_any()
            }
          }
        }}
      </div>
    </section>
  }
}

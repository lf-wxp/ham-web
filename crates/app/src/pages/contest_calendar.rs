//! 竞赛日历：内置全球主要竞赛，可按届加入倒计时提醒。

use ham_web_core::contest_calendar::{CONTEST_CALENDAR, ContestEvent, next_start};
use leptos::prelude::*;

use crate::components::common::{PageContainer, PageHeader};
use crate::i18n::{t, tf};
use crate::ui::{Button, ButtonLink, Size, Variant};
use crate::util::{alert, set_title};

#[component]
pub fn ContestCalendarPage() -> impl IntoView {
  set_title("shell.contest-calendar");

  let now = js_sys::Date::new_0();
  let now_t = (
    now.get_utc_full_year() as i32,
    now.get_utc_month() + 1,
    now.get_utc_date(),
  );

  // 按下一次开始时间排序。
  let mut scheduled: Vec<(&ContestEvent, (i32, u32, u32))> = CONTEST_CALENDAR
    .iter()
    .map(|e| (e, next_start(e, now_t)))
    .collect();
  scheduled.sort_by_key(|(_, (y, m, d))| (*y, *m, *d));

  let add_reminder = move |e: &'static ContestEvent| {
    let (y, m, d) = next_start(e, now_t);
    let month_start = js_sys::Date::utc(y as f64, (m - 1) as f64);
    let target = month_start + (d as f64 - 1.0) * 86_400_000.0;
    super::countdown::add_countdown(&tf("contest.contest-2", &[(e.name)]), target as i64);
    alert(&tf("contest.added-to-countdown-reminders", &[(e.name)]));
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <PageHeader
        title=move || t("shell.contest-calendar")
        subtitle=move || t("contest.major-global-contests-add")
        actions=ViewFn::from(move || {
          view! {
            <ButtonLink
              href="/countdown"
              variant=Variant::Outline
              size=Size::Sm
            >
              {move || t("contest.my-countdowns")}
            </ButtonLink>
          }
        })
      />

      <PageContainer>
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("contest.all-contests-sorted-by")}</h2>
          <div class="divide-y">
            {scheduled
              .iter()
              .map(|(e, (y, m, d))| {
                let ev = *e;
                let dur = if ev.duration_days <= 1 { t("contest.24-hours") } else { t("contest.48-hours") };
                view! {
                  <div class="grid gap-2 px-4 py-3 sm:grid-cols-[1fr_auto]">
                    <div class="min-w-0">
                      <div class="flex flex-wrap items-baseline gap-x-2">
                        <span class="font-medium">{move || t(ev.name)}</span>
                        <span class="rounded-full border bg-muted/50 px-2 py-0.5 text-[10px] text-muted-foreground">
                          {move || t(ev.mode)}
                        </span>
                      </div>
                      <div class="mt-0.5 text-xs tabular-nums text-muted-foreground">
                        {tf(
                          "contest.exchange",
                          &[
                            &y.to_string(),
                            &m.to_string(),
                            &d.to_string(),
                            &dur,
                            &t(ev.exchange),
                          ],
                        )}
                      </div>
                      <div class="mt-1 text-sm text-muted-foreground">{move || t(ev.desc)}</div>
                    </div>
                    <div class="flex items-center gap-2 sm:justify-end">
                      {ev.contest_id.map(|cid| view! {
                        <ButtonLink
                          href=format!("/contest-log?contest={cid}")
                          variant=Variant::Outline
                          size=Size::Sm
                        >
                          {move || t("contest.start-session")}
                        </ButtonLink>
                      })}
                      <Button
                        variant=Variant::Secondary
                        size=Size::Sm
                        on_click=Callback::new(move |_| add_reminder(ev))
                      >
                        {move || t("contest.add-reminder")}
                      </Button>
                    </div>
                  </div>
                }
              })
              .collect_view()}
          </div>
          <p class="px-4 py-3 text-xs text-muted-foreground">
            {move || t("contest.dates-are-utc-start")}
          </p>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("contest.contest-tips")}</h2>
          <ul class="space-y-2 p-4">
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{move || t("contest.contests-often-use-cq")}</span>
            </li>
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{move || t("contest.after-the-contest-log")}</span>
            </li>
          </ul>
          <div class="px-4 pb-4">
            <a href="/contest-log" class="text-xs text-primary underline-offset-4 hover:underline">
              {move || t("contest.go-to-contest-logging")}
            </a>
          </div>
        </section>
      </PageContainer>
    </div>
  }
}

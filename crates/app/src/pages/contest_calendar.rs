//! 竞赛日历：内置全球主要竞赛，可按届加入倒计时提醒。

use ham_web_core::contest_calendar::{CONTEST_CALENDAR, ContestEvent, next_start};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Size, Variant, button_class};
use crate::util::{alert, set_title};

#[component]
pub fn ContestCalendarPage() -> impl IntoView {
  set_title(&t("竞赛日历"));

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
    super::countdown::add_countdown(&tf("竞赛：{}", &[(e.name)]), target as i64);
    alert(&tf("已把「{}」加入倒计时提醒", &[(e.name)]));
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("竞赛日历")}</h1>
            <div class="text-xs text-muted-foreground">{move || t("全球主要竞赛 · 加入倒计时提醒")}</div>
          </div>
          <a href="/countdown" class=button_class(Variant::Outline, Size::Sm, "")>
            {move || t("我的倒计时")}
          </a>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("全年竞赛（按下一届开始时间排序）")}</h2>
          <div class="divide-y">
            {scheduled
              .iter()
              .map(|(e, (y, m, d))| {
                let ev = *e;
                let dur = if ev.duration_days <= 1 { t("24 小时") } else { t("48 小时") };
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
                          "{} 年 {} 月 {} 日 · {} · 交换：{}",
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
                        <a href=format!("/contest-log?contest={cid}") class=button_class(Variant::Outline, Size::Sm, "")>
                          {move || t("开新场次")}
                        </a>
                      })}
                      <button
                        type="button"
                        class=button_class(Variant::Secondary, Size::Sm, "")
                        on:click=move |_| add_reminder(ev)
                      >
                        {move || t("加入提醒")}
                      </button>
                    </div>
                  </div>
                }
              })
              .collect_view()}
          </div>
          <p class="px-4 py-3 text-xs text-muted-foreground">
            {move || t("日期为 UTC 起始（常见规则推算），具体以主办方公告为准；「加入提醒」会把开赛时间写入倒计时，到期通过浏览器通知提醒。")}
          </p>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">{move || t("参赛小贴士")}</h2>
          <ul class="space-y-2 p-4">
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{move || t("竞赛常用「CQ TEST」呼叫，回答前先听清对方守听频率（如 UP 5）。")}</span>
            </li>
            <li class="flex gap-2 text-sm text-muted-foreground">
              <span class="mt-0.5 shrink-0 text-primary">"•"</span>
              <span>{move || t("赛后可到「竞赛录入」快速记录，并一键导出 Cabrillo 日志提交。")}</span>
            </li>
          </ul>
          <div class="px-4 pb-4">
            <a href="/contest-log" class="text-xs text-primary underline-offset-4 hover:underline">
              {move || t("前往竞赛录入 →")}
            </a>
          </div>
        </section>
      </div>
    </div>
  }
}

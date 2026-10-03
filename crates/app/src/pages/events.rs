//! 活动日历：展会 / 火腿节与年度通联活动，可按届加入倒计时提醒。

use ham_web_core::events::{HAM_EVENTS, HamEvent, next_start};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Size, Variant, button_class};
use crate::util::{alert, set_title};

/// 追踪 DX 远征与竞赛的延伸入口：`(路由, 标题, 说明)`。
const TRACKING_LINKS: &[(&str, &str, &str)] = &[
  (
    "/contest-calendar",
    "竞赛日历",
    "全球主要竞赛的开赛时间，可一键加入提醒。",
  ),
  ("/dxpedition", "DX 远征", "远征的玩法、波段与追逐技巧。"),
  (
    "/dx-spots",
    "DX 实时热点",
    "实时查看当前正在呼叫的远征电台。",
  ),
  ("/iota", "IOTA 海岛通联", "海岛编号与海岛通联活动。"),
  ("/portable", "SOTA / POTA", "山顶与公园活动，含编号查询。"),
  (
    "/most-wanted",
    "DXCC 稀有度榜单",
    "最稀有的实体，远征多奔这些目标。",
  ),
];

#[component]
pub fn EventsPage() -> impl IntoView {
  set_title(&t("活动日历"));

  let now = js_sys::Date::new_0();
  let now_t = (
    now.get_utc_full_year() as i32,
    now.get_utc_month() + 1,
    now.get_utc_date(),
  );

  // 按下一次开始时间排序。
  let mut scheduled: Vec<(&HamEvent, (i32, u32, u32))> = HAM_EVENTS
    .iter()
    .map(|e| (e, next_start(e, now_t)))
    .collect();
  scheduled.sort_by_key(|(_, (y, m, d))| (*y, *m, *d));

  let add_reminder = move |e: &'static HamEvent| {
    let (y, m, d) = next_start(e, now_t);
    let month_start = js_sys::Date::utc(y as f64, (m - 1) as f64);
    let target = month_start + (d as f64 - 1.0) * 86_400_000.0;
    super::countdown::add_countdown(&tf("活动：{}", &[e.name]), target as i64);
    alert(&tf("已把「{}」加入倒计时提醒", &[e.name]));
  };

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <h1 class="text-base font-semibold leading-tight">{move || t("活动日历")}</h1>
            <div class="text-xs text-muted-foreground">
              {move || t("展会 · 火腿节 · 年度通联活动")}
            </div>
          </div>
          <a href="/contest-calendar" class=button_class(Variant::Outline, Size::Sm, "")>
            {move || t("竞赛日历")}
          </a>
          <a href="/countdown" class=button_class(Variant::Outline, Size::Sm, "")>
            {move || t("我的倒计时")}
          </a>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">
            {move || t("全年活动（按下一届开始时间排序）")}
          </h2>
          <div class="divide-y">
            {scheduled
              .iter()
              .map(|(e, (y, m, d))| {
                let ev = *e;
                let dur = if ev.duration_days <= 1 {
                  t("1 天")
                } else {
                  tf("{} 天", &[&ev.duration_days.to_string()])
                };
                view! {
                  <div class="grid gap-2 px-4 py-3 sm:grid-cols-[1fr_auto]">
                    <div class="min-w-0">
                      <div class="flex flex-wrap items-baseline gap-x-2">
                        <span class="font-medium">{move || t(ev.name)}</span>
                        <span class="rounded-full border bg-muted/50 px-2 py-0.5 text-[10px] text-muted-foreground">
                          {move || t(ev.kind)}
                        </span>
                      </div>
                      <div class="mt-0.5 text-xs tabular-nums text-muted-foreground">
                        {tf(
                          "{} 年 {} 月 {} 日 · {} · {}",
                          &[
                            &y.to_string(),
                            &m.to_string(),
                            &d.to_string(),
                            &dur,
                            &t(ev.location),
                          ],
                        )}
                      </div>
                      <div class="mt-1 text-sm text-muted-foreground">{move || t(ev.desc)}</div>
                    </div>
                    <div class="flex items-center gap-2 sm:justify-end">
                      {(!ev.source.is_empty())
                        .then(|| {
                          view! {
                            <a
                              href=ev.source
                              target="_blank"
                              rel="noopener noreferrer"
                              class=button_class(Variant::Outline, Size::Sm, "")
                            >
                              {move || t("官网")}
                            </a>
                          }
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
            {move || {
              t(
                "日期为每年常见安排（多在周末，逐年浮动），以主办方公告为准；「加入提醒」会把开始时间写入倒计时，到期通过浏览器通知提醒。",
              )
            }}
          </p>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">
            {move || t("追踪 DX 远征与竞赛")}
          </h2>
          <div class="grid gap-2 p-4 sm:grid-cols-2">
            {TRACKING_LINKS
              .iter()
              .map(|(href, name, desc)| {
                view! {
                  <a
                    href=*href
                    class="flex flex-col gap-0.5 rounded-lg border px-3 py-2 transition-colors hover:bg-accent"
                  >
                    <span class="text-sm font-medium">{move || t(name)}</span>
                    <span class="text-xs text-muted-foreground">{move || t(desc)}</span>
                  </a>
                }
              })
              .collect_view()}
          </div>
        </section>
      </div>
    </div>
  }
}

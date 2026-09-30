use leptos::prelude::*;

use crate::util::set_title;

use super::countdown_section::CountdownSection;

/// 一项竞赛。
struct Contest {
  name: &'static str,
  months: &'static str,
  start: &'static [u32],
  desc: &'static str,
}

/// 主要竞赛。
const CONTESTS: &[Contest] = &[
  Contest {
    name: "CQ WW",
    months: "11 月（SSB）/ 12 月（CW）",
    start: &[11, 12],
    desc: "全球最大竞赛之一，交换 CQ 分区与序号。",
  },
  Contest {
    name: "CQ WPX",
    months: "3 月（SSB）/ 5 月（CW）",
    start: &[3, 5],
    desc: "按不同呼号前缀数量计分，交换序号。",
  },
  Contest {
    name: "IARU HF 锦标赛",
    months: "7 月",
    start: &[7],
    desc: "交换 ITU 分区号与参赛协会缩写。",
  },
  Contest {
    name: "ARRL DX",
    months: "2 月（CW）/ 3 月（SSB）",
    start: &[2, 3],
    desc: "仅与 DXCC 实体通联计分。",
  },
  Contest {
    name: "ARRL 十米竞赛",
    months: "12 月",
    start: &[12],
    desc: "仅限 10 米波段，交换州 / 省 / 序号。",
  },
  Contest {
    name: "CQ WW RTTY",
    months: "9 月",
    start: &[9],
    desc: "RTTY 模式的全球大赛，交换 CQ 分区与序号。",
  },
  Contest {
    name: "ARRL Field Day",
    months: "6 月第四个周末",
    start: &[6],
    desc: "野外应急设台 24 小时，北美最大型活动。",
  },
];

/// 参赛要点。
const CONTEST_TIPS: &[&str] = &[
  "竞赛中「TEST」表示比赛，呼叫用「CQ CONTEST」或「CQ TEST」。",
  "「UP 5」表示在发射频率上方 5kHz 守听，回答前先听清对方守听频率。",
  "参加竞赛应及时提交日志（常用 Cabrillo 格式），以便对方确认通联有效性。",
  "交换内容常见：信号报告 + 序号 / 分区 / 年龄等，须遵守竞赛规则。",
  "「B4」或「WKD」表示该台之前已通过，避免重复通联。",
];

/// 计算某项竞赛距下一次的月数（0 = 本月）。
fn months_until(start: &[u32], current: u32) -> (u32, u32) {
  for &m in start {
    if m >= current {
      return (m - current, m);
    }
  }
  let first = *start.first().unwrap_or(&1);
  (12 - current + first, first)
}

#[component]
pub fn ContestPage() -> impl IntoView {
  set_title("通联竞赛");
  // 当前月份（1–12），用于计算下一个竞赛。
  let current = js_sys::Date::new_0().get_month() + 1;

  let mut scheduled: Vec<(u32, u32, &'static Contest)> = CONTESTS
    .iter()
    .map(|c| {
      let (wait, month) = months_until(c.start, current);
      (wait, month, c)
    })
    .collect();
  scheduled.sort_by_key(|&(w, m, _)| (w, m));

  let next = scheduled.first().map(|&(w, m, c)| (w, m, c.name));

  view! {
    <div class="min-h-screen animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <header class="sticky top-0 z-20 border-b bg-background/90 backdrop-blur">
        <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-3 px-4 py-3">
          <div class="mr-auto">
            <div class="text-base font-semibold leading-tight">"通联竞赛"</div>
            <div class="text-xs text-muted-foreground">"CQ WW · WPX · IARU · ARRL"</div>
          </div>
        </div>
      </header>

      <div class="mx-auto max-w-5xl space-y-6 px-4 py-5">
        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"下一个竞赛"</h2>
          <div class="p-4">
            {match next {
              Some((0, m, name)) => view! {
                <div class="rounded-lg border bg-primary/10 px-4 py-3">
                  <div class="text-sm font-semibold">{format!("本月：{name}（{m} 月）")}</div>
                  <div class="mt-1 text-xs text-muted-foreground">"准备参赛，记得及时提交 Cabrillo 日志。"</div>
                </div>
              }
              .into_any(),
              Some((w, m, name)) => view! {
                <div class="rounded-lg border bg-primary/10 px-4 py-3">
                  <div class="text-sm font-semibold">{format!("下一场：{name}（{m} 月）")}</div>
                  <div class="mt-1 text-xs text-muted-foreground">{format!("约 {w} 个月后举行。")}</div>
                </div>
              }
              .into_any(),
              None => view! { <div></div> }.into_any(),
            }}
          </div>
        </section>

        <CountdownSection />

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"全年竞赛日历（按下届时间排序）"</h2>
          <div class="divide-y">
            {scheduled
              .iter()
              .map(|&(wait, _month, c)| {
                view! {
                  <div class="grid gap-1 px-4 py-3 sm:grid-cols-[10rem_11rem_6rem_1fr]">
                    <div class="font-medium">{c.name}</div>
                    <div class="text-xs text-muted-foreground">{c.months}</div>
                    <div class=if wait == 0 { "text-xs font-medium text-primary" } else { "text-xs text-muted-foreground" }>
                      {if wait == 0 { "本月".to_owned() } else { format!("约 {wait} 个月后") }}
                    </div>
                    <div class="text-sm text-muted-foreground">{c.desc}</div>
                  </div>
                }
              })
              .collect_view()}
          </div>
        </section>

        <section class="rounded-xl border bg-card">
          <h2 class="border-b px-4 py-3 text-sm font-semibold">"参赛要点"</h2>
          <ul class="space-y-2 p-4">
            {CONTEST_TIPS
              .iter()
              .map(|tip| {
                view! {
                  <li class="flex gap-2 text-sm text-muted-foreground">
                    <span class="mt-0.5 shrink-0 text-primary">"•"</span>
                    <span>{*tip}</span>
                  </li>
                }
              })
              .collect_view()}
          </ul>
        </section>
      </div>
    </div>
  }
}

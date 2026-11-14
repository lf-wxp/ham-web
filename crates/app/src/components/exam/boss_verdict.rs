//! 交卷结算的 QSL 卡（像素风，canvas「成绩 · QSL 卡」）：主色抬头、To / Call / Date / UTC 四栏、
//! 分数上屏 + 合格章 + 评级、信号报告（S 表，S9 = 合格线）、稀有台的堆叠还剩多少与收到的 QSL。
//!
//! 版式与导出的成绩图（`share_score::render_score_card`）是同一张卡；只写真的：呼号取日志设置的
//! 本台呼号、没有就写「未设置呼号」，日期与 UTC 是看到这张卡的此刻。

use ham_web_core::rig_panel::{exam_s_units, s_reading};
use ham_web_core::rpg::{Grade, XP_PER_EXAM, boss_result};
use ham_web_core::{Bank, ExamScore};
use leptos::prelude::*;

use crate::components::radio::SignalMeter;
use crate::i18n::{bank_class, t, tf, tp};
use crate::ui::{Progress, Readout, ReadoutTone};

/// 评级字母的颜色：S 金、A 绿、B 蓝、C 红。
const fn grade_color(grade: Grade) -> &'static str {
  match grade {
    Grade::S => "text-gold-text",
    Grade::A => "text-win-text",
    Grade::B => "text-xp-text",
    Grade::C => "text-hp-text",
  }
}

/// 交卷结算的 QSL 卡。
#[component]
pub fn BossVerdict(
  #[prop(into)] score: Signal<ExamScore>,
  #[prop(into)] pass_line: Signal<usize>,
  /// 薄弱项 / 自定义组卷：题目偏难或范围自选，不计入合格与满分的额外经验
  /// （与 `rpg::load_profile` 的口径一致），这里展示的回执也要同口径。
  #[prop(into)]
  weak: Signal<bool>,
  #[prop(into)] bank: Signal<Bank>,
) -> impl IntoView {
  let result = Signal::derive(move || {
    let s = score.get();
    boss_result(s.correct, s.total, pass_line.get())
  });
  let units = Signal::derive(move || {
    let s = score.get();
    exam_s_units(s.correct, s.total, pass_line.get())
  });
  // 通过与否只认 `boss_result` 这一个口径（评级、章、读数的颜色都读它）。
  let passed = Signal::derive(move || result.get().passed);
  // 呼号 / 日期 / UTC 是「看到这张卡的此刻」：弹窗每次打开都会重建内容（`Dialog` 的 `children` 在
  // 在场闭包里重跑），所以在这里读一次就是打开时的值；标签文字则放进下面的闭包，切语言要跟着变。
  let call = crate::share_score::own_callsign();
  let today = crate::util::local_today();
  let now = js_sys::Date::new_0();
  let utc = format!("{:02}:{:02}", now.get_utc_hours(), now.get_utc_minutes());
  // 信号报告的读法：合格线是 S9，高出 / 差几题读作多少。
  let reading_line = move || {
    let (s, pl) = (score.get(), pass_line.get());
    let reading = units.get().map_or_else(|| t("common.no-signal"), s_reading);
    let base = [pl.to_string(), s.total.to_string()];
    if s.correct >= pl {
      let n = s.correct - pl;
      if n == 0 {
        tf("exam.qsl-reading-at", &[&base[0], &base[1], &reading])
      } else {
        tp(
          "exam.qsl-reading-above",
          n,
          &[&base[0], &base[1], &n.to_string(), &reading],
        )
      }
    } else {
      let n = pl - s.correct;
      tp(
        "exam.qsl-reading-below",
        n,
        &[&base[0], &base[1], &n.to_string(), &reading],
      )
    }
  };
  let fields = move || {
    [
      (t("exam.qsl-to"), t("exam.qsl-to-value"), false),
      (
        t("exam.qsl-call"),
        call.clone().unwrap_or_else(|| t("exam.qsl-no-call")),
        call.is_none(),
      ),
      (t("exam.qsl-date"), today.clone(), false),
      (t("exam.qsl-utc"), utc.clone(), false),
    ]
  };
  view! {
    <section class="pxl-window" aria-labelledby="boss-verdict-title">
      <div class="flex items-center gap-2.5 border-b-2 border-ink bg-primary px-3.5 py-2.5 text-primary-foreground">
        <span class="pxl-title text-xs">{move || t("exam.qsl-code")}</span>
        <span class="text-xs">{move || tf("exam.qsl-head", &[&bank_class(bank.get().as_str())])}</span>
      </div>
      <div class="space-y-3 p-3.5">
        <h3 id="boss-verdict-title" class="pxl-title text-sm">
          {move || if result.get().passed { t("rpg.boss-defeated") } else { t("rpg.boss-survived") }}
        </h3>
        <dl class="grid grid-cols-2 gap-2">
          {move || {
            fields()
              .into_iter()
              .map(|(label, value, muted)| {
                view! {
                  <div class="border-b-2 border-ink/20 pb-1">
                    <dt class="pxl-label text-xs text-muted-foreground">{label}</dt>
                    <dd class=if muted { "mt-0.5 text-xs text-muted-foreground" } else { "mt-0.5 text-xs" }>{value}</dd>
                  </div>
                }
              })
              .collect_view()
          }}
        </dl>
        <div class="flex flex-wrap items-center gap-3">
          <div class="pxl-display flex items-end gap-1.5 px-3 py-2.5">
            <Readout
              text=Signal::derive(move || score.get().correct.to_string())
              label=Signal::derive(move || {
                let s = score.get();
                tf("exam.qsl-score", &[&s.correct.to_string(), &s.total.to_string()])
              })
              scale=4
              tone=Signal::derive(move || if passed.get() { ReadoutTone::Ok } else { ReadoutTone::Hot })
            />
            <span class="pxl-label pxl-d-dim text-xs" aria-hidden="true">{move || format!("/ {}", score.get().total)}</span>
          </div>
          // 合格章：双线框 + PASS / FAIL 丝印（不只靠颜色：字与框都在）。中文结论在卡片下面那一行，章里不重复。
          <div class="pxl-qsl-stamp px-3 py-2 text-center" data-pass=move || passed.get().then_some("")>
            <span class="pxl-label block text-base">
              {move || if passed.get() { t("exam.qsl-pass") } else { t("exam.qsl-fail") }}
            </span>
          </div>
          <div class="ml-auto flex flex-col items-center">
            <span class="pxl-label text-xs text-muted-foreground">{move || t("rpg.grade")}</span>
            <span class=move || format!("pxl-title text-3xl leading-none {}", grade_color(result.get().grade))>
              {move || result.get().grade.letter()}
            </span>
          </div>
        </div>
        <div class="pxl-display space-y-1 p-2">
          <div class="flex items-center justify-between gap-2">
            <span class="pxl-label pxl-d-dim text-xs">{move || t("exam.signal-report")}</span>
            {move || {
              let u = units.get();
              view! {
                <Readout
                  text=u.map_or_else(|| "--".to_owned(), s_reading)
                  label=u.map_or_else(|| t("common.no-signal"), s_reading)
                  tone=ReadoutTone::for_s_units(u)
                />
              }
            }}
          </div>
          <SignalMeter units=units label=Signal::derive(move || t("exam.signal-report")) />
        </div>
        <p class="text-xs text-muted-foreground">{reading_line}</p>
        <Progress
          value=Signal::derive(move || i64::from(result.get().boss_left_percent))
          label=Signal::derive(move || t("rpg.boss-hp"))
          class="pxl-bar-hp"
        />
        <div class="flex flex-wrap items-baseline justify-between gap-2 text-xs text-muted-foreground">
          <span>{move || tf("rpg.boss-hp-left", &[&result.get().boss_left_percent.to_string()])}</span>
          <span class="pxl-label text-gold-text">
            {move || {
              let xp = if weak.get() { XP_PER_EXAM } else { result.get().xp };
              format!("{} {}", t("rpg.loot"), tf("rpg.reward-xp", &[&xp.to_string()]))
            }}
          </span>
        </div>
        <div class="border-t-2 border-dashed border-ink/20 pt-2">
          <span class="pxl-label text-xs text-muted-foreground">{move || t("exam.qsl-signoff")}</span>
        </div>
      </div>
    </section>
  }
}

//! 考试页的 Boss 舞台：考试进行中 Boss 被「封印」，交卷后按答对数结算伤害。
//!
//! 为什么不在作答时就扣 Boss 的血：真实考试里作答时不告诉你对错，扣血等于泄露答案，
//! 也让「机考仿真」失去意义。所以进行中只展示三样不涉及对错的东西 —— 封印提示、
//! 已作答进度、剩余时间；交卷后血条才一次性落到最终值（宽度过渡是 `steps()` 的，
//! 一格一格掉）。
//!
//! 版式是 canvas 的「模拟考试 · ON AIR」：上面一行是红框 ON AIR（在考灯 + 状态）与一块大号倒计时屏，
//! 下面一行收成紧凑的稀有 DX 台状态条（小天线塔 + 守听提示 + 堆叠剩余 + 已发送）。手机上最要紧的是
//! **题干**，舞台只占两行。

use ham_web_core::ExamRule;
use ham_web_core::exam_clock::{ClockTone, clock_tone};
use ham_web_core::rpg::boss_hp_percent;
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::icons::PixelSprite;
use crate::ui::{Led, LedState, LedTone, Progress};

use super::exam_clock::ExamClock;

/// Boss 舞台。
#[component]
pub(super) fn BossStage(
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] total: Signal<usize>,
  /// 剩余毫秒数。
  #[prop(into)]
  remaining: Signal<i64>,
  #[prop(into)] rule: Signal<ExamRule>,
  #[prop(into)] finished: Signal<bool>,
  /// 交卷后的答对数；交卷前不读。
  #[prop(into)]
  correct: Signal<usize>,
) -> impl IntoView {
  let boss_hp = Signal::derive(move || {
    if finished.get() {
      i64::from(boss_hp_percent(correct.get(), rule.get().pass))
    } else {
      100
    }
  });
  let timed = Signal::derive(move || rule.get().minutes > 0);
  let tone = Signal::derive(move || clock_tone(remaining.get(), rule.get().duration_ms()));
  let sealed_hint = move || {
    if finished.get() {
      tf("rpg.boss-hp-left", &[&boss_hp.get().to_string()])
    } else {
      t("rpg.boss-sealed")
    }
  };
  // 读数屏左上的丝印：档位名不写具体分钟数 —— 提醒线 / 冲刺线会按总时长封顶（`core::exam_clock`）。
  let clock_code = move || match tone.get() {
    ClockTone::Calm => t("exam.clock-calm"),
    ClockTone::Warn => t("exam.clock-warn"),
    ClockTone::Danger => t("exam.clock-danger"),
  };
  view! {
    <section class="space-y-2" aria-label=move || t("rpg.boss-name")>
      {move || {
        timed
          .get()
          .then(|| {
            // 读数不加 `aria-live`（见 `ExamClock`）：倒计时每秒变一次，朗读出来是持续噪声，
            // 跨档的提醒由 Toast 负责。ON AIR 框是装饰（考试进行中这件事页面本身就在说）。
            view! {
              <div class="flex items-stretch gap-2">
                <div class="pxl-onair flex shrink-0 flex-col justify-center gap-1 px-2.5 py-1.5" aria-hidden="true">
                  <span class="flex items-center gap-1.5">
                    <Led
                      tone=LedTone::Tx
                      state=Signal::derive(move || {
                        if finished.get() {
                          LedState::Off
                        } else if tone.get() == ClockTone::Danger {
                          LedState::Blink
                        } else {
                          LedState::On
                        }
                      })
                    />
                    <span class="pxl-label text-xs text-hp-text">{move || t("exam.on-air-code")}</span>
                  </span>
                  <span class="text-xs text-muted-foreground">
                    {move || if finished.get() { t("exam.submitted") } else { t("exam.on-air-state") }}
                  </span>
                </div>
                <div class="pxl-display flex min-w-0 flex-1 items-center justify-between gap-2 px-2.5 py-2">
                  <span class="pxl-label pxl-d-dim text-xs" aria-hidden="true">{clock_code}</span>
                  <ExamClock remaining=remaining rule=rule />
                </div>
              </div>
            }
          })
      }}
      <div class="pxl-card flex items-center gap-3 border bg-card p-2.5">
        <div class=move || if finished.get() && boss_hp.get() == 0 { "shrink-0 pxl-flash" } else { "shrink-0" }>
          <PixelSprite name="boss" scale=2 />
        </div>
        <div class="min-w-0 flex-1 space-y-1.5">
          <div class="flex flex-wrap items-baseline justify-between gap-x-2">
            <span class="pxl-label text-xs">{move || t("rpg.boss-name")}</span>
            <span class="text-xs text-muted-foreground">{sealed_hint}</span>
          </div>
          <Progress value=boss_hp label=Signal::derive(move || t("rpg.boss-hp")) class="pxl-bar-hp h-2" />
          <span class="block text-xs tabular-nums text-muted-foreground">
            {move || tf("rpg.boss-answered", &[&answered.get().to_string(), &total.get().to_string()])}
          </span>
        </div>
      </div>
    </section>
  }
}

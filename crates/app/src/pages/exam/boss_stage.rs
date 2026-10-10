//! 考试页的 Boss 舞台：考试进行中 Boss 被「封印」，交卷后按答对数结算伤害。
//!
//! 为什么不在作答时就扣 Boss 的血：真实考试里作答时不告诉你对错，扣血等于泄露答案，
//! 也让「机考仿真」失去意义。所以进行中只展示三样不涉及对错的东西 —— 封印提示、
//! 已作答进度、剩余时间；交卷后血条才一次性落到最终值（宽度过渡是 `steps()` 的，
//! 一格一格掉）。

use ham_web_core::ExamRule;
use ham_web_core::rpg::boss_hp_percent;
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::icons::PixelSprite;
use crate::ui::Progress;

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
  let answered_percent = Signal::derive(move || {
    // 没有题（尚未加载 / 题库为空）时 `checked_div` 返回 `None`，进度按 0 处理。
    (answered.get() * 100)
      .checked_div(total.get())
      .map_or(0, |p| i64::try_from(p).unwrap_or(100))
  });
  // `minutes == 0` 表示不限时（自定义组卷）：没有时间条可画。
  let time_percent = Signal::derive(move || {
    let total_ms = rule.get().duration_ms();
    if total_ms <= 0 {
      0
    } else {
      (remaining.get().clamp(0, total_ms) * 100) / total_ms
    }
  });
  let timed = Signal::derive(move || rule.get().minutes > 0);

  view! {
    <section class="pxl-window flex items-center gap-4 p-3" aria-label=move || t("rpg.boss-name")>
      <div class=move || if finished.get() && boss_hp.get() == 0 { "pxl-flash" } else { "pxl-bob" }>
        <PixelSprite name="boss" scale=5 />
      </div>
      <div class="min-w-0 flex-1 space-y-2">
        <div class="flex flex-wrap items-baseline justify-between gap-2">
          <span class="pxl-label text-xs">{move || t("rpg.boss-name")}</span>
          <span class="text-xs text-muted-foreground">
            {move || {
              if finished.get() {
                tf("rpg.boss-hp-left", &[&boss_hp.get().to_string()])
              } else {
                t("rpg.boss-sealed")
              }
            }}
          </span>
        </div>
        <Progress value=boss_hp label=Signal::derive(move || t("rpg.boss-hp")) class="pxl-bar-hp" />
        <div class="grid gap-3 sm:grid-cols-2">
          <div class="space-y-1">
            <div class="text-xs tabular-nums text-muted-foreground">
              {move || tf("rpg.boss-answered", &[&answered.get().to_string(), &total.get().to_string()])}
            </div>
            // 这条就是「答题进度」，沿用 `Progress` 的默认读屏名。
            <Progress value=answered_percent class="pxl-bar-xp h-3" />
          </div>
          {move || {
            timed
              .get()
              .then(|| {
                view! {
                  <div class="space-y-1">
                    <div class="text-xs text-muted-foreground">{t("learning.time-left")}</div>
                    <Progress
                      value=time_percent
                      label=Signal::derive(move || t("learning.time-left"))
                      class="pxl-bar-gold pxl-bar-danger h-3"
                    />
                  </div>
                }
              })
          }}
        </div>
      </div>
    </section>
  }
}

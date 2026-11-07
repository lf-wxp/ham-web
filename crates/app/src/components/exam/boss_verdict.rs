//! 交卷结算里的 Boss 判词：评级、Boss 还剩多少血、战利品。

use ham_web_core::ExamScore;
use ham_web_core::rpg::{Grade, XP_PER_EXAM, boss_result};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::icons::PixelSprite;
use crate::ui::Progress;

/// 评级字母的颜色：S 金、A 绿、B 蓝、C 红。
const fn grade_color(grade: Grade) -> &'static str {
  match grade {
    Grade::S => "text-gold-text",
    Grade::A => "text-win-text",
    Grade::B => "text-xp-text",
    Grade::C => "text-hp-text",
  }
}

/// Boss 判词。
#[component]
pub fn BossVerdict(
  #[prop(into)] score: Signal<ExamScore>,
  #[prop(into)] pass_line: Signal<usize>,
  /// 薄弱项 / 自定义组卷：题目偏难或范围自选，不计入合格与满分的额外经验
  /// （与 `rpg::load_profile` 的口径一致），这里展示的战利品也要同口径。
  #[prop(into)]
  weak: Signal<bool>,
) -> impl IntoView {
  let result = Signal::derive(move || {
    let s = score.get();
    boss_result(s.correct, s.total, pass_line.get())
  });
  // 像素风：打赢了是奖杯精灵，没打赢是那只 Boss。
  let sprite = Signal::derive(move || {
    if result.get().passed {
      "badge_trophy"
    } else {
      "boss"
    }
  });
  view! {
    <section class="pxl-window flex items-center gap-4 p-3" aria-labelledby="boss-verdict-title">
      <div class="flex shrink-0 flex-col items-center">
        <span class="pxl-label text-xs text-muted-foreground">{move || t("rpg.grade")}</span>
        <span
          class=move || format!("pxl-title text-5xl leading-none {}", grade_color(result.get().grade))
        >
          {move || result.get().grade.letter()}
        </span>
      </div>
      <div class="min-w-0 flex-1 space-y-2">
        <h3 id="boss-verdict-title" class="pxl-title text-sm">
          {move || if result.get().passed { t("rpg.boss-defeated") } else { t("rpg.boss-survived") }}
        </h3>
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
      </div>
      {move || {
        if crate::kit::is_modern() {
          // 打赢了是奖杯纹章，没打赢就该看见那只 Boss —— 两种图形不是一类东西，
          // 不要都塞进纹章里（奖杯是身份，Boss 是角色）。直接读 `result`，
          // 评分变化（重新交卷）时这一块跟着换，不依赖字符串比较。
          if result.get().passed {
            view! {
              <crate::kit_modern::ModernEmblem
                icon=crate::kit_modern::emblem_icon("badge_trophy")
                tone=crate::kit_modern::emblem_tone("badge_trophy")
                size=72
                locked=Signal::derive(|| false)
              />
            }
              .into_any()
          } else {
            view! { <crate::kit_modern::ModernMonster sprite="boss" size=72 /> }.into_any()
          }
        } else {
          view! { <PixelSprite name=sprite scale=4 class="pxl-bob" /> }.into_any()
        }
      }}
    </section>
  }
}

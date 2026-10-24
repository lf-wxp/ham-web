use ham_web_core::ExamRule;
use ham_web_core::exam_clock::{ClockTone, clock_tone};
use ham_web_core::text::format_ms;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

/// 考试倒计时读数：HUD 字体 + 档位图标。
///
/// 为什么抽成组件：同一个读数以前在顶部元信息和 Boss 舞台里各抄一份，阈值和配色也各写一遍
/// （都是「最后 60 秒变红」一级）。现在档位由 `ham_web_core::exam_clock` 判定，读数只管渲染。
///
/// 档位用**图标 + 颜色**两个通道表达（提醒区是时钟、冲刺区是警示），不只靠红色；
/// 也不闪烁 —— 读数是用户要反复读的东西，闪烁会让它在一半的时间里读不出来。
/// 不限时（`minutes == 0`，自定义组卷）时调用方不该渲染它。
///
/// 不加 `aria-live`：读数每秒变一次，朗读出来是持续噪声；跨档的提醒由 Toast 负责。
#[component]
pub(super) fn ExamClock(
  #[prop(into)] remaining: Signal<i64>,
  #[prop(into)] rule: Signal<ExamRule>,
) -> impl IntoView {
  move || {
    let ms = remaining.get();
    let (tone_class, icon) = match clock_tone(ms, rule.get().duration_ms()) {
      ClockTone::Calm => ("text-foreground", None),
      ClockTone::Warn => ("text-gold-text", Some(IconKind::Clock)),
      ClockTone::Danger => ("text-hp-text", Some(IconKind::AlertCircle)),
    };
    view! {
      <span
        data-slot="exam-clock"
        class=format!("pxl-label inline-flex items-center gap-1 tabular-nums {tone_class}")
      >
        {icon.map(|kind| view! { <Icon kind=kind class="size-6 shrink-0" /> })}
        {format_ms(ms)}
      </span>
    }
  }
}

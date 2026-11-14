use ham_web_core::ExamRule;
use ham_web_core::exam_clock::{ClockTone, clock_tone};
use ham_web_core::text::format_ms;
use leptos::prelude::*;

use crate::icons::{Icon, IconKind};

/// 底部操作栏里的剩余时间：一行文字 `MM:SS`，不是七段屏。
///
/// 为什么底栏还要放一份：长题 / 看图滚动后，上方 ON AIR 读数屏早已滚出视口，而限时考试里
/// 剩余时间是最不能丢的信息。这里刻意用最朴素的文字（每秒只改一个文本节点），
/// 不再画第二块七段屏。
///
/// 档位用**图标 + 颜色**两个通道表达（提醒区是时钟、冲刺区是警示），不只靠红色；不闪烁。
/// 不加 `aria-live`：读数每秒变一次，朗读出来是持续噪声；跨档的提醒由 Toast 负责。
/// `data-slot="exam-clock"` 与屏幕上那份同名：e2e 按「`main` 内 / 操作栏内」区分。
/// 不限时（`minutes == 0`，自定义组卷）时调用方不该渲染它。
#[component]
pub(super) fn ExamBarClock(
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

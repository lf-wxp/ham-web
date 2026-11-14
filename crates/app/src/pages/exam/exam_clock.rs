use ham_web_core::ExamRule;
use ham_web_core::exam_clock::{ClockTone, clock_tone};
use ham_web_core::text::format_ms;
use leptos::prelude::*;

use crate::ui::{Readout, ReadoutTone};

/// 考试倒计时读数：一块屏幕上的七段 `MM:SS`，档位配色由 `ham_web_core::exam_clock` 判定。
///
/// **屏幕上只有这一份七段读数**：它坐在 Boss 舞台的 ON AIR 读数屏上，页头元信息不再抄一份
/// （`docs/ui/pixel.md` §3.4）。底栏另有一份朴素文字（`ExamBarClock`），滚动后仍可见。
/// 「在考」灯与丝印在旁边的 ON AIR 框里，读数本身只画数字。
///
/// 档位只用**颜色**一个通道（`ReadoutTone`）：提醒区琥珀、冲刺区橙红；读数本身不闪 ——
/// 它是用户要反复读的东西，冲刺区闪的是 ON AIR 框里的红灯（`docs/ui/motion.md` §8）。
/// 不限时（`minutes == 0`，自定义组卷）时调用方不该渲染它。
///
/// `data-slot="exam-clock"` 的文本就是 `MM:SS`（七段图形 `aria-hidden`，旁边一份 `sr-only` 文字）：
/// 读屏与 e2e 都按这串字读。不加 `aria-live`：读数每秒变一次，朗读出来是持续噪声；跨档的提醒由 Toast 负责。
#[component]
pub(super) fn ExamClock(
  #[prop(into)] remaining: Signal<i64>,
  #[prop(into)] rule: Signal<ExamRule>,
) -> impl IntoView {
  let tone = move || clock_tone(remaining.get(), rule.get().duration_ms());
  let readout = move || {
    let readout = match tone() {
      ClockTone::Calm => ReadoutTone::Normal,
      ClockTone::Warn => ReadoutTone::Hot,
      ClockTone::Danger => ReadoutTone::Alarm,
    };
    let text = format_ms(remaining.get());
    view! {
      <Readout text=text.clone() tone=readout scale=3 />
      <span class="sr-only">{text}</span>
    }
  };
  view! { <span data-slot="exam-clock" class="inline-flex items-center">{readout}</span> }
    .into_any()
}

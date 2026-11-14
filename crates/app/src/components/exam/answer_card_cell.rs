use leptos::prelude::*;

use crate::cn::cn;
use crate::ui::{Button, Led, LedState, LedTone, Size, Variant};

use super::answer_card_model::cell_label;

/// 答题卡里的一格题号（像素风叫它「存储频道」，canvas 的答题卡格，`docs/ui/pixel.md` §3.4）。
///
/// 状态用多个通道表达，不靠颜色单独成立：
/// - 右上角一盏灯：标记 = 琥珀、当前题 = 蓝、已答 = 绿、未答 = 灭；
/// - 已答 = 凹槽底；当前题 = 主色描边加一道内框 + `aria-current`；
/// - 交卷后 = 右下角 ✓ / ✗ 小牌。
///
/// 题号写在左下角、不补零：按钮的可见文本就是题号本身（读屏 / e2e 都靠它），状态另由 `aria-label` 说明。
///
/// 整格在父级 `grid` 闭包里随答案 / 筛选 / 当前题重建，所以 `class` 取静态值即可。
///
/// 刻意**不**在格子里放「点击看题图」的触发器：格子只有 36px 高，任何一个 16px 的角标都会
/// 压掉题号按钮一块可点区域（点偏了就变成开图而不是跳题）。题图在题目卡片里本来就能放大看。
#[component]
pub fn AnswerCardCell(
  /// 题号（从 1 起）。
  number: usize,
  answered: bool,
  current: bool,
  flagged: bool,
  /// 交卷后的对错；`None` 表示还在作答中。
  verdict: Option<bool>,
  /// 机考仿真：不能回看，格子置灰而不是点了没反应。
  locked: bool,
  on_pick: Callback<()>,
) -> impl IntoView {
  let class = cn(&[
    "relative h-9 w-full items-end justify-start px-1.5 pb-0.5 pointer-coarse:h-11",
    if answered { "bg-inset" } else { "" },
    if current {
      "border-primary outline-2 outline-primary -outline-offset-[6px]"
    } else {
      ""
    },
  ]);
  let lamp = if flagged {
    LedTone::Warn
  } else if current {
    LedTone::Info
  } else {
    LedTone::Rx
  };
  // 无障碍名：题号 + 状态。可见文本仍是纯题号（label-in-name 成立）；规则与现代版共用。
  let label = cell_label(number.to_string(), flagged, verdict);
  view! {
    <Button
      variant=Variant::Outline
      size=Size::Sm
      class=class
      disabled=locked
      aria_current=current
      aria_label=label
      on_click=Callback::new(move |_| on_pick.run(()))
    >
      <span class="pxl-label text-xs">{number}</span>
      <Led
        tone=lamp
        state=LedState::lit(flagged || current || answered)
        small=true
        class="absolute top-1 right-1"
      />
      {verdict
        .map(|correct| {
          let tone = if correct {
            "border-win text-win-text after:content-['✓']"
          } else {
            "border-destructive text-hp-text after:content-['✗']"
          };
          view! {
            <span
              data-slot="badge"
              aria-hidden="true"
              class=format!("absolute -bottom-1 -right-1 flex size-5 items-center justify-center border-2 bg-card text-xs {tone}")
            ></span>
          }
        })}
    </Button>
  }
}

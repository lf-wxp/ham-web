use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Button, Size, Variant};

/// 答题卡里的一格题号。
///
/// 状态用多个通道表达，不靠颜色单独成立：
/// - 已答 = 主色实心；未答 = 描边空心（`Variant` 不同，形状上就能分）；
/// - 当前题 = 底部加粗的金色下沿 + `aria-current`（以前完全看不出自己在哪一题）；
/// - 标记 = 右上角带 `!` 的小牌；交卷后 = 右下角 ✓ / ✗ 小牌。
///
/// 整格在父级 `grid` 闭包里随答案 / 筛选 / 当前题重建，所以 `variant` 与 `class` 取静态值即可，
/// 不需要退回原生 `<button>`。
///
/// 刻意**不**在格子里放「点击看题图」的触发器：格子只有 36px 见方，任何一个 16px 的角标都会
/// 压掉题号按钮一块可点区域（点偏了就变成开图而不是跳题）。题图在题目卡片里本来就能放大看，
/// 这里只负责跳题。
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
  let class = if current {
    "relative h-9 w-full border-b-4 border-b-gold"
  } else {
    "relative h-9 w-full"
  };
  let variant = if answered {
    Variant::Default
  } else {
    Variant::Outline
  };
  // 无障碍名：题号 + 状态。可见文本仍是纯题号（label-in-name 成立）。
  let label = {
    let mut parts = vec![number.to_string()];
    if flagged {
      parts.push(t("exam.flagged"));
    }
    match verdict {
      Some(true) => parts.push(t("exam.correct-label")),
      Some(false) => parts.push(t("exam.wrong-label")),
      None => {}
    }
    parts.join(" · ")
  };
  view! {
    <Button
      variant=variant
      size=Size::Sm
      class=class
      disabled=locked
      aria_current=current
      aria_label=label
      on_click=Callback::new(move |_| on_pick.run(()))
    >
      <span>{number}</span>
      // 角标的字符用伪元素画（`after:content-[…]`），不进按钮的文本内容：题号按钮的文本就是
      // 题号本身（读屏 / e2e 都靠它），状态另由 `aria-label` 说明。
      {flagged
        .then(|| {
          view! {
            <span
              aria-hidden="true"
              class="pxl-label absolute -top-1 -right-1 flex size-5 items-center justify-center border-2 border-ink bg-accent text-xs text-accent-foreground after:content-['!']"
            ></span>
          }
        })}
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

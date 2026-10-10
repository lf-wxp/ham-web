use leptos::prelude::*;

use crate::cn::cn;
use crate::i18n::t;

use super::control::TextValue;

/// 进度条：像素「分格条」。
///
/// 槽是凹陷的深色，填充按格推进（宽度过渡用 `steps()`，不是平滑拉伸）。
/// 传 `class="pxl-bar-hp"` / `pxl-bar-xp` / `pxl-bar-win` / `pxl-bar-gold` 可换成血条 / 经验条 /
/// 通关绿 / 金币色；`data-low` 在低于 25% 时让填充闪烁，提示「危险」（只给血条语义用）。
///
/// `label` 是读屏用的名字；不传时沿用「答题进度」。血条 / 经验条这类**不是答题进度**的条
/// 必须传 —— 否则读屏会把玩家生命念成「答题进度 60%」。
#[component]
pub fn Progress(
  #[prop(into)] value: Signal<i64>,
  #[prop(optional, into)] label: Option<TextValue>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let class = cn(&["pxl-bar", &class]);
  view! {
    <div
      data-slot="progress"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow=move || value.get().clamp(0, 100).to_string()
      aria-label=move || {
        label
          .as_ref()
          .map_or_else(|| t("common.answering-progress"), TextValue::get)
      }
      data-low=move || (value.get().clamp(0, 100) < 25).to_string()
      class=class
    >
      <div
        data-slot="progress-indicator"
        class="pxl-bar-fill"
        style=move || format!("width: {}%", value.get().clamp(0, 100))
      ></div>
    </div>
  }
}

//! 数据条里的一格读数。
//!
//! 抽出来的唯一原因：数字要「滚」到目标值。像素版靠 `ui::Stat` 做同一件事，
//! 现代风的缓动与时长不同（expo-out / 760ms），所以是两份实现而不是复用。
//!
//! 首次挂载从 0 起跳：数据条通常紧跟在页头之后，滚上来时报数正好落位，
//! 比「一睁眼就是最终值」更像有人在后台算。动效被关时直接落位（`use_count_up` 内部判定）。

use leptos::prelude::*;

use crate::motion::use_count_up;

/// 一格读数：大号数字 + 单位后缀。`value` 为 `None` 时显示占位符（如正确率没有作答时）。
#[component]
pub fn ModernStatValue(
  #[prop(into)] value: Signal<Option<usize>>,
  /// 单位后缀，例如 `%`。不参与滚动。
  #[prop(optional, into)]
  suffix: String,
) -> impl IntoView {
  let numeric = Signal::derive(move || value.get().unwrap_or(0));
  let display = use_count_up(numeric, true);
  let has_value = Signal::derive(move || value.get().is_some());
  let suffix = StoredValue::new(suffix);

  view! {
    <span class="md-strip-value">
      {move || {
        if has_value.get() {
          view! {
            <>
              {move || display.get()}
              {suffix.get_value()}
            </>
          }
            .into_any()
        } else {
          view! { "—" }.into_any()
        }
      }}
    </span>
  }
}

//! 「今日值机」里的一行：灯 + 任务名 + 右侧进度（canvas 的值机列表，`docs/ui/pixel.md` §3.4）。

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Led, LedState, LedTone};

/// 一行任务。整行是一个链接：点哪里都能出发。说明文字（连续天数、奖励）放在 `title` 与 `sr-only` 里，行内只留进度。
#[component]
pub(super) fn QuestRow(
  #[prop(into)] href: String,
  #[prop(into)] title: Signal<String>,
  #[prop(into)] hint: Signal<String>,
  /// 右侧进度（「9 / 10」「3 条」「12 / 30」），只写真实数据。
  #[prop(into)]
  count: Signal<String>,
  done: bool,
) -> impl IntoView {
  view! {
    <li class="border-b-2 border-dashed border-ink/20 last:border-b-0">
      <a
        href=href
        title=move || hint.get()
        class="motion-press flex min-h-11 items-center gap-2 py-2 text-sm hover:bg-accent"
      >
        // 灯：绿 = 今天做完了，琥珀灭 = 还没做。右边的进度是同一件事的文字版。
        <Led tone=if done { LedTone::Rx } else { LedTone::Warn } state=LedState::lit(done) small=true />
        <span class="min-w-0 flex-1">{move || title.get()}</span>
        <span class=if done { "pxl-label text-xs text-win-text" } else { "pxl-label text-xs text-muted-foreground" }>
          {move || count.get()}
        </span>
        // 灯是装饰（`aria-hidden`）、说明只在 `title` 里（触屏与读屏拿不到）：状态与说明另放一份
        // `sr-only` 文字，名字里就带上「已完成」与连续天数 / 奖励。
        <span class="sr-only">
          {move || {
            if done { format!("{} · {}", t("rpg.done-tag"), hint.get()) } else { hint.get() }
          }}
        </span>
      </a>
    </li>
  }
}

//! 实时数据降级提示：挂在页头下面，把「只有实时部分不可用」与「整页实时数据不可用」分开说。
//!
//! 页面各自的错误态只负责「这一块没有数据」，全局这一条负责「这一页的数据是什么来路」：
//! 十几个实时页若各写一份措辞必然漂移，且页面无法知道某块数据其实来自离线快照。
//! 用 `aria-live="polite"` 而不挂 `role="status"`：降级是常态信息，不该打断读屏用户；
//! 且全站 toast 与多处状态播报都由 `getByRole("status")` 定位（含 `toHaveCount(0)` 断言），
//! 这里再多一个同名 role 会让那些断言串台。
//!
//! 也不放「重试」按钮：各实时页自己已有刷新 / 重试入口，再放一个同名按钮既让用户分不清
//! 该点哪个，也会让按名称定位的用例撞上严格模式。

use ham_web_core::data_status::{PageDataState, summarize_origins};
use leptos::prelude::*;

use crate::i18n::{t, tf};

#[component]
pub fn DataStatusNotice() -> impl IntoView {
  let state = Signal::derive(move || {
    // 换页时 `reset_page_api_status` 会清空记录（见 `app/main_content.rs`），
    // 所以这里读到的永远是当前页的来源，不必再比对路径。
    let status = crate::data::page_api_status().get();
    let origins: Vec<_> = status.origins.values().copied().collect();
    summarize_origins(&origins)
  });

  view! {
    {move || {
      let (kind, text) = match state.get() {
        PageDataState::Live => return ().into_any(),
        PageDataState::Snapshot {
          cached_at_ms,
          incomplete,
        } => {
          let time = crate::data::format_snapshot_time(cached_at_ms);
          // 同页还有块既无实时也无快照时，另用一句如实说「有的块是空的」。
          let text = if incomplete {
            tf("common.data-snapshot-incomplete-notice", &[&time])
          } else {
            tf("common.data-snapshot-notice", &[&time])
          };
          ("snapshot", text)
        }
        PageDataState::Partial => ("partial", t("common.data-partial-notice")),
        PageDataState::Unavailable => ("unavailable", t("common.data-unavailable-notice")),
      };
      view! {
        <div
          aria_live="polite"
          data-data-status=kind
          class="flex flex-wrap items-center gap-2 border-b bg-card px-4 py-2 text-xs text-muted-foreground"
        >
          <span class="min-w-0 flex-1">{text}</span>
        </div>
      }
      .into_any()
    }}
  }
}

//! [`Pagination`]：列表分页条。

use leptos::prelude::*;

use ham_web_core::pagination::{PageItem, page_window};

use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::ui::{Size, TextValue, Variant, button_class};

/// 分页条：上一页 / 页码窗口 / 下一页。
///
/// - `page` 与 `total_pages` 都从 1 起；`total_pages <= 1` 时整个组件不渲染。
/// - 页码窗口（首尾固定、邻域展开、省略号取舍）的纯逻辑在
///   `ham_web_core::pagination`（带单测）。
/// - 当前页样式由 `aria-current="page"` 驱动（`controls.css` 的
///   `.pxl-btn[aria-current="page"]`），不在 Rust 里拼 ON / OFF 类名。
///
/// 内部直接用 `button_class` 而不是 [`crate::ui::Button`]：页码按钮的「当前页高亮」
/// 依赖 `aria-current` 属性动态切换，`Button` 的 `class` 是静态串、也没有
/// `aria_current` 通道 —— 这是 §10 之外不成立的「刻意的例外」清单不收录的理由：
/// 分页条住在 `ui/` 里，样式与 [`Button`] 同源（`button_class`）。
#[component]
pub fn Pagination(
  #[prop(into)] page: Signal<u32>,
  #[prop(into)] total_pages: Signal<u32>,
  on_change: Callback<u32>,
  /// 无障碍名（读屏与 e2e 定位用）；缺省是词条 `common.pagination`。
  #[prop(optional, into)]
  aria_label: Option<TextValue>,
) -> impl IntoView {
  // `StoredValue`（Copy）：内层属性闭包要求 `'static`，直接 move `Option<TextValue>`
  // 会让外层重渲染块退化成 `FnOnce`（见 docs/ui-components.md §8 坑 1）。
  let aria_label = StoredValue::new(aria_label.filter(|v| !v.is_empty()));
  let btn = button_class(Variant::Outline, Size::Sm, "");

  view! {
    {move || {
      let total = total_pages.get();
      let current = page.get();
      if total <= 1 {
        return ().into_any();
      }
      let items = page_window(current, total);
      view! {
        <nav
          role="navigation"
          data-slot="pagination"
          aria-label=move || aria_label.with_value(|v| v.as_ref().map(TextValue::get).unwrap_or_else(|| t("common.pagination")))
        >
          <div class="flex flex-wrap items-center gap-1">
            <button
              type="button"
              class=btn.clone()
              disabled=current <= 1
              aria-label=move || t("common.prev-page")
              on:click=move |_| {
                if current > 1 {
                  on_change.run(current - 1);
                }
              }
            >
              <Icon kind=IconKind::ChevronLeft class="size-6" />
            </button>
            {items
              .into_iter()
              .map(|item| match item {
                PageItem::Page(p) => {
                  let is_current = p == current;
                  view! {
                    <button
                      type="button"
                      class=btn.clone()
                      aria-current=is_current.then_some("page")
                      on:click=move |_| {
                        if !is_current {
                          on_change.run(p);
                        }
                      }
                    >
                      {p.to_string()}
                    </button>
                  }
                  .into_any()
                }
                PageItem::Ellipsis => {
                  view! {
                    <span class="px-1 text-sm text-muted-foreground" aria-hidden="true">{"…"}</span>
                  }
                  .into_any()
                }
              })
              .collect_view()}
            <button
              type="button"
              class=btn.clone()
              disabled=current >= total
              aria-label=move || t("common.next-page")
              on:click=move |_| {
                if current < total {
                  on_change.run(current + 1);
                }
              }
            >
              <Icon kind=IconKind::ChevronRight class="size-6" />
            </button>
          </div>
        </nav>
      }
      .into_any()
    }}
  }
}

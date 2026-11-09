use leptos::prelude::*;

use crate::i18n::tf;
use crate::ui::{Button, Variant};

/// 长列表底部的「加载更多（已显示 / 总数）」按钮。
///
/// `visible` 是页面里「列表放开到第几项」的信号，点一下多放开 `step` 项；`total` 是符合筛选的总数。
/// 已显示的数量按 `visible` 算、不按列表真正建出来的行数算：首帧只建了前几行（`after_paint::grow_cap`
/// 分批补齐）时，按钮也不该显示「8 / 1100」再跳成「40 / 1100」。
///
/// `total` 可以是固定值，也可以是会变的信号（如收藏页取消收藏后总数减一）：按钮自己读它，
/// 页面里包着 `<For>` 的闭包不必为它订阅。读 `visible` 同理 —— **那个闭包里不要读 `visible`**，
/// 否则每次「加载更多」整块重跑，键控就白做了。
///
/// `class` 是按钮自己的间距 / 宽度类，默认 `mt-4 w-full`（列表与按钮之间留 1rem）；
/// 列表容器本身已经带间距的页面（收藏页、错题本的像素风）传 `w-full`。
/// 像素风走 `ui::Button`；现代风用 `md-btn-outline`（带 surface 底，和列表里的卡片同一层），
/// 不是 `ui::Button` 在现代风下给的 `md-btn-line`（透明底）。
#[component]
pub fn LoadMore(
  visible: RwSignal<usize>,
  #[prop(into)] total: Signal<usize>,
  step: usize,
  #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
  let modern = crate::kit::is_modern_at_mount();
  let class = class.unwrap_or_else(|| "mt-4 w-full".to_owned());
  move || {
    let total = total.get();
    let shown = visible.get().min(total);
    (shown < total).then(|| {
      let label = move || {
        tf(
          "knowledge.load-more-count",
          &[&shown.to_string(), &total.to_string()],
        )
      };
      let more = move || visible.update(|v| *v += step);
      if modern {
        view! {
          <button
            type="button"
            class=format!("md-btn md-btn-outline {class}")
            on:click=move |_| more()
          >
            {label}
          </button>
        }
        .into_any()
      } else {
        view! {
          <Button
            variant=Variant::Outline
            class=class.clone()
            on_click=Callback::new(move |_| more())
          >
            {label}
          </Button>
        }
        .into_any()
      }
    })
  }
}

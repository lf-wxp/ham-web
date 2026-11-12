use std::sync::atomic::{AtomicU32, Ordering};

use leptos::prelude::*;

use super::super::popover;
use super::select_root::SelectCtx;

/// 下拉选项。
///
/// 行的 DOM id 全局唯一（键盘高亮靠 `aria-activedescendant` 指它）；是否高亮由根组件持有的
/// `active` 与自己的 id 比较得出，行本身不需要知道自己排第几。
///
/// 点击与键盘 `Enter` 走同一个处理函数：键盘选中是对高亮行触发一次 `click()`，
/// 所以「选中 → 回调 → 关闭」只写了这一遍。
#[component]
pub fn SelectItem(#[prop(into)] value: String, children: Children) -> impl IntoView {
  static SEQ: AtomicU32 = AtomicU32::new(0);

  let ctx = expect_context::<SelectCtx>();
  let id = format!("select-opt-{}", SEQ.fetch_add(1, Ordering::Relaxed));
  let my_id = StoredValue::new(id.clone());
  let v = StoredValue::new(value);
  let selected = move || {
    ctx
      .value
      .with(|cur| v.with_value(|v| cur.as_deref() == Some(v.as_str())))
  };
  // 自己是高亮行时被卸载（选项信号变化导致整批重建）：清掉指向自己的 id 并请根组件重新定位，
  // 否则 `aria-activedescendant` 会指向已不存在的元素。`try_*`：整个下拉一起卸载时信号可能已释放。
  on_cleanup(move || {
    let mine_active = ctx
      .active
      .try_with_untracked(|a| my_id.with_value(|mine| a.as_deref() == Some(mine.as_str())))
      .unwrap_or(false);
    if mine_active {
      ctx.active.try_set(None);
      ctx.reprime.try_update(|n| *n = n.wrapping_add(1));
    }
  });
  let highlighted = move || {
    ctx
      .active
      .with(|a| my_id.with_value(|mine| a.as_deref() == Some(mine.as_str())))
  };
  view! {
    <div
      role="option"
      id=id
      aria-selected=move || selected().to_string()
      data-state=move || if selected() { "checked" } else { "unchecked" }
      data-active=move || highlighted().to_string()
      tabindex="-1"
      class=popover::item_class()
      on:click=move |e| {
        popover::swallow(&e);
        ctx.on_change.run(v.get_value());
        ctx.open.set(false);
      }
    >
      <span class="w-full">{children()}</span>
    </div>
  }
}

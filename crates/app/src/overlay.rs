//! 浮层对页面的副作用：锁住背后页面的滚动。
//!
//! 两套 Kit 的弹层形状完全不同（像素窗 / 现代抽屉），但「打开时锁住背后页面、关掉后放开」
//! 是同一件事，而且**计数必须是同一份**：两个弹层叠开（目录 → 设置）时，后关的那个不能把
//! 前一个的锁一起解掉。

use std::cell::Cell;

use leptos::prelude::*;

thread_local! {
  /// 当前持有锁的浮层数。
  ///
  /// 窗口滚动条已被 `ui::ScrollBar` 接管（原生那条在 `style/pixel/surfaces.css` 里隐藏了），
  /// 所以这里不需要补 `padding-right` 抵消滚动条宽度 —— 隐藏前后布局宽度不变，页面不会横向跳。
  /// 这个前提写在 `surfaces.css` 的 `html` 规则里，改那边要回来一起看。
  static SCROLL_LOCKS: Cell<u32> = const { Cell::new(0) };
}

/// 改一次锁计数，并按计数写 `body` 的 `overflow`。
pub fn set_locked(on: bool) {
  let count = SCROLL_LOCKS.with(|c| {
    let n = if on {
      c.get() + 1
    } else {
      c.get().saturating_sub(1)
    };
    c.set(n);
    n
  });
  let Some(body) = crate::util::body() else {
    return;
  };
  let style = body.style();
  let _ = if count > 0 {
    style.set_property("overflow", "hidden")
  } else {
    style.remove_property("overflow").map(|_| ())
  };
}

/// 浮层打开期间锁滚动；**组件卸载时自动放开**。
///
/// 卸载时释放是必须的：风格切换会重建整棵路由子树（见 `app/main_content.rs`），
/// 浮层还开着就被卸载，不释放的话 `body { overflow: hidden }` 会留在页面上，
/// 整页从此滚不动 —— 用户看到的是「切了风格之后要刷新才能滚」。
pub fn use_scroll_lock(open: RwSignal<bool>) {
  // 记「本组件当前是否持有锁」而不是每次读 `open`：卸载时 `open` 可能仍是 `true`，
  // 只有这一位能回答「我到底该不该减回去」。
  let held = StoredValue::new(false);
  if open.get_untracked() {
    held.set_value(true);
    set_locked(true);
  }
  Effect::new(move |_| {
    let on = open.get();
    if on != held.get_value() {
      held.set_value(on);
      set_locked(on);
    }
  });
  on_cleanup(move || {
    if held.get_value() {
      held.set_value(false);
      set_locked(false);
    }
  });
}

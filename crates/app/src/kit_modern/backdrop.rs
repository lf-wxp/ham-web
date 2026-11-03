//! 弹层遮罩的「点空白处关闭」：只认**按下与抬起都落在遮罩本身**的点击。
//!
//! 为什么不在面板上 `stop_propagation`：那会把冒泡一起挡掉，路由挂在 `window` 上的全局 click
//! 拦截就收不到面板里 `<a href>` 的点击，站内链接退化成整页刷新（用户会看到一次白屏加载）。
//! 为什么还要记 `pointerdown`：在面板里拖选文字、松手落在遮罩上时，浏览器把 click 的目标
//! 算成两者的最近公共祖先 —— 恰好是遮罩，只看 click 目标会把弹层误关掉。

use leptos::prelude::*;

/// 一个遮罩的关闭手势。字段全是 `Copy`，可以直接进多个事件闭包。
#[derive(Clone, Copy)]
pub struct Backdrop {
  open: RwSignal<bool>,
  /// 最近一次 `pointerdown` 是否落在遮罩本身。
  armed: StoredValue<bool>,
}

impl Backdrop {
  /// 在弹层组件体里创建一次。
  pub fn new(open: RwSignal<bool>) -> Self {
    Self {
      open,
      armed: StoredValue::new(false),
    }
  }

  /// 挂到遮罩的 `on:pointerdown`。
  pub fn on_pointerdown(self) -> impl Fn(web_sys::PointerEvent) + 'static {
    move |e| {
      self
        .armed
        .try_set_value(on_backdrop_itself(e.target(), e.current_target()));
    }
  }

  /// 挂到遮罩的 `on:click`。
  pub fn on_click(self) -> impl Fn(web_sys::MouseEvent) + 'static {
    move |e| {
      let armed = self.armed.try_get_value().unwrap_or(false);
      self.armed.try_set_value(false);
      if armed && on_backdrop_itself(e.target(), e.current_target()) {
        self.open.set(false);
      }
    }
  }
}

/// 事件目标就是挂监听的那个元素（没有经过面板冒泡上来）。
fn on_backdrop_itself(
  target: Option<web_sys::EventTarget>,
  current: Option<web_sys::EventTarget>,
) -> bool {
  matches!((target, current), (Some(a), Some(b)) if a == b)
}

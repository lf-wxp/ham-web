//! 指示灯：电台面板上最小的状态件（`docs/ui/pixel.md` §2 硬规则 15）。外观在 `style/pixel/radio.css`。

use leptos::prelude::*;

use crate::cn::cn;

/// 指示灯：12px（`small` 为 8px）的方形灯，亮 / 灭 / 闪三态。
///
/// 纯装饰（`aria-hidden`）：灯旁边必须有文字，或者状态已经写进常驻的 `role="status"` ——
/// 颜色不能是唯一通道（`docs/ui/pixel.md` §2 硬规则 15）。
#[component]
pub fn Led(
  tone: LedTone,
  #[prop(into)] state: Signal<LedState>,
  #[prop(optional)] small: bool,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  view! {
    <span
      aria-hidden="true"
      class=cn(&["pxl-led", &class])
      data-tone=tone.as_str()
      data-size=small.then_some("sm")
      data-state=move || state.get().as_str()
    ></span>
  }
}

/// 指示灯的四个意思（`docs/ui/pixel.md` §2 硬规则 15），各自指向一个既有状态色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedTone {
  /// 绿：锁定 / 正确 / 开。
  Rx,
  /// 红：发射 / 错误 / 告急 / 在考。
  Tx,
  /// 琥珀：忙 / 提醒 / 已标记。
  Warn,
  /// 蓝：当前 / 信息。
  Info,
}

impl LedTone {
  const fn as_str(self) -> &'static str {
    match self {
      Self::Rx => "rx",
      Self::Tx => "tx",
      Self::Warn => "warn",
      Self::Info => "info",
    }
  }
}

/// 指示灯的状态。闪烁只给「正在进行」的事（加载、告急），降级时变常亮。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LedState {
  #[default]
  Off,
  On,
  Blink,
}

impl LedState {
  /// 布尔开关的常用写法：亮 / 灭。
  #[must_use]
  pub const fn lit(on: bool) -> Self {
    if on { Self::On } else { Self::Off }
  }

  const fn as_str(self) -> &'static str {
    match self {
      Self::Off => "off",
      Self::On => "on",
      Self::Blink => "blink",
    }
  }
}

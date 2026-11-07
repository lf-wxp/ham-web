//! 输入法组字门控的 DOM 侧：从事件里读出 `isComposing` 与时间戳，判定交给 core 的
//! [`CompositionGate`]（顺序规则与单元测试都在那边）。
//!
//! 只给**搜索框**这类「每个字符都触发一次整份重算」的输入用（`ui::Input` 的 `type="search"`、
//! `kit_modern::ModernInput` 的 `kind="search"`）：表单字段需要逐字回调（字数计数、`maxlength`、
//! 逐字校验），组字期间不派发会让它们滞后。

use ham_web_core::ime_gate::CompositionGate;
use leptos::prelude::*;

/// 一个输入框一份：必须在组件体里创建（状态挂在当前 `Owner` 上）。
#[derive(Clone, Copy)]
pub(crate) struct Composition(StoredValue<CompositionGate>);

impl Composition {
  pub(crate) fn new() -> Self {
    Self(StoredValue::new(CompositionGate::default()))
  }

  /// `input` 事件：返回该派发的值，`None` 表示这是组字中的中间串（或 Safari 尾随的重复），吞掉。
  pub(crate) fn on_input(self, e: &web_sys::Event) -> Option<String> {
    let value = event_target_value(e);
    self
      .0
      .try_update_value(|g| g.on_input(is_composing(e), &value, e.time_stamp()))
      .flatten()
  }

  /// `compositionend` 事件：返回要补派的值（选字确认后的完整内容）。
  pub(crate) fn on_end(self, e: &web_sys::Event) -> Option<String> {
    let value = event_target_value(e);
    self
      .0
      .try_update_value(|g| g.on_end(&value, e.time_stamp()))
  }
}

/// 这次 `input` 是否发生在输入法组字过程中（`InputEvent.isComposing`）。
///
/// 读属性而不是转成 `InputEvent`：合成事件（e2e 的 `dispatchEvent(new Event("input"))`、
/// 部分浏览器的自动填充）不是 `InputEvent`，没有这个属性时按「不在组字」处理。
fn is_composing(e: &web_sys::Event) -> bool {
  js_sys::Reflect::get(e, &"isComposing".into())
    .ok()
    .and_then(|v| v.as_bool())
    .unwrap_or(false)
}

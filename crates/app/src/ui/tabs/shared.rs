//! tabs 一族共用的组上下文。

use leptos::prelude::*;

/// [`Tabs`](super::Tabs) 与其 trigger / panel 之间的上下文。
///
/// 组里只有一份事实来源（当前激活值），trigger 的 `aria-selected` 与 panel 的
/// 显隐都由它派生 —— 视觉、无障碍属性、面板显隐不会各说各话。
///
/// `id_base` 是 `util::unique_id` 生成的 `id` 前缀：trigger 拼 `-tab-<value>`、
/// panel 拼 `-panel-<value>`，`aria-controls` ↔ `aria-labelledby` 靠这对 id 关联。
/// 一页里可以有多个 `Tabs`，前缀不同才不会撞 id。
#[derive(Clone, Copy)]
pub(super) struct TabsCtx {
  pub(super) value: Signal<String>,
  pub(super) on_change: Callback<String>,
  pub(super) id_base: StoredValue<String>,
}

impl TabsCtx {
  /// 当前 `value` 是否是这一项。
  pub(super) fn is_active(&self, value: &StoredValue<String>) -> bool {
    self
      .value
      .with(|cur| value.with_value(|v| cur.as_str() == v.as_str()))
  }

  /// trigger 的 `id`（也是 panel 的 `aria-labelledby`）。
  pub(super) fn tab_id(&self, value: &str) -> String {
    check_slug(value);
    self
      .id_base
      .with_value(|base| format!("{base}-tab-{value}"))
  }

  /// panel 的 `id`（也是 trigger 的 `aria-controls`）。
  pub(super) fn panel_id(&self, value: &str) -> String {
    check_slug(value);
    self
      .id_base
      .with_value(|base| format!("{base}-panel-{value}"))
  }
}

/// `value` 必须是 ascii slug：它直接拼进 `id`，含空格 / 非法字符会生成无效 HTML id，
/// 导致 `aria-controls` / `aria-labelledby` 断链。这是组件 API 契约，违规直接 panic。
fn check_slug(value: &str) {
  assert!(
    !value.is_empty()
      && value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
    // `[tabs]` 前缀是「开发者诊断消息」约定：check-i18n 对 `[` 开头的字面量不做译文要求。
    "[tabs] value 必须是 ascii slug（[A-Za-z0-9_-]+），收到 {value:?}"
  );
}

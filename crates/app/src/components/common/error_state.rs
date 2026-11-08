//! 错误态：一次操作失败了，告诉用户「什么失败了 / 为什么 / 能不能再来一次」。
//!
//! 与 [`EmptyState`](super::EmptyState) 分开：空态是「这里确实没有内容」（虚线圈 + 安静的文字），
//! 错误态需要一个一眼能认出的「出事了」形态。两者的共同点只有「都在卡片里占一块位置」。
//!
//! 为什么要有这个组件：解码页那 10 处 `role="alert"` + `border-destructive/40` 是逐字复制的，
//! 改一次颜色要改 10 个文件，且只有像素风的取值（`text-hp-text` 是血量色）——
//! 现代风里靠 `legacy-utilities.css` 的变量映射兜底，误差在两套皮肤之间越拉越大
//! （见 `docs/ui/modern-review.md` 的 S3 / T2）。
//!
//! 形制：
//! - 现代风 `.md-error`：行动色淡底 + `--md-danger-ink` 文字，`detail` 用等宽小字（原文 / 错误码）；
//! - 像素风沿用原来的 `border-destructive/40 bg-destructive/5 text-hp-text` 方块。

use leptos::prelude::*;

use crate::icons::{Icon, IconKind};
use crate::kit_modern::{MIcon, ModernIcon};
use crate::ui::{Button, Size, TextValue, Variant};

/// 失败提示。`on_retry` 只在**重试是有效的**时候传（解码失败通常要重新选文件，就不传）。
#[component]
pub fn ErrorState(
  #[prop(into)] title: TextValue,
  /// 具体原因：后端报错、worker 错误码、文件名……不传只显示标题。
  #[prop(optional, into)]
  detail: Option<TextValue>,
  #[prop(optional)] on_retry: Option<Callback<()>>,
  /// 重试按钮文案（默认「重试」，与 `exam.retry` 同一个词条）。
  #[prop(optional, into)]
  retry_label: Option<TextValue>,
) -> impl IntoView {
  let modern = crate::kit::is_modern_at_mount();
  // 默认文案走信号：`t()` 在组件体里只求值一次，切语言后按钮会停在旧语言
  // （周围文案都是 `move || t(…)`，唯独这一颗按钮不变会很显眼）。
  let retry_label = retry_label
    .unwrap_or_else(|| TextValue::from(Signal::derive(move || crate::i18n::t("exam.retry"))));

  let retry = on_retry.map(|on_retry| {
    view! {
      // `text-xs`：像素风 `.pxl-btn` 默认 `--text-sm`，错误块里要更小一档；现代风 `.md-btn-sm`
      // 已是 `--md-text-xs`，`text-xs` 对它是等宽替换，两套皮肤视觉都不变。
      <Button variant=Variant::Outline size=Size::Sm class="text-xs" on_click=on_retry>
        {move || retry_label.get()}
      </Button>
    }
  });

  if modern {
    return view! {
      <div role="alert" class="md-error">
        <ModernIcon kind=MIcon::Alert class="md-error-icon" />
        <div class="md-error-body">
          <div class="md-error-title">{move || title.get()}</div>
          {detail.map(|d| view! { <div class="md-error-detail">{move || d.get()}</div> })}
        </div>
        {retry}
      </div>
    }
    .into_any();
  }

  view! {
    <div
      role="alert"
      class="flex items-start gap-2 border border-destructive/40 bg-destructive/5 p-4 text-hp-text"
    >
      <Icon kind=IconKind::AlertCircle class="mt-0.5 size-6 shrink-0" />
      <div class="flex-1">
        <div>{move || title.get()}</div>
        {detail.map(|d| view! { <div class="mt-1 text-sm">{move || d.get()}</div> })}
      </div>
      {retry}
    </div>
  }
  .into_any()
}

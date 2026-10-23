//! 首屏加载骨架：页头 + 内容块的轮廓先到位，再换成真实内容。
//!
//! 为什么不直接用一个 spinner：题库类页面的首屏是「一大块题干 / 表格」，转圈消失的瞬间
//! 高度会跳一次；骨架把最终版式先占住，视觉上更稳。骨架本身对读屏静音（`aria-hidden`），
//! 状态由外层 `role="status"` + `aria-busy` + `sr-only` 文案播报。
//!
//! 与 [`crate::components::common::Loading`] 的分工：整页首屏用这里；页内局部等待
//! （切换筛选、提交表单）仍用 `Loading`。

use leptos::prelude::*;

use crate::components::common::{PageContainer, Skeleton};
use crate::ui::TextValue;

/// 整页加载态：`label` 是无障碍播报文案（屏幕上有骨架，没有可见文字）。
#[component]
pub fn PageLoading(#[prop(into)] label: TextValue) -> impl IntoView {
  view! {
    <PageContainer>
      <div role="status" aria-busy="true" aria-live="polite" class="space-y-6">
        <span class="sr-only">{move || label.get()}</span>
        // 轮廓按「页头标题行 + 题干块 + 解析块」排，和加载完成后的高度大体对齐。
        <div class="space-y-3">
          <Skeleton class="h-6 w-24" />
          <Skeleton class="h-4 w-2/3" />
        </div>
        <Skeleton class="h-44 w-full" />
        <Skeleton class="h-16 w-full" />
      </div>
    </PageContainer>
  }
}

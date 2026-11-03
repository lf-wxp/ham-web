//! 页面加载骨架（现代风）：题卡形状的灰块 + 微光扫过，替代像素风的加载窗体。
//!
//! 骨架的形状要贴近真实内容（标题行 + 题干 + 4 个选项），切换时视觉不会跳。

use leptos::prelude::*;

#[component]
pub fn ModernLoading(#[prop(into)] label: Signal<String>) -> impl IntoView {
  view! {
    <div class="md-root">
      <div class="md-page md-skel-page">
        <div class="md-skel md-skel-line" style="width: 22%; height: 12px"></div>
        <div class="md-skel md-skel-card">
          <div class="md-skel md-skel-line" style="width: 34%; height: 14px"></div>
          <div class="md-skel md-skel-line" style="width: 82%; height: 18px"></div>
          <div class="md-skel md-skel-line" style="width: 68%; height: 18px"></div>
          <div class="md-skel-rows">
            {(0..4)
              .map(|_| view! { <div class="md-skel md-skel-option"></div> })
              .collect_view()}
          </div>
        </div>
      </div>
      <span class="sr-only">{move || label.get()}</span>
    </div>
  }
}

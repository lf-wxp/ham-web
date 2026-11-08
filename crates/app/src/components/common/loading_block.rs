//! 卡片内的加载占位：一句「加载中」换成与实际内容同高的灰块。
//!
//! 为什么需要：几十张「等外部数据」的卡片在加载时只印一行「加载中」，数据到位后那一行变成
//! 三四行内容，卡片跟着长高一截，整页向下顶（台账 S1）。占位块的高度贴近真实内容
//! （两行短文本），到位时行数一致，位置不跳。
//!
//! 两套皮肤各一份：
//! - 现代风：`.md-skel-block`（两行灰块 + 扫光，与 `.md-skel` 同一套），文案只给读屏；
//! - 像素风：保留原来那行 `text-sm text-muted-foreground` 文字 —— 像素风版式冻结，
//!   而且它的加载文案本来就是一句人话（点阵字体下灰块反而看不出是加载）。

use leptos::prelude::*;

use crate::ui::TextValue;

/// 卡片内的加载占位。
#[component]
pub fn LoadingBlock(
  #[prop(into)] label: TextValue,
  /// 大块内容（地图 / 图表）：渲染一整块灰面，而不是两行短灰块。
  #[prop(default = false)]
  tall: bool,
) -> impl IntoView {
  if crate::kit::is_modern_at_mount() {
    return view! {
      <div class="md-skel-block" role="status" aria-busy="true">
        {if tall {
          view! { <div class="md-skel md-skel-fill"></div> }.into_any()
        } else {
          view! {
            <div class="md-skel md-skel-line" style="width: 62%"></div>
            <div class="md-skel md-skel-line" style="width: 38%"></div>
          }
            .into_any()
        }}
        <span class="sr-only">{move || label.get()}</span>
      </div>
    }
    .into_any();
  }
  view! {
    // 两套皮肤都挂 `role="status"`（隐含 polite）：现代风只有 `sr-only` 文案、
    // 像素风把文字显示出来，读屏用户拿到的播报因此一致。
    <div class="text-sm text-muted-foreground" role="status" aria-busy="true">
      {move || label.get()}
    </div>
  }
  .into_any()
}

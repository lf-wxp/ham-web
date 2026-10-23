//! 解码进行中的「已用时」读数。
//!
//! 解码 Worker 是一次性的同步计算，协议里没有中间进度（见
//! `docs/decode-worker-protocol.md`），所以页面给不出百分比；能给的、也是用户最想知道的
//! 是「它还在动 + 已经跑了多久」。数值来自 `DecodeSession::elapsed_ms`（200ms 一跳）。
//!
//! 走 `pxl-label`（Silkscreen）+ 等宽数字：读数是数字，不该和旁边的说明文字混在一起。

use leptos::prelude::*;

/// 解码已用时，形如 `3.2s`。
#[component]
pub fn DecodeElapsed(#[prop(into)] ms: Signal<u64>) -> impl IntoView {
  view! {
    <span class="pxl-label text-xs tabular-nums text-muted-foreground">
      {move || format!("{:.1}s", ms.get() as f64 / 1000.0)}
    </span>
  }
}

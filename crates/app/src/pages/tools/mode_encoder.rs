use ham_web_core::mode_encoder::{text_to_baudot_bits, text_to_morse};
use leptos::prelude::*;

use super::INPUT;
use crate::i18n::t;

/// 数字模式报文编码：文本 → 摩尔斯（CW）与 RTTY（ITA2）比特流。
#[component]
pub(super) fn ModeEncoder() -> impl IntoView {
  let text = RwSignal::new("CQ CQ DE BG1AAA".to_string());

  view! {
    <div class="space-y-4">
      <label class="flex flex-col gap-1.5 text-sm">
        <span class="text-xs text-muted-foreground">{move || t("报文内容")}</span>
        <input
          type="text"
          prop:value=move || text.get()
          on:input=move |e| text.set(event_target_value(&e))
          class=INPUT
        />
      </label>
      <div class="space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        <div class="text-xs font-medium text-foreground/70">{move || t("CW 摩尔斯电码")}</div>
        <div class="break-all font-mono tabular-nums">{move || text_to_morse(&text.get())}</div>
      </div>
      <div class="space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        <div class="text-xs font-medium text-foreground/70">{move || t("RTTY（ITA2）比特流")}</div>
        <div class="break-all font-mono tabular-nums">{move || text_to_baudot_bits(&text.get())}</div>
      </div>
      <p class="text-xs text-muted-foreground">
        {move || t("RTTY 编码：11111 为字母移态（LTRS）、11011 为数字移态（FIGS）。CW 中 `/` 表示单词间隔。")}
      </p>
    </div>
  }
}

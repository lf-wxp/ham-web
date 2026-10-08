use ham_web_core::mode_encoder::{text_to_baudot_bits, text_to_morse};
use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{Field, Input};
use crate::util::unique_id;

/// 数字模式报文编码：文本 → 摩尔斯（CW）与 RTTY（ITA2）比特流。
#[component]
pub(super) fn ModeEncoder() -> impl IntoView {
  let text = RwSignal::new("CQ CQ DE BG1AAA".to_string());
  let text_id = unique_id("mode-encoder");

  view! {
    <div class="space-y-4">
      <Field label=Signal::derive(move || t("tools.message-content")) r#for=text_id.clone()>
        <Input id=text_id value=text on_change=Callback::new(move |v: String| text.set(v)) />
      </Field>
      <div class="space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        <div class="text-xs font-medium text-foreground/70">{move || t("tools.cw-morse-code")}</div>
        <div class="break-all font-mono tabular-nums">{move || text_to_morse(&text.get())}</div>
      </div>
      <div class="space-y-1 rounded-lg bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
        <div class="text-xs font-medium text-foreground/70">{move || t("tools.rtty-ita2-bit-stream")}</div>
        <div class="break-all font-mono tabular-nums">{move || text_to_baudot_bits(&text.get())}</div>
      </div>
      <p class="text-xs text-muted-foreground">
        {move || t("tools.rtty-coding-11111-is")}
      </p>
    </div>
  }
}

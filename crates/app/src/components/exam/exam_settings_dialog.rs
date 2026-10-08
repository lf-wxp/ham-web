use leptos::prelude::*;

use super::shortcut_row::ShortcutRow;
use crate::i18n::t;
use crate::shortcuts::SHORTCUT_HELP;
use crate::ui::{Checkbox, Dialog, DialogDescription, DialogHeader, DialogTitle, Label, Separator};

#[component]
pub fn ExamSettingsDialog(
  open: RwSignal<bool>,
  #[prop(into)] show_explanation: Signal<bool>,
  on_change_show_explanation: Callback<bool>,
  #[prop(into)] weighted: Signal<bool>,
  on_change_weighted: Callback<bool>,
  #[prop(into)] strict: Signal<bool>,
  on_change_strict: Callback<bool>,
) -> impl IntoView {
  view! {
    <Dialog open=open class="sm:max-w-[520px]">
      <DialogHeader>
        <DialogTitle>{move || t("exam.settings")}</DialogTitle>
        <DialogDescription>{move || t("exam.question-selection-shortcuts-and")}</DialogDescription>
      </DialogHeader>
      <div class="space-y-5">
        <div class="flex items-center gap-2">
          <Checkbox id="exam-show-expl" checked=show_explanation on_change=on_change_show_explanation />
          <Label r#for="exam-show-expl">{move || t("exam.show-explanations-after-submitting")}</Label>
        </div>
        <div class="flex items-start gap-2">
          <Checkbox id="exam-weighted" checked=weighted on_change=on_change_weighted />
          <Label r#for="exam-weighted">
            {move || t("exam.draw-questions-by-topic")}
          </Label>
        </div>
        <div class="flex items-start gap-2">
          <Checkbox id="exam-strict" checked=strict on_change=on_change_strict />
          <Label r#for="exam-strict">
            {move || t("exam.exam-simulation-full-screen")}
          </Label>
        </div>
        <div class="space-y-2 text-sm">
          <div class="text-muted-foreground">{move || t("exam.shortcuts")}</div>
          // 键位统一取自 SHORTCUT_HELP，避免这里与帮助面板各写一份而逐渐脱节。
          {SHORTCUT_HELP
            .iter()
            .map(|(keys, desc)| {
              view! { <ShortcutRow label=Signal::derive(move || t(desc)) keys=(*keys).to_owned() /> }
            })
            .collect_view()}
        </div>
        <Separator />
        <div class="text-xs text-muted-foreground">
          {move || {
            t("exam.exam-rules-class-a")
          }}
        </div>
      </div>
    </Dialog>
  }
}

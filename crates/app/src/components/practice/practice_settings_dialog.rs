use ham_web_core::practice::PracticeOrder;
use leptos::prelude::*;

use crate::components::exam::ShortcutRow;
use crate::i18n::t;
use crate::shortcuts::SHORTCUT_HELP;
use crate::ui::{
  Checkbox, Dialog, DialogDescription, DialogHeader, DialogTitle, Label, RadioGroup,
  RadioGroupItem, Separator, Switch,
};

#[component]
pub fn PracticeSettingsDialog(
  open: RwSignal<bool>,
  #[prop(into)] order: Signal<PracticeOrder>,
  on_change_order: Callback<PracticeOrder>,
  #[prop(into)] show_answer: Signal<bool>,
  on_toggle_show_answer: Callback<bool>,
  #[prop(into)] show_explanation: Signal<bool>,
  on_toggle_show_explanation: Callback<bool>,
  /// 计算变体：同型题换数重新生成（防背答案），只在当次练习里出现。
  #[prop(into)]
  variants: Signal<bool>,
  on_toggle_variants: Callback<bool>,
) -> impl IntoView {
  let order_value = Signal::derive(move || order.get().as_str().to_owned());
  let on_order = Callback::new(move |v: String| {
    if let Some(o) = PracticeOrder::parse(&v) {
      on_change_order.run(o);
    }
  });
  view! {
    <Dialog open=open class="sm:max-w-[520px]">
      <DialogHeader>
        <DialogTitle>{move || t("exam.settings")}</DialogTitle>
        <DialogDescription>{move || t("exam.order-answer-display-and")}</DialogDescription>
      </DialogHeader>
      <div class="space-y-5">
        <div class="space-y-2">
          <div class="text-sm text-muted-foreground">{move || t("exam.order-random")}</div>
          <RadioGroup class="flex items-center gap-4" value=order_value on_change=on_order>
            <div class="flex items-center space-x-2">
              <RadioGroupItem value="sequential" id="order-seq" />
              <Label r#for="order-seq">{move || t("exam.sequential")}</Label>
            </div>
            <div class="flex items-center space-x-2">
              <RadioGroupItem value="random" id="order-rand" />
              <Label r#for="order-rand">{move || t("exam.random")}</Label>
            </div>
          </RadioGroup>
        </div>
        <div class="flex items-center gap-2">
          <Checkbox id="show-ans" checked=show_answer on_change=on_toggle_show_answer />
          <Label r#for="show-ans">{move || t("exam.show-correct-answer")}</Label>
        </div>
        <div class="flex items-center gap-2">
          <Checkbox id="show-expl" checked=show_explanation on_change=on_toggle_show_explanation />
          <Label r#for="show-expl">{move || t("exam.show-explanation")}</Label>
        </div>
        <div class="flex items-start justify-between gap-3">
          <div>
            <div class="text-sm">{move || t("exam.calc-variants")}</div>
            <p class="mt-0.5 text-xs text-muted-foreground">{move || t("exam.calc-variants-hint")}</p>
          </div>
          <Switch
            checked=variants
            on_change=on_toggle_variants
            aria_label=Signal::derive(move || t("exam.calc-variants"))
          />
        </div>
        <Separator />
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
      </div>
    </Dialog>
  }
}

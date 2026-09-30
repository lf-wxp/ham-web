use ham_web_core::practice::PracticeOrder;
use leptos::prelude::*;

use crate::components::exam::ShortcutRow;
use crate::ui::{
  Checkbox, Dialog, DialogDescription, DialogHeader, DialogTitle, Label, RadioGroup,
  RadioGroupItem, Separator,
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
        <DialogTitle>"设置"</DialogTitle>
        <DialogDescription>"题序、显示答案与快捷键说明"</DialogDescription>
      </DialogHeader>
      <div class="space-y-5">
        <div class="space-y-2">
          <div class="text-sm text-muted-foreground">"顺序/随机"</div>
          <RadioGroup class="flex items-center gap-4" value=order_value on_change=on_order>
            <div class="flex items-center space-x-2">
              <RadioGroupItem value="sequential" id="order-seq" />
              <Label r#for="order-seq">"顺序"</Label>
            </div>
            <div class="flex items-center space-x-2">
              <RadioGroupItem value="random" id="order-rand" />
              <Label r#for="order-rand">"随机"</Label>
            </div>
          </RadioGroup>
        </div>
        <div class="flex items-center gap-2">
          <Checkbox id="show-ans" checked=show_answer on_change=on_toggle_show_answer />
          <Label r#for="show-ans">"显示正确答案"</Label>
        </div>
        <div class="flex items-center gap-2">
          <Checkbox id="show-expl" checked=show_explanation on_change=on_toggle_show_explanation />
          <Label r#for="show-expl">"显示答案解析"</Label>
        </div>
        <Separator />
        <div class="space-y-2 text-sm">
          <div class="text-muted-foreground">"快捷键"</div>
          <ShortcutRow label="上一题 / 下一题" keys="← / →" />
          <ShortcutRow label="选择 / 切换选项（单选/多选）" keys="1-9" />
          <ShortcutRow label="严格选择（多选，仅该项）" keys="Shift 或 Cmd（macOS） + 1-9" />
          <ShortcutRow label="打开搜索（仅顺序模式）" keys="Enter" />
        </div>
      </div>
    </Dialog>
  }
}

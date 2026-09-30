use leptos::prelude::*;

use crate::ui::{
  Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Size, Variant, button_class,
};

#[component]
pub fn ExamSubmitConfirmDialog(
  open: RwSignal<bool>,
  on_confirm: Callback<()>,
  #[prop(into)] total: Signal<usize>,
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] flagged: Signal<usize>,
) -> impl IntoView {
  let unanswered = move || total.get().saturating_sub(answered.get());
  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>"确认交卷？"</DialogTitle>
        <DialogDescription>"交卷后将停止计时，答案将不可修改，但可以浏览查看正确答案与成绩。"</DialogDescription>
      </DialogHeader>
      <div class="space-y-2 text-sm">
        <div class="flex items-center justify-between">
          <span>"已作答"</span>
          <span>{move || answered.get()} " / " {move || total.get()}</span>
        </div>
        <div class="flex items-center justify-between">
          <span>"未作答"</span>
          <span class=move || (unanswered() > 0).then_some("text-red-600 dark:text-red-400")>{unanswered}</span>
        </div>
        <div class="flex items-center justify-between">
          <span>"已标记"</span>
          <span>{move || flagged.get()}</span>
        </div>
      </div>
      <DialogFooter>
        <button class=button_class(Variant::Outline, Size::Default, "") on:click=move |_| open.set(false)>
          "取消"
        </button>
        <button class=button_class(Variant::Destructive, Size::Default, "") on:click=move |_| on_confirm.run(())>
          "确认交卷"
        </button>
      </DialogFooter>
    </Dialog>
  }
}

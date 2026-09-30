use leptos::prelude::*;

use crate::ui::{
  Dialog, DialogDescription, DialogHeader, DialogTitle, Size, Variant, button_class,
};

#[component]
pub fn ExamResumeDialog(
  open: RwSignal<bool>,
  #[prop(into)] expires_in_ms: Signal<i64>,
  #[prop(into)] answered: Signal<usize>,
  #[prop(into)] total: Signal<usize>,
  on_resume: Callback<()>,
  on_restart: Callback<()>,
) -> impl IntoView {
  let remaining = move || {
    let ms = expires_in_ms.get().max(0);
    format!("{:02}:{:02}", ms / 60_000, (ms % 60_000) / 1000)
  };
  view! {
    <Dialog open=open class="sm:max-w-[520px]">
      <DialogHeader>
        <DialogTitle>"恢复考试"</DialogTitle>
        <DialogDescription>
          "检测到未完成的考试。已答 " {move || answered.get()} " / " {move || total.get()} "，剩余时间约 " {remaining}
          "。"
        </DialogDescription>
      </DialogHeader>
      <div class="flex items-center justify-end gap-2 pt-2">
        <button class=button_class(Variant::Outline, Size::Default, "") on:click=move |_| on_restart.run(())>
          "重新开始"
        </button>
        <button class=button_class(Variant::Default, Size::Default, "") on:click=move |_| on_resume.run(())>
          "继续考试"
        </button>
      </div>
    </Dialog>
  }
}

use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Button, Dialog, DialogDescription, DialogHeader, DialogTitle, Size, Variant};

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
        <DialogTitle>{move || t("exam.resume-exam")}</DialogTitle>
        <DialogDescription>
          {move || {
            tf(
              "exam.found-an-unfinished-exam",
              &[&answered.get().to_string(), &total.get().to_string(), &remaining()],
            )
          }}
        </DialogDescription>
      </DialogHeader>
      <div class="flex items-center justify-end gap-2 pt-2">
        <Button
          variant=Variant::Outline
          size=Size::Default
          on_click=Callback::new(move |_| on_restart.run(()))
        >
          {move || t("exam.restart")}
        </Button>
        <Button
          variant=Variant::Default
          size=Size::Default
          on_click=Callback::new(move |_| on_resume.run(()))
        >
          {move || t("exam.resume-2")}
        </Button>
      </div>
    </Dialog>
  }
}

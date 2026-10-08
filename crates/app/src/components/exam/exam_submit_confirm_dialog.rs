use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{
  Button, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Size, Variant,
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
        <DialogTitle>{move || t("exam.submit-now")}</DialogTitle>
        <DialogDescription>{move || t("exam.submitting-stops-the-timer")}</DialogDescription>
      </DialogHeader>
      <div class="space-y-2 text-sm">
        <div class="flex items-center justify-between">
          <span>{move || t("exam.answered")}</span>
          <span>{move || answered.get()} " / " {move || total.get()}</span>
        </div>
        <div class="flex items-center justify-between">
          <span>{move || t("exam.unanswered")}</span>
          <span class=move || (unanswered() > 0).then_some("text-red-600 dark:text-red-400")>{unanswered}</span>
        </div>
        <div class="flex items-center justify-between">
          <span>{move || t("exam.flagged")}</span>
          <span>{move || flagged.get()}</span>
        </div>
      </div>
      <DialogFooter>
        <Button
          variant=Variant::Outline
          size=Size::Default
          on_click=Callback::new(move |_| open.set(false))
        >
          {move || t("exam.cancel")}
        </Button>
        <Button
          variant=Variant::Destructive
          size=Size::Default
          on_click=Callback::new(move |_| on_confirm.run(()))
        >
          {move || t("exam.submit-2")}
        </Button>
      </DialogFooter>
    </Dialog>
  }
}

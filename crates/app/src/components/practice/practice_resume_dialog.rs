use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{
  Button, Checkbox, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Label,
  Size, Variant,
};

#[component]
pub fn PracticeResumeDialog(
  open: RwSignal<bool>,
  no_prompt: RwSignal<bool>,
  on_restart: Callback<()>,
  on_resume: Callback<()>,
) -> impl IntoView {
  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>{move || t("exam.found-a-previous-session")}</DialogTitle>
        <DialogDescription>{move || t("exam.resume-where-you-left")}</DialogDescription>
      </DialogHeader>
      <div class="py-2">
        <div class="flex items-center gap-2">
          <Checkbox
            id="no-prompt-this-bank"
            checked=no_prompt
            on_change=Callback::new(move |v| no_prompt.set(v))
          />
          <Label r#for="no-prompt-this-bank">{move || t("exam.don-t-ask-again")}</Label>
        </div>
      </div>
      <DialogFooter>
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
          {move || t("exam.resume")}
        </Button>
      </DialogFooter>
    </Dialog>
  }
}

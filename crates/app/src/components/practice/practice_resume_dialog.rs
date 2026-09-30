use leptos::prelude::*;

use crate::ui::{
  Checkbox, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Label, Size,
  Variant, button_class,
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
        <DialogTitle>"发现上次练习记录"</DialogTitle>
        <DialogDescription>"是否加载到上次练习的位置，还是重新开始？"</DialogDescription>
      </DialogHeader>
      <div class="py-2">
        <div class="flex items-center gap-2">
          <Checkbox
            id="no-prompt-this-bank"
            checked=no_prompt
            on_change=Callback::new(move |v| no_prompt.set(v))
          />
          <Label r#for="no-prompt-this-bank">"本题库不再提示"</Label>
        </div>
      </div>
      <DialogFooter>
        <button class=button_class(Variant::Outline, Size::Default, "") on:click=move |_| on_restart.run(())>
          "重新开始"
        </button>
        <button class=button_class(Variant::Default, Size::Default, "") on:click=move |_| on_resume.run(())>
          "继续上次"
        </button>
      </DialogFooter>
    </Dialog>
  }
}

use leptos::prelude::*;

use crate::i18n::t;
use crate::ui::{
  Button, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Size, Variant,
};

/// 单按钮提示弹窗。
#[component]
pub fn MessageDialog(
  open: RwSignal<bool>,
  #[prop(into, default = "提示".into())] title: String,
  #[prop(into)] description: Signal<String>,
  #[prop(into, default = "确定".into())] confirm_text: String,
) -> impl IntoView {
  let title = StoredValue::new(title);
  let confirm_text = StoredValue::new(confirm_text);
  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>{move || t(&title.get_value())}</DialogTitle>
        {move || {
          let d = description.get();
          if d.is_empty() {
            view! { <DialogDescription class="sr-only">{move || t("exam.dialog-notice")}</DialogDescription> }.into_any()
          } else {
            view! { <DialogDescription>{d}</DialogDescription> }.into_any()
          }
        }}
      </DialogHeader>
      <DialogFooter>
        <Button
          variant=Variant::Default
          size=Size::Default
          on_click=Callback::new(move |_| open.set(false))
        >
          {move || t(&confirm_text.get_value())}
        </Button>
      </DialogFooter>
    </Dialog>
  }
}

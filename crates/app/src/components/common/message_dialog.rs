use leptos::prelude::*;

use crate::ui::{
  Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Size, Variant, button_class,
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
        <DialogTitle>{title.get_value()}</DialogTitle>
        {move || {
          let d = description.get();
          if d.is_empty() {
            view! { <DialogDescription class="sr-only">"弹窗提示"</DialogDescription> }.into_any()
          } else {
            view! { <DialogDescription>{d}</DialogDescription> }.into_any()
          }
        }}
      </DialogHeader>
      <DialogFooter>
        <button class=button_class(Variant::Default, Size::Default, "") on:click=move |_| open.set(false)>
          {confirm_text.get_value()}
        </button>
      </DialogFooter>
    </Dialog>
  }
}

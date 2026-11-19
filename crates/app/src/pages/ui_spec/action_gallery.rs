//! 动作类组件：链接按钮、文件选择 / 拖放、全局确认框、Toast 与消息弹层。

use leptos::prelude::*;

use crate::components::common::{MessageDialog, SectionCard};
use crate::i18n::t;
use crate::icons::IconKind;
use crate::ui::{
  Button, ButtonLink, ConfirmTone, FileDropZone, FileInput, Separator, ToastKind, Variant, confirm,
  push, push_success,
};

#[component]
pub fn ActionGallery() -> impl IntoView {
  let picked = RwSignal::new(String::new());
  let dropped = RwSignal::new(String::new());
  let message = RwSignal::new(false);
  let last = RwSignal::new(String::new());
  let picked_text = move || {
    let name = picked.get();
    if name.is_empty() {
      t("design.comp-file-none")
    } else {
      name
    }
  };

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-actions"))>
      <div class="space-y-4 p-4">
        <p class="text-xs leading-relaxed text-muted-foreground">
          {move || t("design.comp-actions-hint")}
        </p>
        <div class="flex flex-wrap items-center gap-2">
          <ButtonLink
            href="/ui-spec"
            on_click=Callback::new(move |ev: web_sys::MouseEvent| {
              ev.prevent_default();
              last.set(String::from("ButtonLink"));
            })
          >
            {move || t("design.comp-link")}
          </ButtonLink>
          <ButtonLink
            href="/ui-spec"
            variant=Variant::Outline
            led=crate::ui::LedTone::Info
            led_on=Signal::derive(|| true)
            on_click=Callback::new(move |ev: web_sys::MouseEvent| {
              ev.prevent_default();
              last.set(String::from("ButtonLink + Led"));
            })
          >
            {move || t("design.comp-link-led")}
          </ButtonLink>
          <Button
            variant=Variant::Secondary
            on_click=Callback::new(move |_| {
              confirm(
                t("design.comp-confirm-title"),
                t("design.comp-confirm-desc"),
                t("design.comp-confirm-ok"),
                ConfirmTone::Danger,
                Callback::new(move |_| push_success(t("design.comp-confirm-done"))),
              );
            })
          >
            {move || t("design.comp-confirm-danger")}
          </Button>
          <Button
            variant=Variant::Ghost
            on_click=Callback::new(move |_| {
              confirm(
                t("design.comp-confirm-title"),
                t("design.comp-confirm-desc"),
                t("design.comp-confirm-ok"),
                ConfirmTone::Normal,
                Callback::new(move |_| push_success(t("design.comp-confirm-done"))),
              );
            })
          >
            {move || t("design.comp-confirm-normal")}
          </Button>
          <Button
            variant=Variant::Text
            on_click=Callback::new(move |_| push(ToastKind::Info, t("design.comp-toast-neutral")))
          >
            {move || t("design.comp-toast-neutral")}
          </Button>
          <Button
            variant=Variant::Outline
            on_click=Callback::new(move |_| message.set(true))
          >
            {move || t("design.comp-message-dialog")}
          </Button>
        </div>

        <Separator />

        <div class="grid gap-3 md:grid-cols-2">
          <div class=crate::kit::tile_class("space-y-2 p-3")>
            <FileInput
              accept=".json"
              label=Signal::derive(move || t("design.comp-file-input"))
              variant=Variant::Outline
              on_files=Callback::new(move |files: Vec<web_sys::File>| {
                if let Some(file) = files.into_iter().next() {
                  picked.set(file.name());
                }
              })
            />
            <code class="block text-xs break-all text-muted-foreground">{picked_text}</code>
          </div>
          <div class=crate::kit::tile_class("space-y-2 p-3")>
            <FileDropZone
              on_file=Callback::new(move |file: web_sys::File| dropped.set(file.name()))
              accept=".json"
              title=Signal::derive(move || t("design.comp-file-drop-title"))
              icon=IconKind::FileText
              hint=Signal::derive(move || t("design.comp-file-drop-hint"))
            />
            <code class="block text-xs break-all text-muted-foreground">
              {move || {
                let name = dropped.get();
                if name.is_empty() { t("design.comp-file-none") } else { name }
              }}
            </code>
          </div>
        </div>

        <div class=crate::kit::tile_class("space-y-2 p-3")>
          <code class="block text-xs text-muted-foreground">
            "ConfirmRoot / ToastRoot / ScrollBar"
          </code>
          <p class="text-xs leading-relaxed text-muted-foreground">
            {move || t("design.comp-global-note")}
          </p>
          <code class="block text-xs break-all text-foreground">
            {move || {
              let text = last.get();
              if text.is_empty() { t("design.comp-idle") } else { text }
            }}
          </code>
        </div>
      </div>
    </SectionCard>

    <MessageDialog
      open=message
      title=String::from("design.comp-message-title")
      description=Signal::derive(move || t("design.comp-message-desc"))
      confirm_text=String::from("design.comp-message-ok")
    />
  }
}

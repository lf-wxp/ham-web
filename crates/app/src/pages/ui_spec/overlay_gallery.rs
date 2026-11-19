//! 弹层与容器：Dialog / Sheet / Toast / Tooltip / 两级卡片。

use leptos::prelude::*;

use crate::components::common::SectionCard;
use crate::i18n::t;
use crate::ui::{
  Button, Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Sheet, Tooltip,
  Variant, push_error, push_info, push_success,
};

#[component]
pub fn OverlayGallery() -> impl IntoView {
  let dialog = RwSignal::new(false);
  let sheet = RwSignal::new(false);

  view! {
    <SectionCard title=Signal::derive(move || t("design.comp-overlays"))>
      <div class="space-y-4 p-4">
        <div class="flex flex-wrap items-center gap-2">
          <Button on_click=Callback::new(move |_| dialog.set(true))>
            {move || t("design.comp-open-dialog")}
          </Button>
          <Button
            variant=Variant::Outline
            on_click=Callback::new(move |_| sheet.set(true))
          >
            {move || t("design.comp-open-sheet")}
          </Button>
          <Button
            variant=Variant::Secondary
            on_click=Callback::new(move |_| push_success(t("design.comp-toast-success")))
          >
            {move || t("design.comp-toast-success")}
          </Button>
          <Button
            variant=Variant::Destructive
            on_click=Callback::new(move |_| push_error(t("design.comp-toast-error")))
          >
            {move || t("design.comp-toast-error")}
          </Button>
          <Button
            variant=Variant::Ghost
            on_click=Callback::new(move |_| push_info(t("design.comp-toast-info")))
          >
            {move || t("design.comp-toast-info")}
          </Button>
          <Tooltip label=Signal::derive(move || t("design.comp-tooltip-text"))>
            <Button variant=Variant::Text>{move || t("design.comp-tooltip")}</Button>
          </Tooltip>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          <div class=crate::kit::card_class_with("space-y-2 p-4")>
            <code class="text-xs text-muted-foreground">"flat_card_class()"</code>
            <p class="text-xs leading-relaxed text-foreground">
              {move || t("design.comp-card-l1")}
            </p>
          </div>
          <div class=crate::kit::window_class()>
            <div class="space-y-2 p-4">
              <code class="text-xs text-muted-foreground">"card_class()"</code>
              <p class="text-xs leading-relaxed text-foreground">
                {move || t("design.comp-card-l2")}
              </p>
            </div>
          </div>
        </div>
      </div>
    </SectionCard>

    <Dialog open=dialog>
      <DialogHeader>
        <DialogTitle>{move || t("design.comp-dialog-title")}</DialogTitle>
        <DialogDescription>{move || t("design.comp-dialog-desc")}</DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <Button on_click=Callback::new(move |_| dialog.set(false))>
          {move || t("design.comp-dialog-close")}
        </Button>
      </DialogFooter>
    </Dialog>

    <Sheet open=sheet>
      <div class="space-y-3 p-4">
        <DialogTitle>{move || t("design.comp-sheet-title")}</DialogTitle>
        <DialogDescription>{move || t("design.comp-sheet-desc")}</DialogDescription>
        <Button
          variant=Variant::Outline
          on_click=Callback::new(move |_| sheet.set(false))
        >
          {move || t("design.comp-dialog-close")}
        </Button>
      </div>
    </Sheet>
  }
}

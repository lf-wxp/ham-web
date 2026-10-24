use leptos::prelude::*;
use leptos::task::spawn_local;
use web_sys::File;

use crate::icons::{Icon, IconKind};
use crate::photo::{self, PhotoKind, PhotoResult};
use crate::ui::{
  Button, CARD_HEADER, Size, Variant, card_class, card_content_class, card_title_class,
};

use super::photo_result_view::PhotoResultView;
use super::photo_uploader::PhotoUploader;
use crate::i18n::t;

#[component]
pub(super) fn PhotoProcessor() -> impl IntoView {
  let processing = RwSignal::new(false);
  let result = RwSignal::new(None::<PhotoResult>);
  let error = RwSignal::new(None::<String>);
  let file_name = RwSignal::new(String::new());

  let on_file = Callback::new(move |(kind, file): (PhotoKind, File)| {
    processing.set(true);
    error.set(None);
    result.set(None);
    file_name.set(file.name());
    spawn_local(async move {
      match photo::process_photo(kind, file).await {
        Ok(r) => {
          result.try_set(Some(r));
        }
        Err(e) => {
          error.try_set(Some(e.message()));
        }
      }
      processing.try_set(false);
    });
  });

  let reset = move |_| {
    result.set(None);
    error.set(None);
    file_name.set(String::new());
  };

  view! {
    <div data-slot="card" class=card_class("")>
      <div data-slot="card-header" class=CARD_HEADER>
        <div data-slot="card-title" class=card_title_class("flex items-center gap-2")>
          <Icon kind=IconKind::Camera class="size-6" />
          {move || t("tools.application-photo-processing")}
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-6")>
        <div class="text-sm text-muted-foreground space-y-2">
          <p>
            {move || t("tools.this-tool-prepares-photos")}
            <a
              href="http://82.157.138.16:8091/CRAC/crac/index.html"
              target="_blank"
              rel="nofollow noopener noreferrer"
              class="text-xp-text hover:text-xp-text underline"
            >
              {move || t("tools.amateur-radio-operator-competence")}
            </a>
            {move || t("tools.requirements-processing-happens-locally")}
          </p>
          <p class="text-gold-text">
            {move || t("tools.this-tool-cannot-change")}
          </p>
        </div>

        {move || {
          (result.with(Option::is_none) && !processing.get())
            .then(|| view! { <PhotoUploader on_file=on_file disabled=processing /> })
        }}

        {move || {
          processing
            .get()
            .then(|| {
              view! {
                <div class="flex flex-col items-center justify-center py-8 space-y-3">
                  <Icon kind=IconKind::Loader2 class="w-8 h-8 animate-spin text-xp-text" />
                  <span class="text-sm text-muted-foreground">{move || t("tools.processing-the-photo-please")}</span>
                </div>
              }
            })
        }}

        {move || {
          error
            .get()
            .map(|e| {
              view! {
                <div class="flex items-start gap-2 p-4 bg-destructive/10 border border-destructive/40 text-hp-text">
                  <Icon kind=IconKind::AlertCircle class="size-6 mt-0.5 flex-shrink-0" />
                  <div class="flex-1">
                    <div>{move || t("tools.processing-failed")}</div>
                    <div class="text-sm mt-1">{e}</div>
                  </div>
                </div>
              }
            })
        }}

        {move || {
          result
            .get()
            .map(|r| {
              view! {
                <div class="space-y-4">
                  <PhotoResultView result=r file_name=file_name.get_untracked() />
                  <div class="flex justify-center">
                    <Button
                      variant=Variant::Outline
                      size=Size::Default
                      on_click=Callback::new(move |_| reset(()))
                    >
                      {move || t("tools.process-another-photo")}
                    </Button>
                  </div>
                </div>
              }
            })
        }}

        <div class="text-xs text-muted-foreground leading-relaxed border-t pt-4">
          <div class="mb-2">{move || t("tools.usage-notes")}</div>
          <p>
            {move || t("tools.the-photo-standards-in")}
          </p>
        </div>
      </div>
    </div>
  }
}

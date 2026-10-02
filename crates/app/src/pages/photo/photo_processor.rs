use leptos::prelude::*;
use leptos::task::spawn_local;
use web_sys::File;

use crate::icons::{Icon, IconKind};
use crate::photo::{self, PhotoKind, PhotoResult};
use crate::ui::{
  CARD_HEADER, Size, Variant, button_class, card_class, card_content_class, card_title_class,
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
          error.try_set(Some(if e.is_empty() {
            t("处理失败，请重试")
          } else {
            e
          }));
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
          <Icon kind=IconKind::Camera class="w-5 h-5" />
          {move || t("报名照片处理")}
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-6")>
        <div class="text-sm text-gray-600 dark:text-gray-400 space-y-2">
          <p>
            {move || t("本工具旨在处理照片，使之符合")}
            <a
              href="http://82.157.138.16:8091/CRAC/crac/index.html"
              target="_blank"
              rel="nofollow noopener noreferrer"
              class="text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300 underline"
            >
              {move || t("业余无线电台操作技术能力验证及信息管理系统")}
            </a>
            {move || t("的要求。处理过程在设备本地处理，不会保存到服务器。")}
          </p>
          <p class="text-amber-700 dark:text-amber-400">
            {move || t("⚠️ 本工具无法帮助处理人像照片的底色要求，仅能处理尺寸。若需换白底，请自行寻找其他解决方案。")}
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
                  <Icon kind=IconKind::Loader2 class="w-8 h-8 animate-spin text-blue-500" />
                  <span class="text-sm text-gray-600 dark:text-gray-400">{move || t("正在处理照片，请稍候...")}</span>
                </div>
              }
            })
        }}

        {move || {
          error
            .get()
            .map(|e| {
              view! {
                <div class="flex items-start gap-2 p-4 bg-red-50 border border-red-200 rounded-lg text-red-800 dark:bg-red-950/40 dark:border-red-900 dark:text-red-200">
                  <Icon kind=IconKind::AlertCircle class="w-5 h-5 mt-0.5 flex-shrink-0" />
                  <div class="flex-1">
                    <div class="font-medium">{move || t("处理失败")}</div>
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
                    <button class=button_class(Variant::Outline, Size::Default, "") on:click=reset>
                      {move || t("处理其他照片")}
                    </button>
                  </div>
                </div>
              }
            })
        }}

        <div class="text-xs text-gray-500 dark:text-gray-400 leading-relaxed border-t pt-4">
          <div class="font-medium mb-2">{move || t("使用须知")}</div>
          <p>
            {move || t("本工具提供的照片处理标准基于开发时的无线电考试报名要求制作。因政策可能随时调整，使用者应自行核实最新官方标准。开发者不对因政策变化导致的格式不符承担任何责任。使用者须自行验证处理后的照片是否符合要求，因照片不合格导致的报名问题，开发者不承担任何法律责任。本工具为第三方便民服务，与各级无线电管理机构无任何隶属或合作关系。使用本工具应同意上述条款。")}
          </p>
        </div>
      </div>
    </div>
  }
}

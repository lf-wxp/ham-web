//! 报名照片处理页面。

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::{DragEvent, File, HtmlInputElement};

use crate::icons::{Icon, IconKind};
use crate::photo::{self, PhotoKind, PhotoResult};
use crate::ui::{
  CARD_HEADER, Size, Variant, button_class, card_class, card_content_class, card_title_class,
};
use crate::util::{alert, set_title};

#[component]
pub fn PhotoProcessorPage() -> impl IntoView {
  set_title("报名照片处理工具 - 业余无线电执照考试");
  view! {
    <main class="container mx-auto px-4 py-6 max-w-4xl space-y-4 pb-28 sm:pb-20 animate-in fade-in slide-in-from-bottom-2 duration-300 ease-out">
      <div class="mb-8">
        <h1 class="text-lg font-semibold mb-2">"业余无线电报名照片处理"</h1>
        <p class="text-muted-foreground">"专业的照片处理工具，帮助您快速处理符合报名要求的证件照和人像照"</p>
      </div>
      <PhotoProcessor />
    </main>
  }
}

#[component]
fn PhotoProcessor() -> impl IntoView {
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
            "处理失败，请重试".to_owned()
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
          "报名照片处理"
        </div>
      </div>
      <div data-slot="card-content" class=card_content_class("space-y-6")>
        <div class="text-sm text-gray-600 dark:text-gray-400 space-y-2">
          <p>
            "本工具旨在处理照片，使之符合"
            <a
              href="http://82.157.138.16:8091/CRAC/crac/index.html"
              target="_blank"
              rel="nofollow noopener noreferrer"
              class="text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300 underline"
            >
              "业余无线电台操作技术能力验证及信息管理系统"
            </a>
            "的要求。处理过程在设备本地处理，不会保存到服务器。"
          </p>
          <p class="text-amber-600 dark:text-amber-400">
            "⚠️ 本工具无法帮助处理人像照片的底色要求，仅能处理尺寸。若需换白底，请自行寻找其他解决方案。"
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
                  <span class="text-sm text-gray-600 dark:text-gray-400">"正在处理照片，请稍候..."</span>
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
                    <div class="font-medium">"处理失败"</div>
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
                      "处理其他照片"
                    </button>
                  </div>
                </div>
              }
            })
        }}

        <div class="text-xs text-gray-500 dark:text-gray-400 leading-relaxed border-t pt-4">
          <div class="font-medium mb-2">"使用须知"</div>
          <p>
            "本工具提供的照片处理标准基于开发时的无线电考试报名要求制作。因政策可能随时调整，使用者应自行核实最新官方标准。开发者不对因政策变化导致的格式不符承担任何责任。使用者须自行验证处理后的照片是否符合要求，因照片不合格导致的报名问题，开发者不承担任何法律责任。本工具为第三方便民服务，与各级无线电管理机构无任何隶属或合作关系。使用本工具应同意上述条款。"
          </p>
        </div>
      </div>
    </div>
  }
}

#[component]
fn PhotoUploader(
  on_file: Callback<(PhotoKind, File)>,
  #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
  let drag_over = RwSignal::new(None::<PhotoKind>);

  let submit = move |kind: PhotoKind, file: File| {
    if !photo::is_image(&file) {
      alert("请选择有效的图片文件");
      return;
    }
    on_file.run((kind, file));
  };

  let zone = move |kind: PhotoKind,
                   id: &'static str,
                   icon: IconKind,
                   title: &'static str,
                   hint: &'static str| {
    let class = move || {
      let state = if disabled.get() {
        "opacity-50 cursor-not-allowed"
      } else {
        "hover:bg-gray-50 dark:hover:bg-accent/50"
      };
      let border = if drag_over.get() == Some(kind) {
        "border-blue-500 bg-blue-50 dark:border-blue-400 dark:bg-blue-950/40"
      } else {
        "border-gray-300 dark:border-gray-600"
      };
      format!(
        "relative flex flex-col items-center justify-center p-6 border-2 border-dashed rounded-lg cursor-pointer transition-all duration-200 {state} {border}"
      )
    };
    view! {
      <div class="space-y-2">
        <label
          for=id
          class=class
          on:dragover=move |e: DragEvent| e.prevent_default()
          on:dragenter=move |e: DragEvent| {
            e.prevent_default();
            drag_over.set(Some(kind));
          }
          on:dragleave=move |e: DragEvent| {
            e.prevent_default();
            if let Some(el) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
              let r = el.get_bounding_client_rect();
              let (x, y) = (f64::from(e.client_x()), f64::from(e.client_y()));
              if x < r.left() || x > r.right() || y < r.top() || y > r.bottom() {
                drag_over.set(None);
              }
            }
          }
          on:drop=move |e: DragEvent| {
            e.prevent_default();
            drag_over.set(None);
            if let Some(file) = e.data_transfer().and_then(|dt| dt.files()).and_then(|f| f.get(0)) {
              submit(kind, file);
            }
          }
        >
          <Icon kind=icon class="w-8 h-8 text-gray-400 dark:text-gray-500 mb-2" />
          <span class="text-sm font-medium text-gray-700 dark:text-gray-200">{title}</span>
          <span class="text-xs text-gray-500 dark:text-gray-400 mt-1">"点击选择或拖拽文件到此处"</span>
          <span class="text-xs text-gray-400 dark:text-gray-500 mt-1">{hint}</span>
        </label>
        <input
          id=id
          type="file"
          accept="image/*"
          disabled=move || disabled.get()
          class="hidden"
          on:change=move |e| {
            let input: HtmlInputElement = event_target(&e);
            if let Some(file) = input.files().and_then(|f| f.get(0)) {
              submit(kind, file);
            }
            input.set_value("");
          }
        />
      </div>
    }
  };

  view! {
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
      {zone(PhotoKind::Id, "id-file", IconKind::CreditCard, "选取证件照", "推荐尺寸: 1024×768 - 4096×3072")}
      {zone(PhotoKind::Profile, "profile-file", IconKind::User, "选取人像照", "推荐尺寸: 300×400 - 3375×4500")}
    </div>
  }
}

#[component]
fn PhotoResultView(result: PhotoResult, file_name: String) -> impl IntoView {
  let r = StoredValue::new(result.clone());
  let name = StoredValue::new(file_name);
  view! {
    <div class="space-y-4">
      <div class="flex items-center gap-2 p-3 bg-green-50 border border-green-200 rounded-lg text-green-800 dark:bg-green-950/40 dark:border-green-900 dark:text-green-200">
        <Icon kind=IconKind::CheckCircle class="w-4 h-4" />
        <span class="text-sm">"处理完成！请右键或长按保存下方处理后的图片。"</span>
      </div>
      <div class="text-sm font-mono text-gray-600 bg-gray-50 dark:text-gray-300 dark:bg-muted p-3 rounded-lg">
        <div class="grid grid-cols-2 gap-2">
          <div>"文件大小: " {photo::format_size(result.size)}</div>
          <div>"格式: " {result.mime.clone()}</div>
          <div>"宽度: " {result.width} "px"</div>
          <div>"高度: " {result.height} "px"</div>
        </div>
      </div>
      <div class="relative w-full max-w-md mx-auto">
        <div class="relative aspect-auto border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
          <img
            src=result.data_url.clone()
            alt="处理后的照片"
            width=result.width
            height=result.height
            class="w-full h-auto object-contain"
          />
        </div>
      </div>
      <div class="flex justify-center">
        <button
          class=button_class(Variant::Default, Size::Default, "flex items-center gap-2")
          on:click=move |_| r.with_value(|r| name.with_value(|n| photo::download(r, n)))
        >
          <Icon kind=IconKind::Download class="w-4 h-4" />
          "下载处理后的照片"
        </button>
      </div>
    </div>
  }
}

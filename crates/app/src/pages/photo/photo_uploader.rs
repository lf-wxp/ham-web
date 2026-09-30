use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{DragEvent, File, HtmlInputElement};

use crate::icons::{Icon, IconKind};
use crate::photo::{self, PhotoKind};
use crate::util::alert;

#[component]
pub(super) fn PhotoUploader(
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

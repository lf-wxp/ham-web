//! 单通道图像展示：展示一张 PNG data URL 并支持下载。

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::t;
use crate::ui::{Button, Size, Variant};
use crate::util::document;

fn download(name: &str, url: &str) {
  let Ok(el) = document().create_element("a") else {
    return;
  };
  let a: web_sys::HtmlAnchorElement = el.unchecked_into();
  a.set_href(url);
  a.set_download(name);
  if let Some(body) = document().body() {
    let _ = body.append_child(&a);
    a.click();
    let _ = body.remove_child(&a);
  }
}

#[component]
pub(super) fn ChannelView(
  #[prop(into)] label: String,
  url: String,
  #[prop(into)] download_name: String,
) -> impl IntoView {
  let download_url = url.clone();
  let label_alt = label.clone();
  view! {
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <h3 class="text-sm">{label}</h3>
        <Button
          variant=Variant::Outline
          size=Size::Sm
          on_click=move |_| download(&download_name, &download_url)
        >
          {move || t("tools.download-png")}
        </Button>
      </div>
      <img src=url alt=label_alt class="w-full border bg-black/5" />
    </div>
  }
}

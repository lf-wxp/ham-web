//! 证件照 / 人像照两个上传位：整块可点击、可拖放，选中后按图片校验再回调。
//!
//! 拖放区本身是 `ui::FileDropZone`（两个上传位共用一份，只有用途、图标、文案与 id 不同）。
//! 这里只留「这一个文件该不该收」的判断：非图片走原生 `alert` 提示，不进入处理流程。

use leptos::prelude::*;
use web_sys::File;

use crate::i18n::t;
use crate::icons::IconKind;
use crate::photo::{self, PhotoKind};
use crate::ui::FileDropZone;
use crate::util::alert;

#[component]
pub(super) fn PhotoUploader(
  on_file: Callback<(PhotoKind, File)>,
  #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
  let submit = move |kind: PhotoKind, file: File| {
    if !photo::is_image(&file) {
      alert(&t("common.choose-a-valid-image"));
      return;
    }
    on_file.run((kind, file));
  };

  // 标题与推荐尺寸由调用点构造：`t("…")` 的字面量留在调用点，`check-i18n` 的静态扫描才认得出。
  let zone = move |kind: PhotoKind,
                   id: &'static str,
                   icon: IconKind,
                   title: Signal<String>,
                   hint: Signal<String>| {
    view! {
      <FileDropZone
        on_file=Callback::new(move |file: File| submit(kind, file))
        accept="image/*"
        title=title
        icon=icon
        hint=hint
        // 上传的是图片：用通用那句，不要组件默认的「拖拽音频文件」。
        choose_hint=Signal::derive(move || t("tools.click-to-choose-or"))
        disabled=disabled
        id=id
      />
    }
  };

  view! {
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
      {zone(
        PhotoKind::Id,
        "id-file",
        IconKind::CreditCard,
        Signal::derive(move || t("tools.choose-an-id-photo")),
        Signal::derive(move || t("tools.recommended-1024-768-4096")),
      )}
      {zone(
        PhotoKind::Profile,
        "profile-file",
        IconKind::User,
        Signal::derive(move || t("tools.choose-a-portrait-photo")),
        Signal::derive(move || t("tools.recommended-300-400-3375")),
      )}
    </div>
  }
}

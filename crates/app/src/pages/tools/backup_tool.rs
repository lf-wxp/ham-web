use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::ui::{Size, Variant, button_class};

/// 数据备份：导出 / 导入全部本地数据。
#[component]
pub(super) fn BackupTool() -> impl IntoView {
  let import = move |e: web_sys::Event| {
    let Some(target) = e.target() else { return };
    let input: web_sys::HtmlInputElement = target.unchecked_into();
    let Some(file) = input.files().and_then(|f| f.get(0)) else {
      return;
    };
    spawn_local(async move {
      let Some(text) = crate::util::read_file_text(&file).await else {
        return;
      };
      match crate::util::import_backup(&text) {
        Ok(n) => crate::util::alert(&format!("已恢复 {n} 条数据")),
        Err(e) => crate::util::alert(&format!("导入失败：{e}")),
      }
    });
  };

  view! {
    <div class="flex flex-wrap items-center gap-2">
      <button
        type="button"
        class=button_class(Variant::Default, Size::Default, "")
        on:click=move |_| crate::util::export_backup()
      >
        "导出备份"
      </button>
      <label class=button_class(Variant::Outline, Size::Default, "cursor-pointer")>
        "导入备份"
        <input type="file" accept=".json" class="hidden" on:change=import />
      </label>
    </div>
  }
}

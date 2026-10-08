//! 文件选择：把「隐藏的 `<input type="file">` + 触发按钮」这一常见组合封装成组件。

use leptos::html;
use leptos::prelude::*;
use web_sys::File;

use crate::i18n::t;

use super::control::TextValue;
use super::{Size, Variant, button_class};

/// 文件选择按钮。
#[component]
pub fn FileInput(
  /// 选中文件后的回调（多选时按序全部给出）。
  on_files: Callback<Vec<File>>,
  /// `accept` 过滤串，如 `".json,application/json"`。
  #[prop(optional, into)]
  accept: Option<String>,
  #[prop(optional)] multiple: bool,
  /// 按钮文案（默认「选择文件」）。
  #[prop(optional, into)]
  label: TextValue,
  #[prop(optional)] variant: Variant,
  #[prop(optional)] size: Size,
  #[prop(optional, into)] disabled: Signal<bool>,
  #[prop(optional, into)] class: String,
  #[prop(optional)] node_ref: NodeRef<html::Input>,
) -> impl IntoView {
  let picked = RwSignal::new(String::new());
  let label = if label.is_empty() {
    TextValue::from(t("common.choose-a-file"))
  } else {
    label
  };
  // `<input type="file">` 视觉上被隐藏，但读屏 / axe 仍把它当作一个表单控件，
  // 因此必须自带无障碍名称（可见的按钮文案充当不了它的 label）。
  let aria_label = label.clone();

  let pick = move |_| {
    if let Some(el) = node_ref.get() {
      el.click();
    }
  };
  let read = move |_| {
    let Some(el) = node_ref.get() else {
      return;
    };
    let mut out = Vec::new();
    if let Some(list) = el.files() {
      for i in 0..list.length() {
        if let Some(f) = list.get(i) {
          out.push(f);
        }
      }
    }
    picked.set(out.first().map_or_else(String::new, File::name));
    // 读完必须清空 input 的 value：否则再选**同一个文件**不会触发 `change`，
    // 用户看到的是「点了没反应」（替换影像、重新导入同一份备份都会撞上）。
    el.set_value("");
    on_files.run(out);
  };

  view! {
    <div class="flex min-w-0 items-center gap-2">
      <input
        node_ref=node_ref
        type="file"
        tabindex="-1"
        class="sr-only"
        accept=accept
        multiple=multiple
        aria-label=move || aria_label.get()
        disabled=move || disabled.get()
        on:change=read
      />
      <button
        type="button"
        class=button_class(variant, size, &class)
        disabled=move || disabled.get()
        on:click=pick
      >
        {move || label.get()}
      </button>
      <Show when=move || !picked.get().is_empty()>
        <span class="truncate text-xs text-muted-foreground">{move || picked.get()}</span>
      </Show>
    </div>
  }
}

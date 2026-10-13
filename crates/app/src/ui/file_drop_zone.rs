//! 文件拖放上传区：整块可点击、可拖放，选中文件后回调。

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{DragEvent, File, HtmlInputElement};

use crate::i18n::t;
use crate::icons::{Icon, IconKind};
use crate::util::unique_id;

use super::control::TextValue;

/// 文件拖放上传区。
///
/// `FileInput` 只是个按钮，表达不了「把文件拖到一块区域上」；上传解码类页面
/// （APT / SSTV / WSPR / RTTY / Olivia / NAVTEX / LRPT / DominoEX / PSK31 / 离线瀑布分析）
/// 要的就是整块拖放，所以单独成组件。页面里不要再手写 `<label for>` + 隐藏 input。
#[component]
pub fn FileDropZone(
  /// 选中文件后的回调（拖放或点击都只取第一个文件）。
  on_file: Callback<File>,
  /// `accept` 过滤串，如 `".wav,audio/wav,audio/x-wav,audio/*"`。
  #[prop(into)]
  accept: String,
  /// 主标题，同时充当隐藏 input 的无障碍名。
  #[prop(into)]
  title: TextValue,
  /// 标题上方的图标。
  icon: IconKind,
  /// 主标题下方的补充说明（「推荐用 RTL-SDR 录制」这类）；不传则不渲染。
  #[prop(optional, into)]
  hint: Option<TextValue>,
  /// 「点击选择或拖拽…」那一行的文案；不传用音频版
  /// （[`tools.click-to-choose-or-2`]，解码类页面的默认）。上传的不是音频就覆盖它。
  #[prop(optional, into)]
  choose_hint: Option<TextValue>,
  #[prop(optional, into)] disabled: Signal<bool>,
  /// `label` 的 `for` 与 input 的 `id`；不传则内部生成。e2e 按固定 id
  /// 定位（`#psk-file`）时显式传入。
  #[prop(optional, into)]
  id: Option<String>,
) -> impl IntoView {
  let id = id.unwrap_or_else(|| unique_id("file-drop"));
  let aria_label = title.clone();
  let drag_over = RwSignal::new(false);

  let submit = move |file: File| {
    if disabled.get_untracked() {
      return;
    }
    on_file.run(file);
  };

  let zone_class = move || {
    let state = if disabled.get() {
      "opacity-50 cursor-not-allowed"
    } else {
      "hover:bg-muted/40 cursor-pointer"
    };
    let border = if drag_over.get() {
      "border-primary bg-primary/5"
    } else {
      "border-border"
    };
    format!(
      "flex flex-col items-center justify-center gap-1 border-2 border-dashed p-8 text-center transition-all duration-200 has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring {state} {border}"
    )
  };

  view! {
    <div>
      <label
        for=id.clone()
        class=zone_class
        on:dragover=move |e: DragEvent| e.prevent_default()
        on:dragenter=move |e: DragEvent| {
          e.prevent_default();
          drag_over.set(true);
        }
        on:dragleave=move |e: DragEvent| {
          e.prevent_default();
          // 进入子元素也会触发 dragleave：用坐标判断是否真的离开了整块区域，
          // 否则拖拽经过文字时会闪一下。
          if let Some(el) = e.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            let r = el.get_bounding_client_rect();
            let (x, y) = (f64::from(e.client_x()), f64::from(e.client_y()));
            if x < r.left() || x > r.right() || y < r.top() || y > r.bottom() {
              drag_over.set(false);
            }
          }
        }
        on:drop=move |e: DragEvent| {
          e.prevent_default();
          drag_over.set(false);
          if let Some(f) = e
            .data_transfer()
            .and_then(|dt| dt.files())
            .and_then(|f| f.get(0))
          {
            submit(f);
          }
        }
      >
        <Icon kind=icon class="mb-1 h-8 w-8 text-muted-foreground" />
        <span class="text-sm font-medium">{move || title.get()}</span>
        <span class="text-xs text-muted-foreground">
          {move || match choose_hint.as_ref() {
            Some(h) => h.get(),
            None => t("tools.click-to-choose-audio-file"),
          }}
        </span>
        {move || {
          hint.clone().map(|h| {
            view! { <span class="mt-1 text-xs text-muted-foreground">{move || h.get()}</span> }
          })
        }}
      </label>
      // 隐藏 input 用 `sr-only` 而不是 `hidden`：`display:none` 的 input 不可聚焦，
      // 键盘用户就无法选文件；`sr-only` 留在 Tab 序里，焦点环由 label 的
      // `has-[:focus-visible]` 显示。
      <input
        id=id.clone()
        type="file"
        accept=accept
        aria-label=move || aria_label.get()
        disabled=move || disabled.get()
        class="sr-only"
        on:change=move |e| {
          let input: HtmlInputElement = event_target(&e);
          if let Some(f) = input.files().and_then(|f| f.get(0)) {
            submit(f);
          }
          input.set_value("");
        }
      />
    </div>
  }
}

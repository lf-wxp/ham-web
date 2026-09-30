use leptos::prelude::*;

use crate::ui::{Dialog, DialogDescription, DialogHeader, DialogTitle};

/// 题图：默认渲染缩略图，点击放大；也可通过 `trigger` 自定义触发元素。
#[component]
pub fn PreviewableImage(
  #[prop(into)] src: String,
  #[prop(into)] alt: String,
  #[prop(optional, into)] title: Option<String>,
  #[prop(optional)] small_trigger: bool,
) -> impl IntoView {
  let open = RwSignal::new(false);
  let title = title.unwrap_or_else(|| alt.clone());
  let trigger = if small_trigger {
    view! {
      <span
        role="button"
        tabindex="0"
        aria-label="预览题图"
        class="absolute -top-1 -left-1 inline-flex items-center justify-center h-4 min-w-4 px-1 rounded bg-background/90 border text-[10px] leading-none cursor-pointer"
        on:click=move |e| {
          e.stop_propagation();
          open.set(true);
        }
        on:keydown=move |e| {
          if e.key() == "Enter" || e.key() == " " {
            e.prevent_default();
            e.stop_propagation();
            open.set(true);
          }
        }
      >
        "图"
      </span>
    }
    .into_any()
  } else {
    view! {
      <div class="relative w-full h-40 sm:h-64">
        <img
          src=src.clone()
          alt=alt.clone()
          loading="lazy"
          decoding="async"
          class="absolute inset-0 h-full w-full object-contain rounded border cursor-zoom-in"
          on:click=move |_| open.set(true)
        />
      </div>
    }
    .into_any()
  };

  let (src, title) = (StoredValue::new(src), StoredValue::new(title));
  view! {
    {trigger}
    <Dialog open=open class="max-w-[90vw]">
      <DialogHeader>
        <DialogTitle class="sr-only">{title.get_value()}</DialogTitle>
        <DialogDescription class="sr-only">"点击空白处或按 Esc 关闭对话框"</DialogDescription>
      </DialogHeader>
      <div class="relative w-full h-[80vh]">
        <img src=src.get_value() alt=title.get_value() class="absolute inset-0 h-full w-full object-contain" />
      </div>
    </Dialog>
  }
}

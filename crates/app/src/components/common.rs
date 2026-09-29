//! 通用组件：提示弹窗、答案解析卡片、可预览图片、进度头部、底部操作栏。

use ham_exam_core::QuestionItem;
use leptos::prelude::*;

use crate::cn::cn;
use crate::icons::{Icon, IconKind};
use crate::ui::{
  Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Progress, Size, Variant,
  button_class, card_class,
};

/// 单按钮提示弹窗。
#[component]
pub fn MessageDialog(
  open: RwSignal<bool>,
  #[prop(into, default = "提示".into())] title: String,
  #[prop(into)] description: Signal<String>,
  #[prop(into, default = "确定".into())] confirm_text: String,
) -> impl IntoView {
  let title = StoredValue::new(title);
  let confirm_text = StoredValue::new(confirm_text);
  view! {
    <Dialog open=open>
      <DialogHeader>
        <DialogTitle>{title.get_value()}</DialogTitle>
        {move || {
          let d = description.get();
          if d.is_empty() {
            view! { <DialogDescription class="sr-only">"弹窗提示"</DialogDescription> }.into_any()
          } else {
            view! { <DialogDescription>{d}</DialogDescription> }.into_any()
          }
        }}
      </DialogHeader>
      <DialogFooter>
        <button class=button_class(Variant::Default, Size::Default, "") on:click=move |_| open.set(false)>
          {confirm_text.get_value()}
        </button>
      </DialogFooter>
    </Dialog>
  }
}

/// 可折叠的「答案解析」卡片；题目切换时由调用方重建，自动恢复折叠状态。
#[component]
pub fn ExplanationCard(question: QuestionItem) -> impl IntoView {
  let open = RwSignal::new(false);
  let text = question
    .explanation
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(ToOwned::to_owned);
  text.map(|text| {
    view! {
      <div data-slot="card" class=card_class("gap-0 py-0")>
        <button
          type="button"
          on:click=move |_| open.update(|v| *v = !*v)
          aria-expanded=move || open.get().to_string()
          class="flex w-full items-center justify-between gap-2 px-6 py-4 text-left cursor-pointer transition-colors hover:bg-accent/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 rounded-xl"
        >
          <span class="text-base font-semibold leading-none">"答案解析"</span>
          <Icon
            kind=IconKind::ChevronDown
            class=Signal::derive(move || cn(&["h-4 w-4 shrink-0 transition-transform", if open.get() { "rotate-180" } else { "" }]))
          />
        </button>
        {move || {
          open
            .get()
            .then(|| {
              view! {
                <div class="px-6 pb-5">
                  <div class="whitespace-pre-line text-sm leading-6 text-muted-foreground border-t pt-4">
                    {text.clone()}
                  </div>
                </div>
              }
            })
        }}
      </div>
    }
  })
}

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

/// 进度头部：可选左侧控件、右侧按钮、进度条与说明。
#[component]
pub fn QuestionProgressHeader(
  #[prop(into)] percent: Signal<i64>,
  #[prop(into)] right: ViewFn,
  #[prop(optional, into)] meta: Option<ViewFn>,
  #[prop(optional, into)] left: Option<ViewFn>,
) -> impl IntoView {
  let top_row = if left.is_some() {
    "flex flex-wrap items-center justify-between gap-x-4 gap-y-2"
  } else {
    "flex flex-wrap items-center justify-end gap-x-4 gap-y-2"
  };
  view! {
    <div class="space-y-2">
      <div class=top_row>
        {left.as_ref().map(|l| view! { <div class="flex flex-wrap items-center gap-2">{l.run()}</div> })}
        <div class="flex items-center gap-3">{right.run()}</div>
      </div>
      <div class="flex items-center gap-4 justify-between flex-wrap">
        <div class="flex items-center gap-4 w-full sm:w-auto">
          <div class="min-w-24 text-sm text-muted-foreground">"进度 " {move || percent.get()} "%"</div>
          <Progress value=percent class="h-2 flex-1 sm:flex-none" />
        </div>
        {meta.map(|m| view! { <div class="hidden sm:block text-sm text-muted-foreground">{m.run()}</div> })}
      </div>
    </div>
  }
}

/// 固定在底部的操作栏（桌面与移动端分别布局）。
#[component]
pub fn BottomBar(
  #[prop(into)] stats: ViewFn,
  #[prop(into)] left: ViewFn,
  #[prop(into)] right: ViewFn,
  #[prop(into)] mobile_top: ViewFn,
  #[prop(optional, into)] mobile_bottom: Option<ViewFn>,
) -> impl IntoView {
  view! {
    <div
      class="fixed left-0 right-0 bottom-0 z-40 border-t bg-background/85 backdrop-blur supports-[backdrop-filter]:bg-background/60"
      style="padding-bottom: env(safe-area-inset-bottom);"
    >
      <div class="container mx-auto max-w-4xl px-4 py-2 space-y-2">
        <div class="text-sm text-muted-foreground text-center">{stats.run()}</div>
        <div class="hidden sm:flex items-center justify-between gap-2">
          <div>{left.run()}</div>
          <div class="flex items-center gap-2"></div>
          <div class="flex items-center gap-2">{right.run()}</div>
        </div>
        <div class="sm:hidden space-y-2">
          <div>{mobile_top.run()}</div>
          {mobile_bottom.map(|m| view! { <div>{m.run()}</div> })}
        </div>
      </div>
    </div>
  }
}

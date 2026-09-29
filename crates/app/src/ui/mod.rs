//! 基础 UI 组件：与原项目 shadcn/ui（new-york 风格）保持相同的 Tailwind 类名与交互。

mod checkbox;
mod dialog;
mod radio;
mod select;

pub use checkbox::Checkbox;
pub use dialog::{Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Sheet};
pub use radio::{RadioGroup, RadioGroupItem};
pub use select::{Select, SelectItem};

use leptos::prelude::*;

use crate::cn::cn;

/// 按钮样式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Variant {
  #[default]
  Default,
  Destructive,
  Outline,
  Secondary,
  Ghost,
}

/// 按钮尺寸。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Size {
  #[default]
  Default,
  Sm,
  Icon,
}

const BUTTON_BASE: &str = "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all active:scale-[0.98] cursor-pointer disabled:cursor-not-allowed disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive";

/// 生成按钮类名（等价于 shadcn `buttonVariants({ variant, size, className })`）。
pub fn button_class(variant: Variant, size: Size, extra: &str) -> String {
  let v = match variant {
    Variant::Default => "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90",
    Variant::Destructive => {
      "bg-destructive text-white shadow-xs hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60"
    }
    Variant::Outline => {
      "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:bg-input/30 dark:border-input dark:hover:bg-input/50"
    }
    Variant::Secondary => "bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80",
    Variant::Ghost => "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
  };
  let s = match size {
    Size::Default => "h-9 px-4 py-2 has-[>svg]:px-3",
    Size::Sm => "h-8 rounded-md gap-1.5 px-3 has-[>svg]:px-2.5",
    Size::Icon => "size-9",
  };
  cn(&[BUTTON_BASE, v, s, extra])
}

/// 徽标样式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BadgeVariant {
  #[default]
  Default,
  Secondary,
  Outline,
}

const BADGE_BASE: &str = "inline-flex items-center justify-center rounded-md border px-2 py-0.5 text-xs font-medium w-fit whitespace-nowrap shrink-0 [&>svg]:size-3 gap-1 [&>svg]:pointer-events-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive transition-[color,box-shadow] overflow-hidden";

/// 生成徽标类名。
pub fn badge_class(variant: BadgeVariant, extra: &str) -> String {
  let v = match variant {
    BadgeVariant::Default => {
      "border-transparent bg-primary text-primary-foreground [a&]:hover:bg-primary/90"
    }
    BadgeVariant::Secondary => {
      "border-transparent bg-secondary text-secondary-foreground [a&]:hover:bg-secondary/90"
    }
    BadgeVariant::Outline => {
      "text-foreground [a&]:hover:bg-accent [a&]:hover:text-accent-foreground"
    }
  };
  cn(&[BADGE_BASE, v, extra])
}

/// 卡片容器类名。
pub fn card_class(extra: &str) -> String {
  cn(&[
    "bg-card text-card-foreground flex flex-col gap-6 rounded-xl border py-6 shadow-sm",
    extra,
  ])
}

/// 卡片头部类名。
pub const CARD_HEADER: &str = "@container/card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-1.5 px-6 has-data-[slot=card-action]:grid-cols-[1fr_auto] [.border-b]:pb-6";

/// 卡片标题类名。
pub fn card_title_class(extra: &str) -> String {
  cn(&["leading-none font-semibold", extra])
}

/// 卡片内容类名。
pub fn card_content_class(extra: &str) -> String {
  cn(&["px-6", extra])
}

/// 输入框类名。
pub fn input_class(extra: &str) -> String {
  cn(&[
    "file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input flex h-9 w-full min-w-0 rounded-md border bg-transparent px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
    "focus-visible:border-ring focus-visible:ring-ring/50",
    "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
    extra,
  ])
}

/// 表单标签类名。
pub fn label_class(extra: &str) -> String {
  cn(&[
    "flex items-center gap-2 text-sm leading-none font-medium select-none group-data-[disabled=true]:pointer-events-none group-data-[disabled=true]:opacity-50 peer-disabled:cursor-not-allowed peer-disabled:opacity-50",
    extra,
  ])
}

/// 表单标签。
#[component]
pub fn Label(
  #[prop(into)] r#for: String,
  #[prop(optional, into)] class: Signal<String>,
  children: Children,
) -> impl IntoView {
  view! {
    <label data-slot="label" for=r#for class=move || label_class(&class.get())>
      {children()}
    </label>
  }
}

/// 水平分割线。
#[component]
pub fn Separator() -> impl IntoView {
  view! {
    <div
      data-slot="separator"
      role="none"
      data-orientation="horizontal"
      class="bg-border shrink-0 data-[orientation=horizontal]:h-px data-[orientation=horizontal]:w-full data-[orientation=vertical]:h-full data-[orientation=vertical]:w-px"
    ></div>
  }
}

/// 进度条。
#[component]
pub fn Progress(
  #[prop(into)] value: Signal<i64>,
  #[prop(optional, into)] class: String,
) -> impl IntoView {
  let class = cn(&[
    "bg-primary/20 relative h-2 w-full overflow-hidden rounded-full",
    &class,
  ]);
  view! {
    <div
      data-slot="progress"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow=move || value.get().to_string()
      aria-label="作答进度"
      class=class
    >
      <div
        data-slot="progress-indicator"
        class="bg-primary h-full w-full flex-1 transition-all"
        style=move || format!("transform: translateX(-{}%)", 100 - value.get())
      ></div>
    </div>
  }
}

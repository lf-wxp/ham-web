//! 表单控件的共享基线：尺寸刻度与类名工厂。
//!
//! 所有替代原生元素的控件（[`crate::ui::Input`]、[`crate::ui::Textarea`]、
//! [`crate::ui::NativeSelect`] …）都走同一条基线，保证边框、圆角、聚焦环、禁用态与
//! 错误态在项目内只有一处定义。

use leptos::prelude::*;

use crate::cn::cn;

/// 表单控件尺寸（区别于按钮的 [`crate::ui::Size`]）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ControlSize {
  /// 紧凑 `h-8`：工具栏筛选这类横向排布、数量密集的场景。
  Sm,
  /// 默认 `h-9`：表单主体。移动端保留 16px 字号，避免 iOS 聚焦时自动放大页面。
  #[default]
  Default,
  /// 大号 `h-11`：触屏优先的主表单（如搜索跳转框）。
  Lg,
}

impl ControlSize {
  /// 尺寸对应的高度 / 字号 / 内边距类名。
  #[must_use]
  pub const fn class(self) -> &'static str {
    match self {
      Self::Sm => "h-8 gap-1.5 px-2.5 text-xs md:text-xs",
      Self::Default => "h-9 px-3 text-base md:text-sm",
      Self::Lg => "h-11 px-4 text-base md:text-base",
    }
  }
}

/// 控件样式基线（等价于 shadcn `inputVariants`，聚焦环加粗到 4px、降到 30% 透明度，
/// 做成一圈「柔光」而不是一道硬边）。
///
/// 底色是半透明的 `bg-background/60`：控件放在玻璃卡片里时能透出一点背景，不会像一块
/// 贴上去的白板；悬停先把描边提亮到 `ring/50`，让「这里可以点」在聚焦之前就有反馈。
/// `aria-invalid` 的变体在 CSS 里排在 `hover` / `focus-visible` 之后，错误态的红色描边不会被它们盖掉。
pub const CONTROL_BASE: &str = "file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input flex w-full min-w-0 rounded-lg border bg-background/60 px-3 py-1 shadow-xs transition-[color,box-shadow,border-color] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium hover:border-ring/50 disabled:cursor-not-allowed disabled:pointer-events-none disabled:opacity-50 focus-visible:border-ring focus-visible:ring-ring/30 focus-visible:ring-[4px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive";

/// 生成控件类名：`基线 + 尺寸 + 调用方覆盖`。
pub fn control_class(size: ControlSize, extra: &str) -> String {
  cn(&[CONTROL_BASE, size.class(), extra])
}

/// 复选框 / 开关这类「非输入框」控件的聚焦环，与 [`CONTROL_BASE`] 对齐。
pub const CONTROL_RING: &str =
  "focus-visible:border-ring focus-visible:ring-ring/30 focus-visible:ring-[4px]";

/// `aria-invalid` 只在错误态出现在 DOM 上。
///
/// Tailwind 的 `aria-invalid:` 变体按属性存在与否命中，因此常态必须是「无此属性」，
/// 不能写成 `aria-invalid="false"`（否则整站输入框都会套上 destructive 描边）。
pub fn invalid_attr(
  invalid: Signal<bool>,
) -> impl Fn() -> Option<&'static str> + Copy + Send + Sync {
  move || invalid.get().then_some("true")
}

/// 控件文案：静态字符串，或跟随语言切换的响应式信号。
///
/// 界面文案由 [`crate::i18n::t`] 按全局语言信号翻译，切语言时只有处在响应式上下文里的
/// 调用才会重算。因此 `placeholder` / `aria_label` 一类属性既要能接静态 `String`，也要能
/// 接 `Signal<String>`：
///
/// - `placeholder = t("exam.search-questions")` —— 静态，够用于不会切语言的文案；
/// - `placeholder = Signal::derive(move || t("exam.search-questions"))` —— 切语言后自动更新。
#[derive(Clone, Debug)]
pub struct TextValue(TextInner);

#[derive(Clone, Debug)]
enum TextInner {
  Static(String),
  Dynamic(Signal<String>),
}

impl TextValue {
  /// 取当前文案。
  #[must_use]
  pub fn get(&self) -> String {
    match &self.0 {
      TextInner::Static(s) => s.clone(),
      TextInner::Dynamic(s) => s.get(),
    }
  }

  /// 是否为空（空串的属性一律不渲染，见各组件的 `non_empty` 处理）。
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.get().is_empty()
  }
}

impl Default for TextValue {
  fn default() -> Self {
    Self(TextInner::Static(String::new()))
  }
}

impl<'a> From<&'a str> for TextValue {
  fn from(v: &'a str) -> Self {
    Self(TextInner::Static(v.to_owned()))
  }
}

impl From<String> for TextValue {
  fn from(v: String) -> Self {
    Self(TextInner::Static(v))
  }
}

impl From<Signal<String>> for TextValue {
  fn from(v: Signal<String>) -> Self {
    Self(TextInner::Dynamic(v))
  }
}

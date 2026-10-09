//! 基础 UI 组件：与原项目 shadcn/ui（new-york 风格）保持相同的 Tailwind 类名与交互。
//!
//! 本目录是**全站表单控件的唯一来源**：页面里不应再直接写 `<input>` / `<select>` /
//! `<textarea>` / `<input type="range">` 等原生元素，统一用这里的组件，颜色、圆角、
//! 间距、字体与 hover / focus / disabled / error 状态反馈才能始终一致。
//!
//! > **写代码前先读 `docs/ui-components.md`**：那是使用规范（决策表「要什么用哪个」、
//! > prop 约定、无障碍与多语言要求、常见坑、验收清单）；本文件与 `src/ui/README.md`
//! > 是组件清单与示例。改动本目录时也要同步更新那两份文档。
//!
//! # 组件清单
//!
//! | 组件 | 替代的原生写法 | 说明 |
//! |---|---|---|
//! | [`Button`] | `<button class=button_class(…)>` | [`button_class`] 的组件化封装 |
//! | [`ButtonLink`] | `<a href class=button_class(…)>`（跳转用） | 元素仍是链接（中键新开 / 右键复制地址），外观同 [`Button`] |
//! | [`Input`] | `<input>`（text / search / password / email / tel / url；日期与时间见下两行） | 支持前缀、后缀、一键清除、回车回调 |
//! | [`NumberField`] | `<input type="number">` | 数值录入 + 可选加减步进；负数 / 要保留半截输入时用 `kind=NumberKind::Decimal` |
//! | [`Textarea`] | `<textarea>` | 支持随内容自动增高 |
//! | [`Select`] + [`SelectItem`] | `<select>`（选项需富文本展示时） | 弹层式单选，对应 Radix Select（语言切换即用此组件） |
//! | [`NativeSelect`] | `<select>`（选项固定的筛选器） | 弹层式单选，外观复用 [`Select`]，故与语言切换一致 |
//! | [`DatePicker`] | `<input type="date">` | 弹层日历，样式同 [`Select`] |
//! | [`TimePicker`] | `<input type="time">` | 弹层双列（时 / 分），样式同 [`Select`] |
//! | [`Checkbox`] | `<input type="checkbox">`（表单项） | 对应 Radix Checkbox |
//! | [`Switch`] | `<input type="checkbox">`（切换即生效的设置） | 对应 Radix Switch |
//! | [`RadioGroup`] + [`RadioGroupItem`] | `<input type="radio">` | 对应 Radix RadioGroup |
//! | [`Slider`] | `<input type="range">` | 轨道 / 滑块自绘，进度用 CSS 变量 |
//! | [`ChipGroup`] + [`Chip`] | 手写的分段选择胶囊（`CHIP_ON` / `CHIP_OFF`） | 互斥选择，比 [`RadioGroup`] 紧凑，适合工具条 |
//! | [`ChipToggle`] | 手写的筛选 chip 开关 | `aria-pressed` 语义，允许一个都不选 |
//! | [`FileInput`] | 隐藏 `<input type="file">` + 触发按钮 | 按钮外观走 [`button_class`] |
//! | [`Field`] | 手写的 `label + 控件 + 提示` 排版 | 标签 / 说明 / 错误文案统一 |
//! | [`Label`] | `<label>` | 单独使用时（如 Checkbox 旁） |
//! | [`Dialog`] / [`Sheet`] 及配套 | — | 模态与抽屉 |
//! | [`Progress`] | — | 进度条 |
//! | [`Separator`] | `<hr>` | 分割线 |
//! | [`Stat`] | — | 统计卡片 |
//!
//! # 通用约定
//!
//! - **受控值**：所有输入组件都是受控的 —— `value` 只读、`on_change` 只写。这样调用方
//!   可以在 `on_change` 里做校验 / 格式化后决定是否回写。
//! - **尺寸**：输入类用 [`ControlSize`]（`Sm` / `Default` / `Lg`），按钮用 [`Size`]。
//! - **错误态**：`invalid` 为真时输出 `aria-invalid="true"`，由 Tailwind 的
//!   `aria-invalid:` 变体切换到 `destructive` 描边；常态不输出该属性。
//! - **类名覆盖**：`class` 会经 [`cn`] 与默认类名合并，后写胜出，可安全覆盖 `w-*`、`h-*` 等。
//! - **无障碍**：无可见标签时务必传 `aria_label`。
//! - **文件组织**：一个组件一个文件（整个仓库都这样，业务组件也一样）；一族多组件
//!   （`Dialog` + 它的部件、`RadioGroup` + `RadioGroupItem`、`ChipGroup` + `Chip` +
//!   `ChipToggle`）用文件夹模块 —— `mod.rs` 只放模块文档 + `mod` 声明 + `pub use`，
//!   共享的样式 token / context 放 `shared.rs`。
//!   文件名不能与父目录同名（`chip/chip.rs` 会被 Rust 拒绝，用 `chip_item.rs`）。
//!
//! # 使用示例
//!
//! ```text
//! use leptos::prelude::*;
//! use crate::ui::{ControlSize, Field, Input, InputType, NativeSelect, NumberField, SelectOption};
//!
//! let kw = RwSignal::new(String::new());
//! let band = RwSignal::new(String::new());
//! let count = RwSignal::new(String::from("32"));
//!
//! view! {
//!   <Field label="搜索题目" r#for="kw">
//!     <Input
//!       id="kw"
//!       value=kw
//!       on_change=Callback::new(move |v| kw.set(v))
//!       kind=InputType::Search
//!       size=ControlSize::Sm
//!       clearable=true
//!       placeholder="输入题干关键词"
//!     />
//!   </Field>
//!
//!   <NativeSelect
//!     value=band
//!     on_change=Callback::new(move |v| band.set(v))
//!     options=vec![
//!       SelectOption::new("20m", "20 米"),
//!       SelectOption::new("40m", "40 米"),
//!     ]
//!     placeholder="全部波段"
//!     size=ControlSize::Sm
//!     aria_label="波段筛选"
//!   />
//!
//!   <NumberField value=count on_change=Callback::new(move |v| count.set(v)) min=0.0 max=300.0 step=1.0 />
//! }
//! ```
//!
//! 更完整的对照表与迁移步骤见 `src/ui/README.md`。

mod button;
mod button_link;
mod checkbox;
mod chip;
mod control;
mod date_picker;
mod dialog;
mod field;
mod file_input;
mod input;
mod intl;
mod label;
mod native_select;
mod number_field;
mod popover;
mod progress;
mod radio;
mod select;
mod separator;
mod slider;
mod stat;
mod switch;
mod textarea;
mod time_picker;

pub use button::{Button, ButtonKind};
pub use button_link::ButtonLink;
pub use checkbox::Checkbox;
pub use chip::{Chip, ChipGroup, ChipToggle};
pub use control::{ControlSize, TextValue, control_class};
pub use date_picker::DatePicker;
pub use dialog::{Dialog, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Sheet};
pub use field::Field;
pub use file_input::FileInput;
pub use input::{Input, InputType};
pub use label::Label;
pub use native_select::{NativeSelect, SelectOption};
pub use number_field::{NumberField, NumberKind};
pub use progress::Progress;
pub use radio::{RadioGroup, RadioGroupItem};
pub use select::{Select, SelectItem};
pub use separator::Separator;
pub use slider::Slider;
pub use stat::Stat;
pub use switch::Switch;
pub use textarea::Textarea;
pub use time_picker::TimePicker;

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

// `motion-press` 取代原来的 `transition-all`：只过渡 transform / 阴影 / 颜色这几项，
// 按下时用 back 曲线回弹、抬起时走 instant 快速收尾（见 style/input.css 动效语言一节）。
// `relative` 供 `btn-ripple` 的 `::after` 定位；`active:shadow-none` 让按下时「沉下去」。
// 禁用态：半透明 + 去阴影 + 降饱和 + 不响应指针，与可用态有明显区分。
const BUTTON_BASE: &str = "relative inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium btn-ripple motion-press active:scale-[0.97] active:shadow-none cursor-pointer disabled:cursor-not-allowed disabled:pointer-events-none disabled:opacity-50 disabled:shadow-none disabled:saturate-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive";

/// 生成按钮类名（等价于 shadcn `buttonVariants({ variant, size, className })`）。
///
/// 悬停时颜色加深并加大阴影（实心变体用带色阴影做「发光」，描边 / 次级只轻抬一档），
/// 与按下时的 `active:scale-*` + `active:shadow-none` 组成「悬停浮起 → 按下沉下」的完整手感。
pub fn button_class(variant: Variant, size: Size, extra: &str) -> String {
  let v = match variant {
    Variant::Default => {
      "bg-primary text-primary-foreground shadow-xs shadow-primary/25 hover:bg-primary/90 hover:shadow-md hover:shadow-primary/30"
    }
    Variant::Destructive => {
      "bg-destructive text-white shadow-xs hover:bg-destructive/90 hover:shadow-md hover:shadow-destructive/30 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60"
    }
    Variant::Outline => {
      "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground hover:shadow-sm dark:bg-input/30 dark:border-input dark:hover:bg-input/50"
    }
    Variant::Secondary => {
      "bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80 hover:shadow-sm"
    }
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
    "bg-card text-card-foreground flex flex-col gap-6 rounded-xl border py-6 shadow-sm mb-[24px]",
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

/// 表单标签类名。
pub fn label_class(extra: &str) -> String {
  cn(&[
    "flex items-center gap-2 text-sm leading-none font-medium select-none group-data-[disabled=true]:pointer-events-none group-data-[disabled=true]:opacity-50 peer-disabled:cursor-not-allowed peer-disabled:opacity-50",
    extra,
  ])
}

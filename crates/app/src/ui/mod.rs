//! 基础 UI 组件：像素风（Pixel Art）控件库。外观由 `style/pixel/controls.css` 的 `px-*` 类提供，
//! 组件 API 沿用 shadcn/ui 的 props 约定（`variant` / `size` / 受控 `value` + `on_change`）。
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
//! | [`ChipGroup`] + [`Chip`] | 手写的分段选择胶囊 | 互斥选择，比 [`RadioGroup`] 紧凑，适合工具条 |
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
  /// 大号 `h-12`：落地页主操作（首页 Hero 的三个入口）。更大的圆角与字号，
  /// 让它在版面里明显是「主角」；表单与工具栏里不要用。
  Lg,
  Icon,
}

// 外观全部在 `style/pixel/controls.css` 的 `.pxl-btn*`：凸起 → 悬停上浮 2px → 按下沉 2px。
// 这里只拼类名，不再写一长串工具类 —— 手感涉及位移、阴影、内高光同时变化，放在 CSS 里才不会漏。
// 图标尺寸：像素图标是 24px 网格，`size-6` 是 1:1，其余尺寸会有轻微抗锯齿（见 surfaces.css）。
const BUTTON_BASE: &str = "pxl-btn [&_svg:not([class*='size-'])]:size-6";

/// 生成按钮类名。
///
/// 悬停上浮 2px 并提亮，按下下沉 2px、投影消失、内高光翻转成内阴影；
/// 禁用态是抖动网点遮罩而不是半透明。
pub fn button_class(variant: Variant, size: Size, extra: &str) -> String {
  let v = match variant {
    Variant::Default => "pxl-btn-primary",
    Variant::Destructive => "pxl-btn-destructive",
    Variant::Outline => "pxl-btn-outline",
    Variant::Secondary => "pxl-btn-secondary",
    Variant::Ghost => "pxl-btn-ghost",
  };
  // 高度取 4px 的倍数；按钮文字是 12px 点阵，上下留白按 4px 网格算。
  let s = match size {
    Size::Default => "h-10 px-4 has-[>svg]:px-3",
    Size::Sm => "h-8 gap-1.5 px-3 has-[>svg]:px-2",
    Size::Lg => "h-12 px-6 text-base has-[>svg]:px-4",
    Size::Icon => "size-10 px-0",
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

const BADGE_BASE: &str = "pxl-badge [&>svg]:size-4 [&>svg]:pointer-events-none";

/// 生成徽标类名。
pub fn badge_class(variant: BadgeVariant, extra: &str) -> String {
  let v = match variant {
    BadgeVariant::Default => "bg-primary text-primary-foreground [a&]:hover:brightness-110",
    BadgeVariant::Secondary => "bg-secondary text-secondary-foreground [a&]:hover:brightness-110",
    BadgeVariant::Outline => {
      "bg-card text-foreground [a&]:hover:bg-accent [a&]:hover:text-accent-foreground"
    }
  };
  cn(&[BADGE_BASE, v, extra])
}

/// 卡片容器类名：像素「窗口面板」。
///
/// 描边 / 内高光 / 硬偏移投影在 `.pxl-window`。存量手写的 `rounded-xl border bg-card` 卡片
/// 靠 `style/pixel/surfaces.css` 的兜底规则统一加粗描边，两条路径视觉一致。
pub fn card_class(extra: &str) -> String {
  cn(&["pxl-window flex flex-col gap-6 py-6 mb-[24px]", extra])
}

/// 卡片头部类名。
pub const CARD_HEADER: &str = "@container/card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-1.5 px-6 has-data-[slot=card-action]:grid-cols-[1fr_auto] [.border-b]:pb-6";

/// 卡片标题类名。
pub fn card_title_class(extra: &str) -> String {
  cn(&["pxl-title text-sm leading-none", extra])
}

/// 卡片内容类名。
pub fn card_content_class(extra: &str) -> String {
  cn(&["px-6", extra])
}

/// 表单标签类名。
pub fn label_class(extra: &str) -> String {
  cn(&[
    "flex items-center gap-2 text-sm leading-none select-none group-data-[disabled=true]:pointer-events-none group-data-[disabled=true]:opacity-50 peer-disabled:cursor-not-allowed peer-disabled:opacity-50",
    extra,
  ])
}

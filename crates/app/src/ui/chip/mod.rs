//! 分段选择 chip：工具条上一排紧凑的胶囊按钮。
//!
//! 与 [`RadioGroup`](super::RadioGroup) 的分工是**密度**而不是语义：
//!
//! - `RadioGroup` 是表单里的单选 —— 选项带文字、可以排成网格、和 `Field` 标签配套；
//! - `ChipGroup` 是工具条上的分段选择 —— 选项本身很短（`20m`、「三单元八木」），
//!   横排一行、选中即高亮，塞进 `Field` 反而挤。
//!
//! 两者都是「互斥选择」，都输出 `role="radiogroup"` / `role="radio"` + `aria-checked`，
//! 读屏与键盘的行为一致。单个开关（筛选器上的「只看需要的」）用 [`ChipToggle`]，
//! 它表达的是独立的开 / 关，因此用 `aria-pressed`。
//!
//! 样式只有一份：迁移前 `nec/*`、`waveform_lab/*`、`smith`、`qsl_designer`、`dx_spots`
//! 各自定义过 4 种不同的 chip 写法（圆角、内边距、选中态描边都不一样）。
//!
//! 本目录按「一个组件一个文件」组织（见 `AGENTS.md`）：共享的样式 token 与组上下文在
//! [`shared`]，另外三个文件各放一个组件。

mod chip_group;
mod chip_item;
mod chip_toggle;
mod shared;

pub use chip_group::ChipGroup;
pub use chip_item::Chip;
pub use chip_toggle::ChipToggle;

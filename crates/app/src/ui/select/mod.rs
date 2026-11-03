//! 下拉选择（对应 Radix Select）。
//!
//! 这里是全站**唯一**的弹层式下拉引擎：键盘导航、焦点离开收起、ARIA、箭头动画只在
//! [`Select`] 里写一遍。组合式用法直接用 `Select` + [`SelectItem`] + [`SelectGroup`]；
//! 数据式用法（选项是 `Vec<SelectOption>`）见 [`crate::ui::SimpleSelect`]，它内部就是用本组件拼的。

mod select_group;
mod select_item;
mod select_root;

pub use select_group::SelectGroup;
pub use select_item::SelectItem;
pub use select_root::Select;

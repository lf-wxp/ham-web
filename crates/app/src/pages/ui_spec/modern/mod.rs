//! `/ui-spec` 的现代风分支：七个板块各占一个文件，静态数据在 `data.rs`。
//!
//! 展示项目与像素风逐项对齐：通用组件与状态两块直接复用同一批共享 `crate::ui` 实样
//! （gallery 里的展示性类名按皮肤分叉），其余板块用现代风自己的 `--md-*` 取值重写。
//! 不新增 CSS 类、不渲染任何 `pxl-*` DOM。

mod colors;
mod components;
mod data;
mod feedback;
mod icons;
mod modern_ui_spec;
mod motion;
mod overview;
mod radio;
mod states;
mod typography;

pub use modern_ui_spec::ModernUiSpec;

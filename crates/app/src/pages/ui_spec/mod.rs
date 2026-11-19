//! `/ui-spec`：UI 设计规范展示页（像素风为完整规范，现代风为占位页）。
//!
//! 七个板块各占一个文件（一文件一组件），静态数据表集中在 `spec_data.rs` 里，
//! 那里面没有返回视图的函数。展示用的控件全部取 `crate::ui`，不在页面里重画一套。
//! 「通用组件」板块再用一层子标签把基础控件、进阶表单、数据、电台面板等分组。

mod action_gallery;
mod advanced_input_gallery;
mod button_gallery;
mod colors_section;
mod component_index;
mod components_section;
mod data_gallery;
mod feedback_gallery;
mod form_gallery;
mod icons_section;
mod modern_placeholder;
mod motion_section;
mod overlay_gallery;
mod overview_section;
mod placeholder_gallery;
mod radio_gallery;
mod spec_data;
mod states_section;
mod typography_section;
mod ui_spec_page;

pub use ui_spec_page::UiSpecPage;

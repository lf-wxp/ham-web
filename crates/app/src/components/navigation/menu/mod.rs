//! 顶部导航的菜单状态。
//!
//! 菜单项（分组、路径、标题、图标）统一来自 `ham_web_core::registry`，
//! 这里只保留下拉展开状态所需的 [`MenuKind`]。

mod types;

pub(crate) use types::MenuKind;

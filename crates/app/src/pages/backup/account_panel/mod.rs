//! 在线账号面板：登录 / 注册、云端备份的手动同步、账号管理。
//!
//! 与 `remote_sync_panel`（自定义远端）并列，两者交换同一份备份、共用手动动作执行器
//! （`sync_runner`）与自动同步状态机（`crate::auto_sync`），设计见 `docs/domain/account-sync.md`。
//! 一个文件一个组件：外壳按登录态分派，表单 / 已登录视图 / 两个对话框各一个文件。

mod account_session;
mod auth_form;
mod delete_dialog;
mod panel_root;
mod password_dialog;

pub(super) use panel_root::AccountPanel;

//! 数据备份：本地数据的导出 / 导入与存储占用一览。
//!
//! 原本是小工具页（`/tools`）里最后一个区块，埋在 60 多个计算器之后；
//! 存储写满、换设备、误删恢复这类场景都要求它「随手能找到」，因此独立成页。
//!
//! 页面下半是两条并列的同步通道：在线账号（`account_panel`）与自定义远端（`remote_sync_panel`）。
//! 它们交换同一份备份，共用手动动作执行器（`sync_runner`）与自动同步状态行（`sync_status_line`）；
//! 自动同步一次只跑一条，开关见 `auto_sync_switch`。设计见 `docs/domain/account-sync.md`。

mod account_panel;
mod auto_sync_switch;
mod backup_page;
mod backup_tool;
mod remote_sync_panel;
mod sync_runner;
mod sync_status_line;

pub use backup_page::BackupPage;

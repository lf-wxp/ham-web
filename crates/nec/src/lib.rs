//! NEC 天线建模计算核心：矩量法求解、Yagi 优化、方向图与网格生成。
//!
//! 从 `ham-web-core` 拆出，纯 `std` 实现、零外部依赖。拆出的目的是让计算 Worker
//! 只依赖本 crate（约 5k 行），不必拖入整个 `ham-web-core`（约 49k 行）。
//!
//! 前端仍通过 `ham_web_core::{cx, nec, nec_mesh, nec_templates}` 访问（core 以
//! `pub use` 重导出），因此现有页面无需改动。

pub mod cx;
pub mod nec;
pub mod nec_mesh;
pub mod nec_templates;

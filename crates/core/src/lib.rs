//! 业余无线电执照考试模拟的核心领域模型。
//!
//! 该 crate 不依赖任何浏览器或 IO 能力，同时被 WASM 前端（`ham-exam-app`）
//! 与构建/维护工具（`ham-exam-tools`）使用，保证两端的题目格式、内容指纹、
//! 考试规则与分类体系完全一致。

pub mod bands;
pub mod bank;
pub mod categories;
pub mod exam;
pub mod fingerprint;
pub mod glossary;
pub mod practice;
pub mod question;
pub mod saved_state;
pub mod text;

pub use bank::{Bank, BankConfig, BankInfo, QuestionVersion};
pub use exam::{ExamRule, ExamScore};
pub use fingerprint::fingerprint;
pub use question::{Codes, Pages, QuestionItem, QuestionOption, QuestionType};

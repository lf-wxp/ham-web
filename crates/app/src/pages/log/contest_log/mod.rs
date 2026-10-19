//! 竞赛录入：快速键盘录入、实时查重与自报分数，通联直接写入通联日志，可一键导出 Cabrillo。

mod analysis;
mod cabrillo_import;
mod import_report;
mod pace;
mod scp_group;
mod scp_source;
mod scp_suggest;
mod session;
mod view;

pub use view::ContestLogPage;

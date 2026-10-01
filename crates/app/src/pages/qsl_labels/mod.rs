//! QSL 标签打印：把日志中的通联按呼号合并，排到 Avery 不干胶标签纸上直接打印。

mod label_view;
mod qsl_labels_page;

pub use qsl_labels_page::QslLabelsPage;

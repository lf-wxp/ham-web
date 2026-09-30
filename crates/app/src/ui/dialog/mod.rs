//! 模态对话框与侧边抽屉（对应 Radix Dialog / shadcn Sheet）。
//!
//! 行为：打开时挂载到 `<body>`、锁定页面滚动、聚焦首个可交互元素；Esc 或点击遮罩关闭；
//! 关闭时保留 `data-state="closed"` 一段时间以播放退出动画。

mod dialog_description;
mod dialog_footer;
mod dialog_header;
mod dialog_root;
mod dialog_title;
mod shared;
mod sheet;

pub use dialog_description::DialogDescription;
pub use dialog_footer::DialogFooter;
pub use dialog_header::DialogHeader;
pub use dialog_root::Dialog;
pub use dialog_title::DialogTitle;
pub use sheet::Sheet;

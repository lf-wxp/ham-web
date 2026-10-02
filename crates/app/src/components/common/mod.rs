//! 通用组件：提示弹窗、答案解析卡片、可预览图片、进度头部、底部操作栏，
//! 以及页面骨架（页头/容器/卡片）与状态反馈（加载/空态）。

mod bottom_bar;
mod empty_state;
mod explanation_card;
mod legend;
mod loading;
mod message_dialog;
mod note_editor;
mod page_container;
mod page_header;
mod previewable_image;
mod question_progress_header;
mod section_card;
mod study_heatmap;

pub use bottom_bar::BottomBar;
pub use empty_state::EmptyState;
pub use explanation_card::ExplanationCard;
pub use legend::Legend;
pub use loading::Loading;
pub use message_dialog::MessageDialog;
pub use note_editor::NoteEditor;
pub use page_container::PageContainer;
pub use page_header::PageHeader;
pub use previewable_image::PreviewableImage;
pub use question_progress_header::QuestionProgressHeader;
pub use section_card::SectionCard;
pub use study_heatmap::StudyHeatmap;

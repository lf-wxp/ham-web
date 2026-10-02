//! 模拟考试相关组件：成绩、恢复、设置、交卷确认对话框与答题卡。

mod answer_card_sheet;
mod category_compare;
mod custom_paper_dialog;
mod exam_result_dialog;
mod exam_resume_dialog;
mod exam_settings_dialog;
mod exam_submit_confirm_dialog;
mod shortcut_row;

pub use answer_card_sheet::{AnswerCardFilter, AnswerCardSheet};
pub use custom_paper_dialog::CustomPaperDialog;
pub use exam_result_dialog::ExamResultDialog;
pub use exam_resume_dialog::ExamResumeDialog;
pub use exam_settings_dialog::ExamSettingsDialog;
pub use exam_submit_confirm_dialog::ExamSubmitConfirmDialog;
pub use shortcut_row::ShortcutRow;

//! 模拟考试相关组件：成绩、恢复、设置、交卷确认对话框与答题卡。

mod answer_card_cell;
mod answer_card_legend;
mod answer_card_model;
mod answer_card_panel;
mod answer_card_sheet;
mod boss_verdict;
mod category_compare;
mod custom_paper_dialog;
mod exam_result_dialog;
mod exam_resume_dialog;
mod exam_settings_dialog;
mod exam_submit_confirm_dialog;
mod rules_walkthrough_dialog;
mod share_call_toggle;
mod shortcut_row;

pub use answer_card_model::{AnswerCardModel, cell_label};
pub use answer_card_panel::AnswerCardPanel;
pub use answer_card_sheet::AnswerCardSheet;
pub use share_call_toggle::ShareCallToggle;
// 筛选枚举在 core（有单测）；这里再导出一次，页面与现代版仍从 `components::exam` 取。
pub use custom_paper_dialog::CustomPaperDialog;
pub use exam_result_dialog::ExamResultDialog;
pub use exam_resume_dialog::ExamResumeDialog;
pub use exam_settings_dialog::ExamSettingsDialog;
pub use exam_submit_confirm_dialog::ExamSubmitConfirmDialog;
pub use ham_web_core::answer_card::AnswerCardFilter;
pub use rules_walkthrough_dialog::RulesWalkthroughDialog;
pub use shortcut_row::ShortcutRow;

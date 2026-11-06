//! 现代设计系统（第二套 UI）。
//!
//! 与 `ui/` + `style/pixel/` 是**两套独立实现**：类名 `md-*`、token `--md-*`、动效平滑曲线，
//! 不引用任何 `pxl-*` / `--pxl-*` / 像素图标 / 精灵。隔离约定与验收口径见
//! `docs/ui/decisions.md`。
//!
//! 覆盖范围：外壳（顶栏 / dock / 页脚 / 设置 / 目录 / 调谐台）、首页，以及练习与考试两条主流程
//! （页头、底栏、答题卡、成绩、各类弹层）。其余页面复用 `ui/` 与 `components/` 里按
//! `kit::is_modern_at_mount()` 派发的共享控件，不在这里重复实现。

mod answer_card;
mod backdrop;
mod bubble;
mod confirm;
mod custom_paper;
mod dialog;
mod dock;
mod emblem;
mod exam_bar;
mod exam_clock;
mod exam_head;
mod exam_result;
mod exam_resume;
mod exam_rules;
mod exam_settings;
mod exam_submit;
mod explanation;
mod footer;
mod home;
mod icons;
mod input;
pub mod interaction;
mod menu;
mod menu_drawer;
mod menu_item;
mod message;
mod module_icon;
pub mod motion;
mod nav;
mod note;
mod page_head;
mod page_mode;
mod palette;
mod practice_bar;
mod practice_head;
mod practice_resume;
mod practice_search;
mod practice_settings;
mod rail;
mod select;
mod settings;
mod shortcut_help;
mod skeleton;
mod spectrum;
mod spectrum_paint;
mod textarea;
mod view_transition;

pub use answer_card::ModernAnswerCard;
pub use bubble::ModernBubble;
pub use confirm::ModernConfirm;
pub use custom_paper::ModernCustomPaper;
pub use dock::ModernDock;
pub use emblem::{EmblemTone, ModernEmblem, emblem_icon, emblem_tone, emblem_variant};
pub use exam_bar::ModernExamBar;
pub use exam_head::ModernExamHead;
pub use exam_result::ModernExamResult;
pub use exam_resume::ModernExamResume;
pub use exam_rules::ModernExamRules;
pub use exam_settings::ModernExamSettings;
pub use exam_submit::ModernExamSubmit;
pub use explanation::ModernExplanation;
pub use footer::ModernFooter;
pub use home::{ModernHome, ModernStatValue};
pub use icons::{MIcon, ModernIcon};
pub use input::ModernInput;
pub use menu::ModernMenu;
pub use menu_drawer::ModernMenuDrawer;
pub use message::ModernMessage;
pub use nav::ModernNav;
pub use note::ModernNote;
pub use page_head::ModernPageHead;
pub use page_mode::use_page_mode;
pub use palette::ModernPalette;
pub use practice_bar::ModernPracticeBar;
pub use practice_head::ModernPracticeHeader;
pub use practice_resume::ModernPracticeResume;
pub use practice_search::ModernPracticeSearch;
pub use practice_settings::ModernPracticeSettings;
pub use rail::ModernRail;
pub use select::ModernSelect;
pub use shortcut_help::ModernShortcutHelp;
pub use skeleton::ModernLoading;
pub use spectrum::ModernSpectrum;
pub use textarea::ModernTextarea;

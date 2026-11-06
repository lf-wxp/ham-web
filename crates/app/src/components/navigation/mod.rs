//! 全局顶部导航：桌面/移动端导航与主题切换。

mod choice_row;
mod group_menu;
mod hud_badge;
mod locale_toggle;
mod menu;
mod mobile_menu;
mod mobile_nav_section;
mod mobile_tabs;
mod nav_bar;
mod nav_root;
mod scheme_choice;
mod settings_dialog;
mod theme_toggle;

pub use mobile_menu::provide_mobile_menu;
pub use mobile_tabs::MobileTabs;
pub use nav_root::Navigation;

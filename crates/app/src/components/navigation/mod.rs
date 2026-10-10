//! 全局顶部导航：桌面/移动端导航与主题切换。

mod group_menu;
mod hud_badge;
mod locale_toggle;
mod menu;
mod mobile_menu;
mod mobile_tabs;
mod nav_bar;
mod scheme_choice;
mod settings_dialog;
mod theme_choice;
mod theme_toggle;

pub use mobile_menu::provide_mobile_menu;
pub use mobile_tabs::MobileTabs;
pub use nav_bar::Navigation;

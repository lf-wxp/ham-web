//! 根组件：路由、主题、全局布局（导航 / 主体 / 页脚）与 PWA 更新提示。

use leptos::prelude::*;
use leptos_router::components::Router;

use crate::components::navigation::Navigation;
use crate::components::search_dialog::SearchDialog;
use crate::pwa::PwaUpdatePrompt;
use crate::theme::provide_theme;

mod footer;
mod main_content;

use footer::Footer;
use main_content::MainContent;

#[component]
pub fn App() -> impl IntoView {
  provide_theme();
  let search_open = RwSignal::new(false);
  provide_context(search_open);
  view! {
    <Router>
      <PwaUpdatePrompt />
      <Navigation />
      <MainContent />
      <Footer />
      <SearchDialog />
    </Router>
  }
}

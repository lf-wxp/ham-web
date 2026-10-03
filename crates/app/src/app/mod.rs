//! 根组件：路由、主题、全局布局（导航 / 主体 / 页脚）与 PWA 更新提示。

use leptos::prelude::*;
use leptos_router::components::Router;

use crate::components::navigation::Navigation;
use crate::components::search_dialog::SearchDialog;
use crate::pwa::UpdateNotices;
use crate::theme::provide_theme;

mod footer;
mod main_content;
mod storage_warning;

use crate::i18n::t;
use footer::Footer;
use main_content::MainContent;
use storage_warning::StorageWarning;

#[component]
pub fn App() -> impl IntoView {
  provide_theme();
  crate::i18n::provide_locale();
  crate::pages::log::provide_log_store();
  leptos::task::spawn_local(async move {
    crate::study::ensure_seeded().await;
    // 迁移旧数据后，把最新的「今日待复习数」同步给后端，供 Web Push 每日提醒附带数量。
    crate::push::sync_review_counts();
  });
  crate::sat_alert::start_watcher();
  crate::pages::start_global_watcher();
  crate::study::start_study_reminder_watcher();
  let search_open = RwSignal::new(false);
  provide_context(search_open);
  view! {
    <Router>
      <a
        href="#main-content"
        class="sr-only focus:not-sr-only focus:fixed focus:left-3 focus:top-3 focus:z-[60] focus:rounded-md focus:bg-primary focus:px-3 focus:py-2 focus:text-sm focus:font-medium focus:text-primary-foreground"
        on:click=|e| {
          e.prevent_default();
          if let Some(main) = crate::util::window()
            .document()
            .and_then(|d| d.get_element_by_id("main-content"))
            .and_then(|el| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(el).ok())
          {
            let _ = main.focus();
          }
        }
      >
        {move || t("跳到主要内容")}
      </a>
      <UpdateNotices />
      <Navigation />
      <MainContent />
      <Footer />
      <SearchDialog />
      <StorageWarning />
      <crate::achievements::AchievementToast />
    </Router>
  }
}

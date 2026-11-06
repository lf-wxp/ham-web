//! 根组件：路由、主题、全局布局（导航 / 主体 / 页脚）与 PWA 更新提示。

use leptos::prelude::*;
use leptos_router::components::Router;

use crate::components::navigation::{MobileTabs, Navigation, provide_mobile_menu};
use crate::components::search_dialog::{SearchDialog, SearchOpen};
use crate::motion::RouteTransition;
use crate::pwa::UpdateNotices;
use crate::theme::provide_theme;

mod footer;
mod main_content;
mod pixel_footer;
mod storage_warning;

use crate::i18n::t;
use footer::Footer;
use main_content::MainContent;
use storage_warning::StorageWarning;

#[component]
pub fn App() -> impl IntoView {
  // 第一行：它要在应用自己往存储里写任何东西之前，看一眼「本机原来有没有数据」。
  let auto_sync = crate::auto_sync::snapshot();
  provide_theme();
  provide_mobile_menu();
  // 动效环境要在任何页面组件挂载前就位：页面里的 `Reveal` 一挂载就会注册浮现观察器。
  crate::motion::provide_motion();
  // 现代风的按压反馈走一次全局事件委托：控件不需要各自写 `on:pointerdown`。
  crate::kit_modern::interaction::install_press_feedback();
  crate::i18n::provide_locale();
  crate::pages::log::provide_log_store();
  crate::auto_sync::start(crate::pages::log::use_log_store(), auto_sync);
  leptos::task::spawn_local(async move {
    crate::study::ensure_seeded().await;
    // 迁移旧数据后，把最新的「今日待复习数」同步给后端，供 Web Push 每日提醒附带数量。
    crate::push::sync_review_counts();
  });
  crate::sat_alert::start_watcher();
  crate::pages::start_global_watcher();
  crate::study::start_study_reminder_watcher();
  crate::ui::setup_tooltip_popup();
  let search_open = RwSignal::new(false);
  provide_context(SearchOpen(search_open));
  view! {
    <Router>
      <a
        href="#main-content"
        // `data-skip-link`：它在 `.md-root` 之外，现代风的焦点环按它单独换（否则键盘用户
        // 按第一下 Tab 看到的是像素虚线环）。
        data-skip-link
        class="sr-only focus:not-sr-only focus:fixed focus:left-3 focus:top-3 focus:z-[60] focus:bg-primary focus:px-3 focus:py-2 focus:text-sm focus:text-primary-foreground"
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
        {move || t("shell.skip-to-main-content")}
      </a>
      <UpdateNotices />
      // 真机测量面板：只在地址带 `?perf=1` 的会话里出现（见 `perf_hud.rs`）。
      <crate::perf_hud::PerfHud />
      <Navigation />
      <MainContent />
      <RouteTransition />
      // 现代风不挂像素风的底部 Tab：它的导航在顶栏抽屉里（`kit_modern::ModernNav`）。
      {move || {
        (!crate::kit::is_modern()).then(|| view! { <MobileTabs /> })
      }}
      // 像素风的自绘滚动条同理：现代风用系统细滚动条（`style/modern/kit.css`），
      // 两者同时出现会变成右侧一条 12px 的像素滑块。
      {move || {
        (!crate::kit::is_modern()).then(|| view! { <crate::ui::ScrollBar /> })
      }}
      <Footer />
      // 站内搜索（内容 + 页面）：两套顶栏的搜索按钮与 `/` 都指向它，皮肤差异只在弹层外观
      // （`Dialog` / `Input` 走 `ui::` 分派）。调谐台（⌘K）是另一件事 —— 它是键盘路由跳转。
      // 这两个组件体里的风格派发是构造期求值（`is_modern_at_mount`），而它们挂在 `App` 层、
      // 不在被重建的路由子树里：不订阅风格信号重建一次，切完风格后会是「旧弹层壳 + 新控件」。
      // 开合状态在 context / 线程局部里，重建不丢。
      {move || {
        let _ = crate::kit::is_modern();
        view! { <SearchDialog /> }
      }}

      <StorageWarning />
      <crate::achievements::AchievementToast />
      <crate::ui::ToastRoot />
      {move || {
        let _ = crate::kit::is_modern();
        view! { <crate::ui::ConfirmRoot /> }
      }}
    </Router>
  }
}

//! 根组件：路由、主题、全局布局（导航 / 主体 / 页脚）与 PWA 更新提示。

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::components::navigation::Navigation;
use crate::pages::{
  BandsPage, BrowsePage, ExamPage, GlossaryPage, HomePage, NotFoundPage, PhotoProcessorPage,
  PracticePage,
};
use crate::pwa::PwaUpdatePrompt;
use crate::theme::provide_theme;

#[component]
pub fn App() -> impl IntoView {
  provide_theme();
  view! {
    <Router>
      <PwaUpdatePrompt />
      <Navigation />
      <MainContent />
      <Footer />
    </Router>
  }
}

/// 主体内容。
#[component]
fn MainContent() -> impl IntoView {
  view! {
    <main class="flex-1">
      <Routes fallback=|| view! { <NotFoundPage /> }>
        <Route path=path!("/") view=HomePage />
        <Route path=path!("/practice") view=PracticePage />
        <Route path=path!("/exam") view=ExamPage />
        <Route path=path!("/browse") view=BrowsePage />
        <Route path=path!("/glossary") view=GlossaryPage />
        <Route path=path!("/bands") view=BandsPage />
        <Route path=path!("/photo-processor") view=PhotoProcessorPage />
      </Routes>
    </main>
  }
}

#[component]
fn Footer() -> impl IntoView {
  const LINK: &str = "hover:underline underline-offset-4 hover:text-foreground transition-colors";
  view! {
    <footer class="mt-8 border-t bg-secondary/40">
      <div class="max-w-screen-lg mx-auto px-4">
        <div class="py-6 flex flex-col sm:flex-row items-center justify-between gap-3">
          <div class="text-xs sm:text-sm text-muted-foreground">"业余无线电执照考试模拟 · 2025"</div>
          <nav class="text-xs sm:text-sm">
            <ul class="inline-flex flex-wrap items-center gap-2 sm:gap-3 text-muted-foreground">
              <li>
                <a href="https://github.com/lf-wxp/ham-web" rel="nofollow" class=LINK aria-label="仓库地址">
                  "GitHub 仓库"
                </a>
              </li>
              <li aria-hidden="true" class="text-border">"·"</li>
              <li>
                <a href="http://www.crac.org.cn/News/List?type=6&y=" rel="nofollow" class=LINK aria-label="CRAC 题库来源">
                  "CRAC 题库"
                </a>
              </li>
            </ul>
          </nav>
        </div>
      </div>
    </footer>
  }
}

use crate::i18n::t;
use leptos::prelude::*;

#[component]
pub(super) fn Footer() -> impl IntoView {
  const LINK: &str = "hover:underline underline-offset-4 hover:text-foreground transition-colors";
  view! {
    <footer class="relative mt-8 border-t bg-secondary/40">
      <div class="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-primary/40 to-transparent"></div>
      <div class="max-w-screen-lg mx-auto px-4">
        <div class="py-6 flex flex-col sm:flex-row items-center justify-between gap-3">
          <div class="text-xs sm:text-sm text-muted-foreground">{move || t("业余无线电 · 题库 · 知识 · 工具")}</div>
          <nav class="text-xs sm:text-sm">
            <ul class="inline-flex flex-wrap items-center gap-2 sm:gap-3 text-muted-foreground">
              <li>
                <a href="https://github.com/lf-wxp/ham-web" rel="nofollow" class=LINK aria-label=move || t("仓库地址")>
                  {move || t("GitHub 仓库")}
                </a>
              </li>
              <li aria-hidden="true" class="text-border">"·"</li>
              <li>
                <a href="http://www.crac.org.cn/News/List?type=6&y=" rel="nofollow" class=LINK aria-label=move || t("CRAC 题库来源")>
                  {move || t("CRAC 题库")}
                </a>
              </li>
            </ul>
          </nav>
        </div>
      </div>
    </footer>
  }
}

use crate::i18n::t;
use leptos::prelude::*;

#[component]
pub(super) fn Footer() -> impl IntoView {
  const LINK: &str = "hover:underline underline-offset-4 hover:text-foreground";
  view! {
    // 页脚用 2px 的虚线上沿（像存档点的分隔线），底色是实心的 `--secondary`。
    <footer class="relative mt-8 border-t-2 border-dashed border-ink bg-secondary">
      <div class="mx-auto max-w-5xl px-4">
        <div class="py-6 flex flex-col sm:flex-row items-center justify-between gap-3">
          <div class="text-xs sm:text-sm text-muted-foreground">{move || t("shell.amateur-radio-exams-knowledge")}</div>
          <nav class="text-xs sm:text-sm">
            <ul class="inline-flex flex-wrap items-center gap-2 sm:gap-3 text-muted-foreground">
              <li>
                <a href="https://github.com/lf-wxp/ham-web" rel="nofollow" class=LINK aria-label=move || t("shell.repository")>
                  {move || t("shell.github-repository")}
                </a>
              </li>
              <li aria-hidden="true" class="text-border">"·"</li>
              <li>
                <a href="http://www.crac.org.cn/News/List?type=6&y=" rel="nofollow" class=LINK aria-label=move || t("shell.crac-question-bank-source")>
                  {move || t("shell.crac-question-bank")}
                </a>
              </li>
            </ul>
          </nav>
        </div>
      </div>
    </footer>
  }
}

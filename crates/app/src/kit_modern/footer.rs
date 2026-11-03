//! 现代风页脚：一条 1px 分隔线 + 同一排文字链接，没有虚线上沿与实心色块。

use leptos::prelude::*;

use crate::i18n::t;

#[component]
pub fn ModernFooter() -> impl IntoView {
  view! {
    <footer class="md-hairline mt-16">
      // 宽度与内边距取 `.md-shell`（= `--md-page-max` / `--md-page-pad`）：
      // 页脚自己写一个 1200 + px-5 的话，左右边缘会比正文各缩 12px。
      <div class="md-shell flex flex-col items-center justify-between gap-3 py-6 sm:flex-row">
        <div class="md-root md-muted md-sm">
          {move || t("shell.amateur-radio-exams-knowledge")}
        </div>
        <nav class="md-root">
          <ul class="flex flex-wrap items-center justify-center gap-4 md-sm">
            <li>
              <a class="md-nav-link" href="https://github.com/lf-wxp/ham-web" rel="nofollow">
                {move || t("shell.github-repository")}
              </a>
            </li>
            <li>
              <a class="md-nav-link" href="/license.html" aria-label=move || t("shell.license-notice")>
                {move || t("shell.license")}
              </a>
            </li>
            <li>
              <a class="md-nav-link" href="/backup">
                {move || t("tools.data-backup")}
              </a>
            </li>
            <li>
              <a
                class="md-nav-link"
                href="http://www.crac.org.cn/News/List?type=6&y="
                rel="nofollow"
                aria-label=move || t("shell.crac-question-bank-source")
              >
                {move || t("shell.crac-question-bank")}
              </a>
            </li>
          </ul>
        </nav>
      </div>
    </footer>
  }
}

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
                // 可见文本「GitHub 仓库」本身就是完整的名字；再补一个更短的 `aria-label`
                // 会与可见文本对不上（axe 的 label-content-name-mismatch）。
                <a href="https://github.com/lf-wxp/ham-web" rel="nofollow" class=LINK>
                  {move || t("shell.github-repository")}
                </a>
              </li>
              <li aria-hidden="true" class="text-border">"·"</li>
              <li>
                // GPL-3.0 的解码器 wasm 随站点分发，要让用户看得到许可说明与源码出处。
                <a href="/license.html" class=LINK aria-label=move || t("shell.license-notice")>
                  {move || t("shell.license")}
                </a>
              </li>
              <li aria-hidden="true" class="text-border">"·"</li>
              <li>
                // 站内页面：备份是「丢了就找不回」的操作，页脚是每页都能看到的最后一个入口。
                <a href="/backup" class=LINK>
                  {move || t("tools.data-backup")}
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

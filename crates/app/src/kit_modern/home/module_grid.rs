//! 现代风首页的三大模块：三张不同色调的卡片，各带 4 个入口与「查看全部」。
//!
//! 数据来自 `ham_web_core::registry`（与顶部导航同一份）。像素版是「阶梯下沉的三列链接」，
//! 这里是「大序号 + 细描边 + 顶部微渐变」的卡片，两者没有共享任何标记。

use ham_web_core::registry::{self, Module};
use leptos::prelude::*;

use crate::data::core_text_tracked;
use crate::i18n::t;
use crate::kit_modern::ModernRail;

/// 每张卡片列出的入口数：剩下的收在「查看全部」后面，避免首页变成目录页。
const PREVIEW: usize = 4;

#[component]
pub fn ModuleGrid() -> impl IntoView {
  let exam: Vec<&'static Module> = registry::exam_items().into_iter().take(PREVIEW).collect();
  let knowledge: Vec<&'static Module> = registry::KNOWLEDGE_GROUPS
    .iter()
    .flat_map(|g| registry::in_group(g))
    .take(PREVIEW)
    .collect();
  let tools: Vec<&'static Module> = registry::TOOL_GROUPS
    .iter()
    .flat_map(|g| registry::in_group(g))
    .take(PREVIEW)
    .collect();

  view! {
    <section class="mt-14">
      <h2 class="md-h2 mb-4">{move || t("home.modules")}</h2>

      <ModernRail class="md-modules" aria_label=Signal::derive(move || t("home.modules"))>
        <div class="md-mod md-bracket">
          <div class="md-mod-head">
            <a class="md-mod-title" href="/practice">{move || t("shell.exam-center")}</a>
            <span class="md-mod-index" aria-hidden="true">"01"</span>
          </div>
          <div class="md-link-list">
            {exam
              .into_iter()
              .map(|m| {
                view! {
                  <a class="md-link-row" href=m.nav_href()>
                    <span class="md-link-dot" aria-hidden="true"></span>
                    {move || core_text_tracked(m.title)}
                  </a>
                }
              })
              .collect_view()}
            <a class="md-link-row md-link-more" href="/practice">
              {move || t("home.view-all")}
              <crate::kit_modern::ModernIcon
                kind=crate::kit_modern::MIcon::ArrowRight
                class="size-4"
              />
            </a>
          </div>
        </div>

        <div class="md-mod md-mod-blue md-bracket">
          <div class="md-mod-head">
            <a class="md-mod-title" href="/reference">{move || t("shell.knowledge")}</a>
            <span class="md-mod-index" aria-hidden="true">"02"</span>
          </div>
          <div class="md-link-list">
            {knowledge
              .into_iter()
              .map(|m| {
                view! {
                  <a class="md-link-row" href=m.nav_href()>
                    <span class="md-link-dot" aria-hidden="true"></span>
                    {move || core_text_tracked(m.title)}
                  </a>
                }
              })
              .collect_view()}
            <a class="md-link-row md-link-more" href="/reference">
              {move || t("home.view-all")}
              <crate::kit_modern::ModernIcon
                kind=crate::kit_modern::MIcon::ArrowRight
                class="size-4"
              />
            </a>
          </div>
        </div>

        <div class="md-mod md-mod-green md-bracket">
          <div class="md-mod-head">
            <a class="md-mod-title" href="/tools">{move || t("shell.tools")}</a>
            <span class="md-mod-index" aria-hidden="true">"03"</span>
          </div>
          <div class="md-link-list">
            {tools
              .into_iter()
              .map(|m| {
                view! {
                  <a class="md-link-row" href=m.nav_href()>
                    <span class="md-link-dot" aria-hidden="true"></span>
                    {move || core_text_tracked(m.title)}
                  </a>
                }
              })
              .collect_view()}
            <a class="md-link-row md-link-more" href="/tools">
              {move || t("home.view-all")}
              <crate::kit_modern::ModernIcon
                kind=crate::kit_modern::MIcon::ArrowRight
                class="size-4"
              />
            </a>
          </div>
        </div>
      </ModernRail>
    </section>
  }
}

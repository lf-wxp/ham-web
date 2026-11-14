use leptos::prelude::*;

use super::DEFAULT_TITLE;
use crate::components::common::PageContainer;
use crate::components::search_dialog::SearchOpen;
use crate::i18n::t;
use crate::ui::{Button, ButtonLink, LedTone, Readout, ReadoutTone, Size, Variant};
use crate::util::set_title;

#[component]
pub fn NotFoundPage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  // 404 在**路由兜底**里渲染，那里的 `DisplayPrefs` 是默认实例（不是页面树里的那个），
  // 所以 `is_modern()` / `is_modern_at_mount()` 一律返回「像素」。这里读 `<html data-ui-style>`
  // ——同一个事实来源，只是不依赖响应式上下文（`theme.rs` 里唯一的 DOM 读入口）。
  let modern = move || crate::theme::DisplayPrefs::style_is_modern_dom();
  let search = use_context::<SearchOpen>();
  view! {
    <PageContainer class="space-y-0 py-10 animate-in fade-in slide-in-from-bottom-2 duration-(--motion-normal) ease-out">
      // 像素风：一块「无信号」的测试卡屏幕（canvas「404 · 无信号」）。纯装饰（`aria-hidden`）：
      // 下面的标题与说明就是同一件事的文字版。「无信号」不是错误，所以不用红色。
      {move || {
        (!modern())
          .then(|| {
            view! {
              <div class="pxl-display mb-6 max-w-sm" aria-hidden="true">
                <div class="pxl-testcard h-18"></div>
                <div class="flex flex-col items-center gap-2 px-3 py-3">
                  <Readout text="404" scale=4 tone=ReadoutTone::Hot />
                  <span class="pxl-label">{move || t("shell.no-signal")}</span>
                  // 平线中间一个跳变：载波掉了，只剩一下杂波。
                  <svg class="pxl-flatline-svg" viewBox="0 0 240 20" preserveAspectRatio="none">
                    <rect x="0" y="9" width="96" height="2" />
                    <rect x="96" y="5" width="2" height="6" />
                    <rect x="96" y="5" width="4" height="2" />
                    <rect x="100" y="5" width="2" height="12" />
                    <rect x="100" y="15" width="4" height="2" />
                    <rect x="104" y="9" width="2" height="8" />
                    <rect x="104" y="9" width="136" height="2" />
                  </svg>
                </div>
              </div>
            }
          })
      }}
      // 刻意的例外（`ui_check` 的「页面手写 `<h1>`」跳过本文件）：404 是居中的错误页，
      // 没有内容页的「页头 + 主色左轨」语义。
      <h1 class=move || {
        if modern() { "md-h2 mb-2" } else { "pxl-title text-lg mb-2" }
      }>{move || t("knowledge.page-not-found")}</h1>
      {move || {
        if modern() {
          view! {
            <p class="md-muted md-sm mb-4">{move || t("knowledge.the-page-you-requested")}</p>
            <a class=crate::kit::link_class() href="/">{move || t("knowledge.back-to-home")}</a>
          }
            .into_any()
        } else {
          view! {
            <p class="mb-4 text-sm leading-relaxed">{move || t("knowledge.no-signal-hint")}</p>
            <div class="flex flex-wrap gap-2">
              <ButtonLink href="/" variant=Variant::Default size=Size::Default>
                {move || t("knowledge.back-to-home")}
              </ButtonLink>
              {search.map(|SearchOpen(open)| {
                view! {
                  <Button
                    variant=Variant::Outline
                    size=Size::Default
                    led=LedTone::Warn
                    // 灯常亮：这个键没有开关状态，灭着的灯只会被读成「坏了」。
                    led_on=true
                    on_click=Callback::new(move |()| open.set(true))
                  >
                    {move || t("shell.scan-site")}
                  </Button>
                }
              })}
            </div>
          }
            .into_any()
        }
      }}
    </PageContainer>
  }
}

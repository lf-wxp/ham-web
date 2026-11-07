//! 现代风顶栏：56px 细栏 + 纯文字链接 + 一个入口抽屉。
//!
//! 与像素风的 `nav_bar.rs` 是两套实现：
//! - 像素风：64px HUD 横栏、硬投影、三段「分组侧栏 + 条目区」下拉、右侧 HUD 徽章；
//! - 现代风：无投影、无 HUD，导航是一项一条文字链接，其余的塞进右侧抽屉；图标只用
//!   `kit_modern::icons` 的线性图标集（不碰 `icons/pixel` 的 24px 点阵图）。

use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

use crate::components::search_dialog::SearchOpen;
use crate::i18n::t;

use super::settings::ModernSettings;
use super::{MIcon, ModernIcon};

#[component]
pub fn ModernNav() -> impl IntoView {
  let location = use_location();
  let menu_open = RwSignal::new(false);
  let settings_open = RwSignal::new(false);
  let palette_open = RwSignal::new(false);
  // 站内搜索与像素风同一个弹层（`components/search_dialog.rs`）：状态由 App 层提供，
  // 现代风的顶栏按钮与 `/` 都只是它的入口。
  let SearchOpen(search_open) = expect_context::<SearchOpen>();

  // 全局「/」快捷键：与像素风同一条约定（输入框里不劫持）。调谐台因此只留 `⌘K`。
  let search_handle = window_event_listener(ev::keydown, move |e| {
    let typing = e
      .target()
      .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
      .map(|el| {
        let tag = el.tag_name().to_lowercase();
        tag == "input" || tag == "textarea" || el.is_content_editable()
      })
      .unwrap_or(false);
    // `/` 是「搜索」不是输入：带 Ctrl / ⌘ / Alt 的组合（浏览器自己的快捷键）与按住连发都不算；
    // 已有弹层打开时也不叠开第二层搜索（焦点在弹层按钮上按 `/` 会开出叠在上面的搜索）。
    let plain = !e.ctrl_key() && !e.meta_key() && !e.alt_key() && !e.repeat();
    if !typing && plain && e.key() == "/" && !crate::focus::has_open_modal() {
      e.prevent_default();
      search_open.set(true);
    }
  });
  let search_handle = SendWrapper::new(Some(search_handle));
  on_cleanup(move || {
    if let Some(h) = search_handle.take() {
      h.remove();
    }
  });

  // Esc 关闭菜单由 `ModernMenu` 自己的 `use_escape_close` 负责（同一个 `menu_open`），这里不重复挂。
  let current = move |href: &'static str| {
    if location.pathname.get() == href {
      "page"
    } else {
      "false"
    }
  };

  view! {
    // 阅读进度条：scroll-driven animation，纯 CSS（不支持的浏览器只是不显示）。
    <div class="md-scroll-progress" aria-hidden="true"></div>
    // `data-nav`：自绘窗口滚动条从这里下沿开始（`ui/scrollbar.rs`），与像素风顶栏同一契约。
    <nav class="md-nav" data-nav aria-label=move || t("shell.main-navigation")>
      <div class="md-nav-inner">
        <a class="md-brand" href="/">
          // 站标：与像素风 `nav_bar.rs` 的 `brand-mark` 同一个用意（文字左边一枚可识别的记号），
          // 形态换成现代风的圆角方块 + 线性卫星图标 —— 不搬像素风的墨框与硬投影。
          <span class="md-brand-mark" aria-hidden="true">
            <ModernIcon kind=MIcon::Satellite class="md-brand-mark-icon" />
          </span>
          {move || t("shell.amateur-radio")}
        </a>

        <div class="md-nav-list">
          <a class="md-nav-link" href="/" aria-current=move || current("/")>
            {move || t("shell.home")}
          </a>
          <a class="md-nav-link" href="/practice" aria-current=move || current("/practice")>
            {move || t("shell.practice")}
          </a>
          <a class="md-nav-link" href="/reference" aria-current=move || current("/reference")>
            {move || t("shell.knowledge")}
          </a>
          <a class="md-nav-link" href="/tools" aria-current=move || current("/tools")>
            {move || t("shell.tools")}
          </a>
          // 768–899（平板竖屏 / 小横屏）是紧凑档：这一项收进「更多」面板
          // （面板里本来就有通联日志的入口），否则语言一长就挤到折行。
          <a
            class="md-nav-link md-nav-compact-hide"
            href="/log"
            aria-current=move || current("/log")
          >
            {move || t("shell.logbook")}
          </a>
          // 次级入口（错题 / 闪卡 / 每日挑战）收在这个面板里；它是链接行的一员，
          // 不再单独占右上角一个图标按钮（那样与链接行重复）。
          <button
            type="button"
            class="md-nav-link md-menu-anchor"
            aria-expanded=move || menu_open.get().to_string()
            on:click=move |_| menu_open.update(|v| *v = !*v)
          >
            {move || t("shell.tab-more")}
          </button>
        </div>

        <div class="md-nav-actions">
          <button
            type="button"
            class="md-text-btn md-search-trigger"
            // 可访问名必须含可见文字（WCAG 2.5.3）：按钮里只印着「/」，
            // 只报「搜索」会让语音控制用户念不出屏上的字。
            // 用空格而不是全角括号：axe 的 label-content-name-mismatch 在
            // 「搜索（/）」上仍判不匹配（括号内的符号没有按子串命中）。
            aria-label=move || format!("{} /", t("shell.search"))
            title=move || t("shell.search-with-shortcut")
            on:click=move |_| search_open.set(true)
          >
            <ModernIcon kind=MIcon::Search />
            <span class="md-search-trigger-hint">"/"</span>
          </button>
          <button
            type="button"
            class="md-text-btn md-text-btn-icon"
            aria-label=move || t("shell.settings")
            on:click=move |_| settings_open.set(true)
          >
            <crate::kit_modern::ModernIcon kind=crate::kit_modern::MIcon::Settings />
          </button>

        </div>
      </div>
    </nav>

    <crate::kit_modern::ModernDock menu_open=menu_open />

    // 目录面板里也要能打开设置：显式传 prop，不再走 `provide_context` ——
    // 它当初 provide 的是裸 `RwSignal<bool>`，与站内搜索的状态同类型，
    // `expect_context` 会把两者混成一个（切风格后顶栏搜索失效，见 `SearchOpen` 的注释）。
    <crate::kit_modern::ModernMenu open=menu_open settings_open=settings_open />

    <crate::kit_modern::ModernPalette open=palette_open />
    <ModernSettings open=settings_open />
  }
}

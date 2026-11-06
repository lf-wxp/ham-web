//! 现代风移动端底部导航（dock）。
//!
//! 为什么需要：顶栏的链接在 `<768px` 会被隐藏，之前只剩「设置 / 更多」两个图标，
//! 等于窄屏上没有主导航。现在手机改用底部 dock（5 个一级入口），顶栏只留品牌与设置。
//!
//! 768 起导航回到顶栏（平板竖屏放得下 5 项，塞进底部 dock 等于「把手机界面放大」）；
//! 900 起顶栏显示完整 6 项。dock 与顶栏导航互斥，同一个断点上只出现一套。
//! 练习页与考试页会主动收起 dock（见 `kit.css` 的 `body[data-page-mode]`，标记由 `page_mode::use_page_mode` 写）。
//!
//! 点击反馈先于重活：点一项时先把高亮挪到目标项、等这一帧画出来再导航；点「更多」时先点亮按钮、
//! 下一帧再挂目录面板（约 180 个入口，挂载本身就要几十到上百毫秒）。原因见 `after_paint`。

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_location, use_navigate};

use crate::after_paint::{after_next_paint, is_plain_click};
use crate::i18n::t;

use super::{MIcon, ModernIcon};

#[component]
pub fn ModernDock(menu_open: RwSignal<bool>) -> impl IntoView {
  let location = use_location();
  let navigate = use_navigate();
  // 已点下、还没导航完的目标：这段时间高亮跟着它走，而不是停在旧页面上。
  let pending = RwSignal::new(None::<&'static str>);
  // 「更多」已点下、目录面板还没挂上的那一帧。
  let menu_pending = RwSignal::new(false);

  let current = move |href: &'static str| {
    let active = pending
      .get()
      .map_or_else(|| location.pathname.get() == href, |target| target == href);
    if active { "page" } else { "false" }
  };

  // `Callback`（Copy）：要进四个 `on:click` 闭包；`use_navigate()` 的闭包只是 Clone。
  let go = Callback::new(move |(ev, href): (web_sys::MouseEvent, &'static str)| {
    if !is_plain_click(&ev) {
      return;
    }
    // 拦下路由器的同步导航（它看到 `default_prevented` 就不再处理），改成画完高亮再导航。
    ev.prevent_default();
    pending.set(Some(href));
    let navigate = navigate.clone();
    after_next_paint(move || {
      // 连点：后一次点击已经把目标改写成了别的页面，这一次作废 —— 不然两个页面各渲染一遍
      // （手机上各要几百毫秒），而且先到的回调还会把后一个的高亮清掉。
      // dock 已被卸载（切换风格）时信号取不到，照常导航：这一下点击不能就此丢失。
      if matches!(pending.try_get_untracked(), Some(target) if target != Some(href)) {
        return;
      }
      navigate(href, NavigateOptions::default());
      let _ = pending.try_set(None);
    });
  });

  let open_menu = move |_| {
    if menu_open.get_untracked() {
      menu_open.set(false);
      return;
    }
    // 帧前连点两下：第二下不再排一次「打开」（那会让一次双击变成「打开两次」而不是被吃掉）。
    if menu_pending.get_untracked() {
      return;
    }
    menu_pending.set(true);
    after_next_paint(move || {
      let _ = menu_open.try_set(true);
      let _ = menu_pending.try_set(false);
    });
  };

  view! {
    <nav class="md-dock" aria-label=move || t("shell.main-navigation")>
      <a
        class="md-dock-item"
        href="/"
        aria-current=move || current("/")
        on:click=move |ev| go.run((ev, "/"))
      >
        <ModernIcon kind=MIcon::Home />
        <span>{move || t("shell.home")}</span>
      </a>
      <a
        class="md-dock-item"
        href="/practice"
        aria-current=move || current("/practice")
        on:click=move |ev| go.run((ev, "/practice"))
      >
        <ModernIcon kind=MIcon::Play />
        <span>{move || t("shell.practice")}</span>
      </a>
      <a
        class="md-dock-item"
        href="/reference"
        aria-current=move || current("/reference")
        on:click=move |ev| go.run((ev, "/reference"))
      >
        <ModernIcon kind=MIcon::Book />
        <span>{move || t("shell.knowledge")}</span>
      </a>
      <a
        class="md-dock-item"
        href="/tools"
        aria-current=move || current("/tools")
        on:click=move |ev| go.run((ev, "/tools"))
      >
        <ModernIcon kind=MIcon::Tools />
        <span>{move || t("shell.tools")}</span>
      </a>
      <button
        type="button"
        class="md-dock-item"
        aria-expanded=move || (menu_open.get() || menu_pending.get()).to_string()
        on:click=open_menu
      >
        <ModernIcon kind=MIcon::Menu />
        <span>{move || t("shell.tab-more")}</span>
      </button>
    </nav>
  }
}

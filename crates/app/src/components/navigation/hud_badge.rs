//! 顶栏 HUD：一块小屏幕 —— 等级七段读数 + 经验电平格 + 连续打卡天数（`docs/ui/pixel.md` §3.4）。
//!
//! 数据由 [`crate::rpg::load_profile`] 从学习存档推算。刷新时机：路由切换（用户做完题回到别的页面）
//! 与跨标签页的 `storage` 事件；不在每次答题时刷新 —— 答题页自己有战斗 HUD，顶栏晚一步更新不影响。
//!
//! 窄屏只留「LV + 等级」：经验格在 `<md` 隐藏、连续天数在 `<lg` 隐藏，避免顶栏挤出两行。

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::i18n::{t, tf};
use crate::rpg::{Profile, load_profile, rank_label, xp_progress};
use crate::ui::{Readout, Tooltip};

/// 屏幕上经验条的格数。
const XP_CELLS: u32 = 8;

/// 顶栏玩家 HUD；点击跳到成就页（等级、徽章、统计都在那里）。
#[component]
pub fn HudBadge() -> impl IntoView {
  let location = use_location();
  let profile = RwSignal::new(load_profile());
  // 路由变化时重读：答题后用户通常会切页，此时 HUD 追上最新等级。
  Effect::new(move |_| {
    let _ = location.pathname.get();
    profile.set(load_profile());
  });

  let level = move || profile.get().level;
  let badge = move || {
    let l = level();
    if l.is_max() {
      t("shell.hud-max-level")
    } else {
      format!("LV {}", l.level)
    }
  };
  // 无障碍名以可见的「LV 1」开头（可见文字要原样出现在名字里，WCAG 2.5.3）；连续天数的完整说法
  // （「连续打卡 12 天」）也放进名字，屏幕上的「12D」在链接外面，见下。
  let label = Signal::derive(move || {
    let Profile { level, xp, streak } = profile.get();
    format!(
      "{} · {} · {xp} XP · {}",
      badge(),
      rank_label(level.level),
      tf("shell.hud-streak", &[&streak.to_string()])
    )
  });

  // 一块屏幕两段：链接只包「LV + 经验格」，连续天数在链接外面 —— 链接里的可见文字必须原样出现在
  // 它的无障碍名里（axe `label-content-name-mismatch`），「12D」放进去名字就对不上了。
  view! {
    // 高度跟同一排的控件走：窄屏是 40px 图标按钮（`h-10`），桌面档统一 36px（见 `nav_bar.rs`）。
    <span class="pxl-display flex h-10 shrink-0 items-center gap-1.5 px-1.5 xl:h-9">
      <a href="/achievements" data-slot="hud" class="flex h-full items-center gap-1.5" aria-label=move || label.get()>
        {move || {
          let l = level();
          if l.is_max() {
            view! { <span class="pxl-label text-xs">{t("shell.hud-max-level")}</span> }.into_any()
          } else {
            // 手机档的七段读数用 1 倍（顶栏的位置让给站名），`md` 起 2 倍。
            view! {
              <span class="pxl-label pxl-d-dim text-xs">"LV"</span>
              <span class="md:hidden" aria-hidden="true">
                <Readout text=format!("{:02}", l.level) scale=1 />
              </span>
              <span class="hidden md:inline-flex">
                <Readout text=format!("{:02}", l.level) label=badge() />
              </span>
            }
              .into_any()
          }
        }}
        <Tooltip label=Signal::derive(move || xp_progress(level())) class="hidden w-12 md:inline-flex">
          <span class="pxl-meter-bar w-full [--cells:8]" aria-hidden="true">
            {move || {
              let l = level();
              let lit = if l.is_max() { XP_CELLS } else { (l.percent() * XP_CELLS).div_ceil(100) };
              (0..XP_CELLS)
                .map(|i| view! { <span class="pxl-meter-seg" data-lit=(i < lit).then_some("")></span> })
                .collect_view()
            }}
          </span>
        </Tooltip>
      </a>
      <Tooltip
        label=Signal::derive(move || tf("shell.hud-streak", &[&profile.get().streak.to_string()]))
        class="hidden lg:inline-flex"
      >
        <span class="pxl-label pxl-d-hot text-xs" aria-hidden="true">
          {move || tf("shell.hud-streak-short", &[&profile.get().streak.to_string()])}
        </span>
      </Tooltip>
    </span>
  }
}

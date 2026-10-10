//! 顶栏 HUD：等级徽章 + 经验条 + 连续打卡天数。
//!
//! 数据由 [`crate::rpg::load_profile`] 从学习存档推算。刷新时机：路由切换（用户做完题回到别的页面）
//! 与跨标签页的 `storage` 事件；不在每次答题时刷新 —— 答题页自己有战斗 HUD，顶栏晚一步更新不影响。
//!
//! 窄屏只显示等级徽章：经验条与连续天数在 `<sm` 隐藏，避免顶栏挤出两行。

use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::i18n::{t, tf};
use crate::icons::{Icon, IconKind, PixelSprite};
use crate::rpg::{Profile, load_profile, rank_label, xp_progress};
use crate::ui::Progress;

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
  let label = Signal::derive(move || {
    let Profile { level, xp, .. } = profile.get();
    let lv = tf("shell.hud-level", &[&level.level.to_string()]);
    format!("{lv} · {} · {xp} XP", rank_label(level.level))
  });

  view! {
    <a
      href="/achievements"
      data-slot="hud"
      class="pxl-btn pxl-btn-ghost h-10 gap-2 px-2"
      title=move || t("shell.hud-profile")
      aria-label=move || label.get()
    >
      <PixelSprite name="hero" scale=2 class="pxl-bob" />
      <span class="hidden flex-col items-start gap-1 leading-none md:flex">
        <span class="pxl-label text-xs" aria-hidden="true">
          {move || {
            let l = level();
            if l.is_max() { t("shell.hud-max-level") } else { format!("LV {}", l.level) }
          }}
        </span>
        <span
          class="hidden w-20 md:block"
          aria-hidden="true"
          title=move || xp_progress(level())
        >
          <Progress value=Signal::derive(move || i64::from(level().percent())) class="pxl-bar-xp h-2" />
        </span>
      </span>
      <span
        class="hidden items-center gap-1 text-xs text-muted-foreground lg:flex"
        aria-hidden="true"
        title=move || tf("shell.hud-streak", &[&profile.get().streak.to_string()])
      >
        <Icon kind=IconKind::Flame class="size-6 text-destructive" />
        {move || profile.get().streak}
      </span>
    </a>
  }
}

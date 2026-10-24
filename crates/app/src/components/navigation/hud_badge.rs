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
use crate::ui::{Progress, Tooltip};

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
  // 无障碍名必须以可见的「LV 1」开头：axe 的 label-content-name-mismatch 要求可见文本
  // 原样（连续词序列）出现在名字里。连续天数的数字也算可见文本，留在链接里的话名字只会
  // 被拖成「LV 1 · 0 · …」这种读不懂的顺序，所以它挪到链接外面（见下面 `Tooltip`）。
  let label = Signal::derive(move || {
    let Profile { level, xp, streak } = profile.get();
    format!(
      "{} · {} · {xp} XP · {}",
      badge(),
      rank_label(level.level),
      tf("shell.hud-streak", &[&streak.to_string()])
    )
  });

  view! {
    <span class="flex items-center">
      <a
        href="/achievements"
        data-slot="hud"
        class="pxl-btn pxl-btn-ghost h-10 gap-2 px-2"
        aria-label=move || label.get()
      >
        <PixelSprite name="hero" scale=2 class="pxl-bob" />
        <span class="hidden flex-col items-start gap-1 leading-none md:flex">
          <span class="pxl-label text-xs" aria-hidden="true">{move || badge()}</span>
          // `hidden` 类必须挂在 Tooltip 的包装 span 上：包装 span 在窄屏只是空盒，但父级
          // `<a>` 的 `gap-2` 仍会在它与头像之间留一段空隙，把 320px 顶栏撑出 8px。
          <Tooltip
            label=Signal::derive(move || xp_progress(level()))
            class="hidden w-20 md:inline-flex"
          >
            // `w-full`：flex 项显式定宽，`pxl-bar` 的 `width: 100%` 才有确定参照。
            <span aria-hidden="true" class="w-full">
              <Progress
                value=Signal::derive(move || i64::from(level().percent()))
                class="pxl-bar-xp h-2"
              />
            </span>
          </Tooltip>
        </span>
      </a>
      // 按钮的 `px-2` 右内边距就是这里与徽章之间的间距：链接里原来隔着 `gap-2`，挪出来后
      // 换成按钮自己的内边距，视觉位置不变。
      <Tooltip
        label=Signal::derive(move || tf("shell.hud-streak", &[&profile.get().streak.to_string()]))
        class="hidden lg:inline-flex"
      >
        <span
          class="flex items-center gap-1 text-xs text-muted-foreground"
          aria-hidden="true"
        >
          <Icon kind=IconKind::Flame class="size-6 text-hp-text" />
          {move || profile.get().streak}
        </span>
      </Tooltip>
    </span>
  }
}

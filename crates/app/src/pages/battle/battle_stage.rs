//! 战斗舞台：玩家与怪物的精灵、血条、连击与受击特效。

use ham_web_core::rpg::{Battle, CRIT_COMBO, PLAYER_MAX_HP};
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::icons::PixelSprite;
use crate::ui::Progress;

use super::run::{HitFx, HitKind};

/// 舞台。
///
/// 精灵放在「随 `hit.seq` 重建」的闭包里：每回合重新创建节点才能重放 CSS 动画；
/// 血条、名字这些要平滑过渡的东西放在闭包外，只更新数值。
#[component]
pub(super) fn BattleStage(
  #[prop(into)] battle: Signal<Battle>,
  #[prop(into)] enemy_name: Signal<String>,
  enemy_sprite: &'static str,
  #[prop(into)] hit: Signal<HitFx>,
) -> impl IntoView {
  let player_hp = Signal::derive(move || i64::from(battle.get().hp_percent()));
  let enemy_hp = Signal::derive(move || i64::from(battle.get().enemy_percent()));

  // 玩家侧：受击时抖动并飘 `-1`。
  let player = move || {
    let fx = hit.get();
    let hurt = fx.kind == HitKind::Hurt;
    view! {
      <div class="relative">
        <div class=if hurt { "pxl-shake" } else { "pxl-bob" }>
          <PixelSprite name="hero" scale=5 />
        </div>
        {hurt
          .then(|| {
            view! {
              <span
                aria-hidden="true"
                class="pxl-float-up pxl-title pointer-events-none absolute -top-2 right-0 text-sm text-hp-text"
              >
                "-1"
              </span>
            }
          })}
      </div>
    }
  };

  // 怪物侧：命中时闪白并飘出伤害；经验飘在上方，两行错开不重叠。
  let enemy = move || {
    let fx = hit.get();
    let strike = match fx.kind {
      HitKind::Strike { damage, xp } => Some((damage, xp)),
      _ => None,
    };
    view! {
      <div class="relative">
        <div class=if strike.is_some() { "pxl-flash" } else { "pxl-bob" }>
          <PixelSprite name=enemy_sprite scale=5 />
        </div>
        {strike
          .map(|(damage, xp)| {
            view! {
              <span
                aria-hidden="true"
                class="pxl-float-up pxl-title pointer-events-none absolute -top-2 left-0 text-sm text-hp-text"
              >
                {format!("-{damage}")}
              </span>
              <span
                aria-hidden="true"
                class="pxl-float-up pxl-label pointer-events-none absolute -top-8 left-0 text-xs text-gold-text"
              >
                {format!("+{xp} XP")}
              </span>
            }
          })}
      </div>
    }
  };

  let combo = move || {
    let c = battle.get().combo;
    (c >= 2).then(|| {
      view! {
        <div class="pxl-blink pxl-label text-xs text-gold-text">
          {tf("rpg.combo", &[&c.to_string()])}
        </div>
        {(c >= CRIT_COMBO).then(|| view! { <div class="pxl-label text-xs">{t("rpg.crit")}</div> })}
      }
    })
  };

  view! {
    <section class="pxl-window p-3 sm:p-4" aria-label=move || t("rpg.battle")>
      <div class="grid grid-cols-[1fr_auto_1fr] items-end gap-2 sm:gap-4">
        <div class="flex min-w-0 flex-col items-start gap-2">
          <div class="pxl-label text-xs">{move || t("rpg.you")}</div>
          {player}
          <Progress
            value=player_hp
            label=Signal::derive(move || t("rpg.hp-label"))
            class="pxl-bar-win pxl-bar-danger"
          />
          <div class="text-xs tabular-nums text-muted-foreground">
            {move || tf("rpg.hp-value", &[&battle.get().hp.to_string(), &PLAYER_MAX_HP.to_string()])}
          </div>
        </div>

        <div class="flex min-w-12 flex-col items-center gap-1 pb-16 text-center" aria-live="polite">
          <span class="pxl-title text-sm" aria-hidden="true">"VS"</span>
          {combo}
        </div>

        <div class="flex min-w-0 flex-col items-end gap-2">
          <div class="pxl-label max-w-full truncate text-xs">{move || enemy_name.get()}</div>
          {enemy}
          <Progress
            value=enemy_hp
            label=Signal::derive(move || t("rpg.enemy-hp-label"))
            class="pxl-bar-hp"
          />
          <div class="text-xs tabular-nums text-muted-foreground">
            {move || {
              let b = battle.get();
              tf("rpg.enemy-hp", &[&b.enemy_hp.to_string(), &b.enemy_max.to_string()])
            }}
          </div>
        </div>
      </div>
    </section>
  }
}

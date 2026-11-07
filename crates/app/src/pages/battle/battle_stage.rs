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
  let modern = crate::kit::is_modern_at_mount();
  let player_hp = Signal::derive(move || i64::from(battle.get().hp_percent()));
  let enemy_hp = Signal::derive(move || i64::from(battle.get().enemy_percent()));

  // 玩家侧：受击时抖动并飘 `-1`。
  let player = move || {
    let fx = hit.get();
    let hurt = fx.kind == HitKind::Hurt;
    // 两套皮肤各有一份「受击 / 待机」动效，类名不共用。
    let fx_class = if modern {
      if hurt { "md-mon-hit" } else { "md-mon-idle" }
    } else if hurt {
      "pxl-shake"
    } else {
      "pxl-bob"
    };
    view! {
      <div class="relative">
        <div class=fx_class>
          {if modern {
            view! { <crate::kit_modern::ModernMonster sprite="hero" size=84 /> }.into_any()
          } else {
            view! { <PixelSprite name="hero" scale=5 /> }.into_any()
          }}
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
    let fx_class = if modern {
      if strike.is_some() {
        "md-mon-hit"
      } else {
        "md-mon-idle"
      }
    } else if strike.is_some() {
      "pxl-flash"
    } else {
      "pxl-bob"
    };
    view! {
      <div class="relative">
        <div class=fx_class>
          // 精灵与外层动效类同按挂载快照派发：只让其中一个跟着信号走，切皮肤时就会出现
          // 「新皮肤的精灵套着旧皮肤的动效类」（整个舞台的外壳、血条也都是快照）。
          {if modern {
            view! { <crate::kit_modern::ModernMonster sprite=enemy_sprite size=84 /> }.into_any()
          } else {
            view! { <PixelSprite name=enemy_sprite scale=5 /> }.into_any()
          }}
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
        <div class=if modern { "md-combo" } else { "pxl-blink pxl-label text-xs text-gold-text" }>
          {tf("rpg.combo", &[&c.to_string()])}
        </div>
        {(c >= CRIT_COMBO)
          .then(|| {
            view! {
              <div class=if modern { "md-combo-crit" } else { "pxl-label text-xs" }>
                {t("rpg.crit")}
              </div>
            }
          })}
      }
    })
  };

  view! {
    <section
      class=if modern { "md-battle-stage" } else { "pxl-window p-3 sm:p-4" }
      aria-label=move || t("rpg.battle")
    >
      <div class="grid grid-cols-[1fr_auto_1fr] items-end gap-2 sm:gap-4">
        <div class="flex min-w-0 flex-col items-start gap-2">
          <div class="pxl-label text-xs">{move || t("rpg.you")}</div>
          {player}
          {move || {
            if modern {
              let pct = player_hp.get().clamp(0, 100);
              let low = pct <= 30;
              view! {
                <div class="md-hp-track" data-low=low.to_string()>
                  <span class="md-hp-fill" style=format!("width: {pct}%")></span>
                </div>
              }
                .into_any()
            } else {
              view! {
                <Progress
                  value=player_hp
                  label=Signal::derive(move || t("rpg.hp-label"))
                  class="pxl-bar-win pxl-bar-danger"
                />
              }
                .into_any()
            }
          }}
          <div class="text-xs tabular-nums text-muted-foreground">
            {move || tf("rpg.hp-value", &[&battle.get().hp.to_string(), &PLAYER_MAX_HP.to_string()])}
          </div>
        </div>

        <div class="flex min-w-12 flex-col items-center gap-1 pb-16 text-center" aria-live="polite">
          <span class=if modern { "md-vs" } else { "pxl-title text-sm" } aria-hidden="true">"VS"</span>
          {combo}
        </div>

        <div class="flex min-w-0 flex-col items-end gap-2">
          <div class="pxl-label max-w-full truncate text-xs">{move || enemy_name.get()}</div>
          {enemy}
          {move || {
            if modern {
              let pct = enemy_hp.get().clamp(0, 100);
              view! {
                <div class="md-hp-track md-hp-enemy">
                  <span class="md-hp-fill" style=format!("width: {pct}%")></span>
                </div>
              }
                .into_any()
            } else {
              view! {
                <Progress
                  value=enemy_hp
                  label=Signal::derive(move || t("rpg.enemy-hp-label"))
                  class="pxl-bar-hp"
                />
              }
                .into_any()
            }
          }}
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

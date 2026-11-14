//! 抗干扰舞台（像素风）：本台屏（电台 + 电量格 + 连对读数）与干扰屏（QRM 强度 + 频谱）两块屏幕。
//!
//! 对手是一块屏幕上的干扰频谱（`QrmSignature`，每个分类一种真实干扰现象），本台的生命是电量格
//! （`docs/ui/decisions.md` §11.6）。现代风仍是矢量怪物与血条，在 `battle_stage.rs`；
//! 两套各一个文件，调用点只在那里按皮肤派发。

use ham_web_core::rig_panel::QrmKind;
use ham_web_core::rpg::{Battle, CRIT_COMBO, PLAYER_MAX_HP};
use leptos::prelude::*;

use crate::components::radio::{Battery, QrmSignature, qrm_name};
use crate::i18n::{t, tf};
use crate::icons::PixelSprite;
use crate::ui::Readout;

use super::run::{HitFx, HitKind};

#[component]
pub(super) fn PixelBattleStage(
  #[prop(into)] battle: Signal<Battle>,
  #[prop(into)] enemy_name: Signal<String>,
  qrm: QrmKind,
  #[prop(into)] hit: Signal<HitFx>,
) -> impl IntoView {
  let combo = move || {
    let c = battle.get().combo;
    (c >= 2).then(|| {
      view! {
        <div class="pxl-blink pxl-label text-xs text-gold-text">{tf("rpg.combo", &[&c.to_string()])}</div>
        {(c >= CRIT_COMBO)
          .then(|| view! { <div class="pxl-label text-xs">{t("rpg.crit")}</div> })}
      }
    })
  };
  let lv = move || {
    let b = battle.get();
    if b.enemy_hp == 0 {
      t("rpg.qrm-clear")
    } else {
      tf("rpg.qrm-level", &[&b.enemy_hp.to_string()])
    }
  };
  view! {
    <section class="space-y-2" aria-label=move || t("rpg.battle")>
      <div class="grid gap-2 sm:grid-cols-2">
        <div class="pxl-display flex items-center gap-3 p-2.5">
          <div class="relative shrink-0">
            {move || {
              let hurt = hit.get().kind == HitKind::Hurt;
              view! {
                <div class=if hurt { "pxl-shake" } else { "" }>
                  <PixelSprite name="radio" scale=3 />
                </div>
                {hurt.then(|| view! {
                  <span aria-hidden="true" class="pxl-float-up pxl-title pointer-events-none absolute -top-2 right-0 text-sm pxl-d-alarm">"-1"</span>
                })}
              }
            }}
          </div>
          <div class="min-w-0 space-y-1.5">
            <span class="pxl-label pxl-d-dim block text-xs">{move || t("rpg.hp-label")}</span>
            <Battery
              level=Signal::derive(move || battle.get().hp)
              max=PLAYER_MAX_HP
              label=Signal::derive(move || t("rpg.hp-label"))
            />
            <span class="flex items-center gap-1.5">
              <span class="pxl-label pxl-d-dim text-xs">{move || t("rpg.combo-label")}</span>
              <Readout text=Signal::derive(move || battle.get().combo.to_string()) />
            </span>
            <span class="pxl-d-dim block text-xs tabular-nums">
              {move || tf("rpg.hp-value", &[&battle.get().hp.to_string(), &PLAYER_MAX_HP.to_string()])}
            </span>
          </div>
        </div>
        <div class="pxl-display relative space-y-1.5 p-2.5">
          <div class="flex items-center justify-between gap-2">
            <span class="pxl-label pxl-d-dim text-xs">{move || t("learning.rig-qrm")}</span>
            <span class=move || if battle.get().enemy_hp == 0 { "pxl-label pxl-d-ok text-xs" } else { "pxl-label pxl-d-hot text-xs" }>{lv}</span>
          </div>
          {move || {
            let fx = hit.get();
            let strike = match fx.kind {
              HitKind::Strike { damage, xp } => Some((damage, xp)),
              _ => None,
            };
            view! {
              <div class=if strike.is_some() { "pxl-flash" } else { "" }>
                // 干扰强度跟着剩余「血量」走：每答对一次矮一截，消除时只剩底噪。
                <QrmSignature
                  kind=qrm
                  strength=Signal::derive(move || f64::from(battle.get().enemy_percent()) / 100.0)
                  cols=16
                  animate=true
                />
              </div>
              {strike.map(|(damage, xp)| view! {
                <span aria-hidden="true" class="pxl-float-up pxl-title pointer-events-none absolute top-6 right-3 text-sm pxl-d-alarm">{format!("-{damage}")}</span>
                <span aria-hidden="true" class="pxl-float-up pxl-label pointer-events-none absolute top-0 right-3 text-xs pxl-d-hot">{format!("+{xp} XP")}</span>
              })}
            }
          }}
          <span class="block truncate text-xs">{move || qrm_name(qrm)}</span>
          <span class="pxl-d-dim block truncate text-xs tabular-nums">
            {move || {
              let b = battle.get();
              format!("{} · {}", enemy_name.get(), tf("rpg.enemy-hp", &[&b.enemy_hp.to_string(), &b.enemy_max.to_string()]))
            }}
          </span>
        </div>
      </div>
      <div class="flex min-h-5 items-center justify-center gap-2 text-center" aria-live="polite">{combo}</div>
    </section>
  }
}

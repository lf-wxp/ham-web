//! 战斗结算窗口：胜负、星级、经验、升级与下一步去哪。

use ham_web_core::Bank;
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::icons::PixelSprite;
use crate::pages::bank_href;
use crate::ui::{Button, ButtonLink, Size, Variant};
use crate::util::encode_uri_component;

use super::result_row::ResultRow;

/// 一局战斗的结算数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Settlement {
  pub(super) victory: bool,
  /// 闯关才有星级；复仇模式为 `None`。
  pub(super) stars: Option<u8>,
  pub(super) new_record: bool,
  pub(super) xp: u32,
  pub(super) best_combo: u32,
  pub(super) correct: u32,
  pub(super) wrong: u32,
  pub(super) defeated: u32,
  /// `(开战前等级, 开战后等级)`；没升级时两者相等。
  pub(super) levels: (u32, u32),
  /// 闯关通关后的下一关 `(key, 名称)`。
  pub(super) next: Option<(String, &'static str)>,
  pub(super) revenge: bool,
}

/// 结算窗口。
#[component]
pub(super) fn BattleResult(
  settlement: Settlement,
  bank: Bank,
  version: Option<String>,
  on_retry: Callback<()>,
) -> impl IntoView {
  let s = settlement;
  let map_href = bank_href("/map", version.as_deref(), bank);
  let next_href = s.next.as_ref().map(|(key, _)| {
    format!(
      "{}&stage={}",
      bank_href("/battle", version.as_deref(), bank),
      encode_uri_component(key)
    )
  });
  let leveled = s.levels.1 > s.levels.0;
  let stars = s.stars.unwrap_or(0);

  view! {
    <section class="pxl-window motion-pop space-y-4 p-4 sm:p-6" aria-labelledby="battle-result-title">
      <div class="flex flex-col items-center gap-2 text-center">
        <PixelSprite
          name=if s.victory { "badge_trophy" } else { "badge_lock" }
          scale=6
          class="pxl-bob"
        />
        <h2 id="battle-result-title" class="pxl-title text-base">
          {move || {
            match (s.victory, s.revenge) {
              (true, true) => t("rpg.revenge-win"),
              (true, false) => t("rpg.stage-clear"),
              (false, _) => t("rpg.defeat"),
            }
          }}
        </h2>
        {s
          .stars
          .filter(|_| s.victory)
          .map(|_| {
            view! {
              <div
                class="flex items-center gap-1"
                role="img"
                aria-label=move || tf("rpg.stars-value", &[&stars.to_string()])
              >
                {(1..=3u8)
                  .map(|n| {
                    view! {
                      <PixelSprite
                        name="badge_star"
                        scale=3
                        class=if n <= stars { "" } else { "opacity-25 grayscale" }
                      />
                    }
                  })
                  .collect_view()}
              </div>
            }
          })}
        {(s.new_record && s.victory)
          .then(|| view! { <div class="pxl-label pxl-blink text-xs">{t("rpg.new-record")}</div> })}
        {leveled
          .then(|| {
            view! {
              <div class="pxl-label pxl-blink text-sm text-gold-text">
                {tf("rpg.level-up", &[&s.levels.0.to_string(), &s.levels.1.to_string()])}
              </div>
            }
          })}
      </div>

      <dl class="mx-auto max-w-sm">
        <ResultRow label=Signal::derive(move || t("rpg.xp-earned")) value=format!("+{} XP", s.xp) />
        <ResultRow label=Signal::derive(move || t("rpg.best-combo")) value=s.best_combo.to_string() />
        <ResultRow label=Signal::derive(move || t("rpg.correct-count")) value=s.correct.to_string() />
        <ResultRow label=Signal::derive(move || t("rpg.wrong-count")) value=s.wrong.to_string() />
        {(s.defeated > 0 || s.revenge)
          .then(|| {
            view! {
              <ResultRow
                label=Signal::derive(move || t("rpg.monsters-defeated"))
                value=s.defeated.to_string()
              />
            }
          })}
      </dl>

      <div class="flex flex-wrap items-center justify-center gap-3">
        {next_href
          .zip(s.next.as_ref().map(|(_, name)| *name))
          .filter(|_| s.victory)
          .map(|(href, name)| {
            view! {
              <ButtonLink href=href variant=Variant::Default size=Size::Lg>
                {move || tf("rpg.next-stage", &[&t(name)])}
              </ButtonLink>
            }
          })}
        <Button variant=Variant::Secondary size=Size::Lg on_click=on_retry>
          {move || t("rpg.retry")}
        </Button>
        <ButtonLink
          href=if s.revenge { "/bestiary".to_owned() } else { map_href }
          variant=Variant::Outline
          size=Size::Lg
        >
          {move || if s.revenge { t("rpg.to-bestiary") } else { t("rpg.back-to-map") }}
        </ButtonLink>
      </div>
    </section>
  }
}

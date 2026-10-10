//! 基地首屏：角色、等级、经验条与三个主入口。

use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::icons::PixelSprite;
use crate::rpg::{load_profile, rank_label, xp_progress};
use crate::ui::{ButtonLink, Progress, Size, Variant};

/// 基地首屏。
#[component]
pub(super) fn BaseHero() -> impl IntoView {
  let profile = load_profile();
  let level = profile.level;
  let percent = i64::from(level.percent());

  view! {
    <section class="relative isolate" aria-labelledby="home-title">
      <div class="hero-grid" aria-hidden="true"></div>
      <div class="pxl-window motion-pop flex flex-col items-center gap-5 p-5 text-center sm:flex-row sm:p-6 sm:text-left">
        <div class="flex shrink-0 flex-col items-center gap-2">
          <PixelSprite name="hero" scale=8 class="pxl-bob" />
          <span class="pxl-label text-xs text-muted-foreground">
            {move || format!("{} · {}", tf("shell.hud-level", &[&level.level.to_string()]), rank_label(level.level))}
          </span>
        </div>

        <div class="min-w-0 flex-1 space-y-4">
          <div class="space-y-2">
            <span class="pxl-label text-xs text-muted-foreground">{move || t("knowledge.amateur-radio")}</span>
            <h1 id="home-title" class="pxl-title text-[clamp(1.1rem,5.5vw,2rem)] leading-tight">
              {move || t("shell.amateur-radio")}
            </h1>
            <p class="text-sm text-muted-foreground">{move || t("home.one-platform-for-exam")}</p>
          </div>

          <div class="space-y-1">
            <div class="flex items-baseline justify-between gap-2 text-xs">
              <span class="pxl-label">{move || t("rpg.exp-label")}</span>
              <span class="tabular-nums text-muted-foreground">{move || xp_progress(level)}</span>
            </div>
            <Progress value=percent label=Signal::derive(move || t("rpg.exp-label")) class="pxl-bar-xp" />
            {(!level.is_max())
              .then(|| {
                view! {
                  <p class="text-xs text-muted-foreground">
                    {move || tf("rpg.level-progress", &[&(level.needed - level.into_level).to_string()])}
                  </p>
                }
              })}
          </div>

          <div class="flex flex-wrap justify-center gap-3 sm:justify-start">
            <ButtonLink href="/map" variant=Variant::Default size=Size::Lg>
              {move || t("rpg.to-stage")}
            </ButtonLink>
            <ButtonLink href="/exam" variant=Variant::Secondary size=Size::Lg>
              {move || t("rpg.to-boss")}
            </ButtonLink>
            <ButtonLink href="/bestiary" variant=Variant::Outline size=Size::Lg>
              {move || t("rpg.bestiary")}
            </ButtonLink>
          </div>
        </div>
      </div>
    </section>
  }
}

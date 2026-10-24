//! 基地首屏：角色、等级、经验条与三个主入口。

use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::icons::PixelSprite;
use crate::rpg::{load_profile, rank_label};
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
          // 等级牌做成一块带框的铭牌，而不是又一行灰色小字：它是首屏里唯一的「我是谁」。
          <span class="pxl-badge bg-secondary text-xs">
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
              // 只写数字：`xp_progress` 把「经验」也带在句子里，与左边的标签重复，
              // 英文下更明显（左边 EXP、右边 XP 5 / 80 各说一遍）。
              <span class="tabular-nums text-muted-foreground">
                {move || {
                  if level.is_max() {
                    t("shell.hud-max-level")
                  } else {
                    format!("{} / {}", level.into_level, level.needed)
                  }
                }}
              </span>
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

          // 首屏只有一个最重的主操作：「出去闯关」。以前这里并排三个 `Lg` 按钮，其中「挑战 Boss」
          // （/exam）与下面「快速开始」里的「开始模拟考试」是同一处去向，两块 L2 窗口里各摆一遍，
          // 视线没有落点（`docs/ui-components.md` §2 硬规则 8）。模拟考试留给带题库选择的快速开始。
          <div class="flex flex-wrap items-center justify-center gap-3 sm:justify-start">
            <ButtonLink href="/map" variant=Variant::Default size=Size::Lg>
              {move || t("rpg.to-stage")}
            </ButtonLink>
            <ButtonLink href="/bestiary" variant=Variant::Ghost size=Size::Default>
              {move || t("rpg.bestiary")}
            </ButtonLink>
          </div>
          // 入口用 RPG 隐喻命名（闯关 / 图鉴）：老用户一眼懂，新用户不懂。
          // 补一行白话对照，比把按钮改成「开始练习」更能保住这套世界观。
          <p class="text-xs text-muted-foreground">{move || t("home.rpg-entry-legend")}</p>
        </div>
      </div>
    </section>
  }
}

//! 通联路线顶部的「下一站」：推荐接下来该通联哪一站（像素风画成那一站的干扰频谱）。

use ham_web_core::Bank;
use ham_web_core::rig_panel::qrm_kind;
use ham_web_core::rpg::Stage;
use leptos::prelude::*;

use crate::components::radio::{QrmSignature, qrm_name};
use crate::i18n::{t, tp};
use crate::icons::PixelSprite;
use crate::pages::bank_href;
use crate::rpg::monster_sprite;
use crate::ui::{ButtonLink, Size, Variant};
use crate::util::encode_uri_component;

/// 下一个任务；`stage` 为 `None` 表示整张地图都通关了，改为引导去打 Boss。
#[component]
pub(super) fn NextQuest(
  stage: Option<Stage>,
  bank: Bank,
  version: Option<String>,
) -> impl IntoView {
  match stage {
    Some(stage) => {
      let battle = format!(
        "{}&stage={}",
        bank_href("/battle", version.as_deref(), bank),
        encode_uri_component(stage.key)
      );
      let practice = format!(
        "{}&topic={}",
        bank_href("/practice", version.as_deref(), bank),
        encode_uri_component(stage.key)
      );
      if crate::kit::is_modern_at_mount() {
        return view! {
          <section class="md-next-quest" aria-labelledby="next-quest-title">
            <crate::kit_modern::ModernMonster sprite=monster_sprite(stage.key) size=72 />
            <div class="md-next-quest-body">
              <p class="md-next-quest-label">{move || t("rpg.next-quest")}</p>
              <h2 id="next-quest-title" class="md-next-quest-title">{move || t(stage.name)}</h2>
              <p class="md-next-quest-desc">{move || t(stage.desc)}</p>
              <p class="md-next-quest-meta">
                {move || tp("rpg.stage-questions", stage.count, &[&stage.count.to_string()])}
              </p>
            </div>
            <div class="md-next-quest-actions">
              <a class="md-btn md-btn-accent" href=battle.clone()>
                {move || t("rpg.start-battle")}
              </a>
              <a class="md-btn md-btn-ghost" href=practice.clone()>
                {move || t("rpg.practice-stage")}
              </a>
            </div>
          </section>
        }
        .into_any();
      }
      view! {
        <section class="pxl-window flex flex-wrap items-center gap-4 p-4" aria-labelledby="next-quest-title">
          // 这一站的干扰信号（每个分类一种真实干扰现象）；名字写在屏幕下沿，颜色不是唯一通道。
          <div class="pxl-display flex shrink-0 flex-col items-center gap-1 p-2">
            <QrmSignature kind=qrm_kind(stage.key) animate=true />
            <span class="pxl-d-dim text-xs">{move || qrm_name(qrm_kind(stage.key))}</span>
          </div>
          // `basis-56`（14rem）而不是 `flex-1`（`basis: 0%`）：这一行里有 `flex-shrink: 0`
          // 的精灵与两个 `Size::Lg` 按钮，`flex-1` 的文字块可以被压到 0 宽 —— 不会触发
          // `flex-wrap`（flex 认为放得下），中文于是逐字竖排成一列（390px 下实测 1 字宽）。
          // 给一个真实的首选宽度，放不下时才会换行。
          <div class="min-w-0 grow basis-56 space-y-1">
            <div class="pxl-label text-xs text-muted-foreground">{move || t("rpg.next-quest")}</div>
            <h2 id="next-quest-title" class=crate::kit::section_head_class()>{move || t(stage.name)}</h2>
            <p class="text-sm text-muted-foreground">{move || t(stage.desc)}</p>
            <p class="text-xs text-muted-foreground">
              {move || tp("rpg.stage-questions", stage.count, &[&stage.count.to_string()])}
            </p>
          </div>
          <div class="flex flex-wrap gap-2">
            <ButtonLink href=battle variant=Variant::Default size=Size::Lg>
              {move || t("rpg.start-battle")}
            </ButtonLink>
            <ButtonLink href=practice variant=Variant::Outline size=Size::Lg>
              {move || t("rpg.practice-stage")}
            </ButtonLink>
          </div>
        </section>
      }
      .into_any()
    }
    None => {
      let exam = bank_href("/exam", version.as_deref(), bank);
      if crate::kit::is_modern_at_mount() {
        return view! {
          <section class="md-next-quest" aria-labelledby="next-quest-title">
            <crate::kit_modern::ModernEmblem
              icon=crate::kit_modern::MIcon::Trophy
              tone=crate::kit_modern::EmblemTone::Gold
              size=72
              locked=Signal::derive(|| false)
            />
            <div class="md-next-quest-body">
              <h2 id="next-quest-title" class="md-next-quest-title">
                {move || t("rpg.all-cleared")}
              </h2>
              <p class="md-next-quest-desc">{move || t("rpg.all-cleared-hint")}</p>
            </div>
            <div class="md-next-quest-actions">
              <a class="md-btn md-btn-accent" href=exam.clone()>
                {move || t("rpg.to-boss")}
              </a>
            </div>
          </section>
        }
        .into_any();
      }
      view! {
        <section class="pxl-window flex flex-wrap items-center gap-4 p-4" aria-labelledby="next-quest-title">
          <PixelSprite name="boss" scale=6 class="pxl-bob" />
          <div class="min-w-0 grow basis-56 space-y-1">
            <h2 id="next-quest-title" class=crate::kit::section_head_class()>{move || t("rpg.all-cleared")}</h2>
            <p class="text-sm text-muted-foreground">{move || t("rpg.all-cleared-hint")}</p>
          </div>
          <ButtonLink href=exam variant=Variant::Default size=Size::Lg>
            {move || t("rpg.to-boss")}
          </ButtonLink>
        </section>
      }
      .into_any()
    }
  }
}

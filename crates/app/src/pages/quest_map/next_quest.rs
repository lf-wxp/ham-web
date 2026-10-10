//! 地图顶部的「下一个任务」：推荐接下来该打哪一关。

use ham_web_core::Bank;
use ham_web_core::rpg::Stage;
use leptos::prelude::*;

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
      view! {
        <section class="pxl-window flex flex-wrap items-center gap-4 p-4" aria-labelledby="next-quest-title">
          <PixelSprite name=monster_sprite(stage.key) scale=6 class="pxl-bob" />
          <div class="min-w-0 flex-1 space-y-1">
            <div class="pxl-label text-xs text-muted-foreground">{move || t("rpg.next-quest")}</div>
            <h2 id="next-quest-title" class="pxl-title text-sm">{move || t(stage.name)}</h2>
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
      view! {
        <section class="pxl-window flex flex-wrap items-center gap-4 p-4" aria-labelledby="next-quest-title">
          <PixelSprite name="boss" scale=6 class="pxl-bob" />
          <div class="min-w-0 flex-1 space-y-1">
            <h2 id="next-quest-title" class="pxl-title text-sm">{move || t("rpg.all-cleared")}</h2>
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

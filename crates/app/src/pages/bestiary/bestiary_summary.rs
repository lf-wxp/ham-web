//! 图鉴汇总：在世 / 已击败 / 到期三个读数，以及图鉴进度条。

use ham_web_core::rpg::bestiary_percent;
use leptos::prelude::*;

use crate::i18n::{t, tf};
use crate::ui::{Progress, Stat};

/// 汇总面板。
#[component]
pub(super) fn BestiarySummary(alive: usize, defeated: usize, due: usize) -> impl IntoView {
  let percent = bestiary_percent(defeated, alive);
  view! {
    <section class="space-y-3" aria-label=move || t("rpg.dex-progress")>
      <div class="grid grid-cols-3 gap-3">
        <Stat label=t("rpg.alive") value=alive />
        <Stat label=t("rpg.defeated") value=defeated />
        <Stat label=t("rpg.due-label") value=due />
      </div>
      <div class="space-y-1">
        <div class="flex items-baseline justify-between text-xs text-muted-foreground">
          <span class="pxl-label">{move || t("rpg.dex-progress")}</span>
          <span class="tabular-nums">{move || tf("rpg.dex-percent", &[&percent.to_string()])}</span>
        </div>
        <Progress
          value=i64::from(percent)
          label=Signal::derive(move || t("rpg.dex-progress"))
          class="pxl-bar-gold"
        />
      </div>
    </section>
  }
}

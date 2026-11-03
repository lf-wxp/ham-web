//! 学习记录：四个读数 + 最近 7 天的信号瀑布，合成一个章节。
//!
//! 读数条原来紧跟首屏，进首页第一眼是「报表」；现在它退到行动清单之后，
//! 与自己的可视化（瀑布）放在同一章里 —— 「我今天做了什么」比「我累计多少」更该先看到。

use leptos::prelude::*;

use crate::i18n::t;

use super::stat::ModernStatValue;
use super::waterfall::Waterfall;

#[component]
pub(super) fn Record() -> impl IntoView {
  let stats = crate::study::load_stats();
  let answered = stats.total.answered;
  let accuracy = (answered > 0).then(|| stats.total.correct * 100 / answered);
  let profile = crate::rpg::load_profile();
  let streak = profile.streak as usize;
  let xp = profile.xp as usize;

  view! {
    <section aria-labelledby="home-record-title">
      <h2 id="home-record-title" class="md-h2">
        {move || t("home.record-title")}
      </h2>

      // 这一条横线是本节的内部刻度（章节顶部已经有一条墨线），所以去掉自己的上边线。
      <div class="md-strip md-strip-flat">
        <div class="md-strip-item">
          <ModernStatValue value=Signal::stored(Some(answered as usize)) />
          <span class="md-strip-label">{move || t("home.answered")}</span>
        </div>
        <div class="md-strip-item">
          <ModernStatValue value=Signal::stored(accuracy.map(|p| p as usize)) suffix="%" />
          <span class="md-strip-label">{move || t("learning.accuracy")}</span>
        </div>
        <div class="md-strip-item">
          <ModernStatValue value=Signal::stored(Some(streak)) />
          <span class="md-strip-label">{move || t("home.streak-days-label")}</span>
        </div>
        <div class="md-strip-item">
          <ModernStatValue value=Signal::stored(Some(xp)) />
          <span class="md-strip-label">{move || t("rpg.exp-label")}</span>
        </div>
      </div>

      <a class="md-strip-link" href="/progress">
        {move || t("shell.progress")}
        <crate::kit_modern::ModernIcon kind=crate::kit_modern::MIcon::ArrowRight class="size-4" />
      </a>

      <Waterfall />
    </section>
  }
}

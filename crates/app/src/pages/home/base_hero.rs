//! 首页主显示屏（`docs/ui/pixel.md` §3.4）：电台 + 操作员信息、经验格、备考信号（S 表，带峰值保持）、
//! 值机记录与主入口。站名与介绍文案不上屏：第一屏只给「我现在在哪、下一步做什么」。

use ham_web_core::Bank;
use ham_web_core::exam_history::assess;
use ham_web_core::rig_panel::{S9, main_bank_pass, s_reading, s_units};
use leptos::prelude::*;

use crate::components::radio::SignalMeter;
use crate::data::kt_tracked;
use crate::i18n::{bank_class, t, tf, tp};
use crate::icons::PixelSprite;
use crate::rpg::{load_profile, rank_label};
use crate::ui::{ButtonLink, LedTone, Readout, ReadoutTone, Size, Variant};

use super::log_strip::LogStrip;

/// 经验条的格数：屏幕上的电平格，不是平滑进度条。
const XP_CELLS: u32 = 16;

/// 首页主显示屏。数据都是同步读本地存档，进首页时算一次。
#[component]
pub(super) fn BaseHero() -> impl IntoView {
  let profile = load_profile();
  let level = profile.level;
  let percent = level.percent();
  // 经验格向上取整：有一点经验就亮一格，「刚升级」与「差一点升级」才分得开两头。
  let xp_lit = if level.is_max() {
    XP_CELLS
  } else {
    (percent * XP_CELLS).div_ceil(100)
  };

  // 备考信号：作答最多的题库，最近几次常规模拟考试的平均分折成 S 表（S9 = 合格线）；
  // 峰值保持是这几次里最好的一次。
  let (bank, pass) = main_bank_pass(crate::study::stats_shared().as_deref());
  let readiness = assess(&crate::exam_history::load(), bank);
  let units = readiness.as_ref().map(|r| s_units(r.average / 100.0, pass));
  let peak = readiness.as_ref().map(|r| s_units(r.best / 100.0, pass));
  let bank_label = bank_name(bank);
  // 操作员信息的第三行：离合格线还差几格（S9 = 合格线）。
  let gap_line = move || match units {
    Some(u) if u < S9 => {
      let n = u32::from(S9 - u);
      tp("home.s-gap", n, &[&n.to_string()])
    }
    Some(_) => t("home.s-pass"),
    None => t("home.s-none"),
  };

  view! {
    <section class="pxl-window motion-pop" aria-labelledby="home-title">
      // 站名不上屏，但页面仍要有一个 h1（读屏的页面大纲从它开始）。
      <h1 id="home-title" class="sr-only">{move || t("shell.amateur-radio")}</h1>
      <h2 class="pxl-section-title">
        {move || t("home.main-display")}
        <span class="pxl-label ml-auto text-xs text-muted-foreground" aria-hidden="true">
          {move || t("home.main-code")}
        </span>
      </h2>
      <div class="space-y-4 p-4 sm:p-5">
        <div class="flex items-center gap-3">
          <div class="pxl-display shrink-0 p-2" aria-hidden="true">
            <PixelSprite name="radio" scale=3 />
          </div>
          <div class="min-w-0 space-y-1">
            <span class="pxl-label block text-xs text-muted-foreground">
              {move || tf("home.operator-tag", &[&level.level.to_string()])}
            </span>
            <span class="pxl-title block text-xs leading-normal">{move || rank_label(level.level)}</span>
            <span class="block text-xs text-muted-foreground">{gap_line}</span>
          </div>
        </div>

        // 主屏幕：经验格、备考信号、值机记录三组读数一块屏（一屏最多两块屏幕：另一块是上面的电台小屏）。
        <div class="pxl-display space-y-3 p-3">
          <div class="space-y-1">
            <span class="flex flex-wrap items-baseline justify-between gap-x-2 text-xs">
              <span class="pxl-label">{move || t("rpg.exp-label")}</span>
              <span class="pxl-d-dim tabular-nums">
                {move || {
                  if level.is_max() {
                    t("shell.hud-max-level")
                  } else {
                    format!("{} / {}", level.into_level, level.needed)
                  }
                }}
              </span>
            </span>
            <span
              class="pxl-meter-bar [--cells:16]"
              role="progressbar"
              aria-label=move || t("rpg.exp-label")
              aria-valuemin="0"
              aria-valuemax="100"
              aria-valuenow=percent.to_string()
            >
              {(0..XP_CELLS)
                .map(|i| view! { <span class="pxl-meter-seg" data-lit=(i < xp_lit).then_some("")></span> })
                .collect_view()}
            </span>
          </div>

          <div class="space-y-2 border-t-2 border-dashed border-(--display-rim) pt-3">
            <div class="flex flex-wrap items-center justify-between gap-2">
              <span class="pxl-label pxl-d-dim text-xs">
                {move || format!("{} · {}", t("home.readiness-signal"), bank_label())}
              </span>
              <Readout
                text=units.map_or_else(|| "--".to_owned(), s_reading)
                label=Signal::derive(move || {
                  units.map_or_else(|| t("common.no-signal"), s_reading)
                })
                tone=ReadoutTone::for_s_units(units)
              />
            </div>
            <SignalMeter
              units=units
              peak=peak
              label=Signal::derive(move || t("home.readiness-signal"))
            />
            <p class="pxl-d-dim text-xs">
              {move || match &readiness {
                Some(r) => {
                  format!(
                    "{} · {} · {}",
                    kt_tracked(r.verdict.label()),
                    tp(
                      "home.readiness-average",
                      r.count,
                      &[&r.count.to_string(), &format!("{:.0}", r.average)],
                    ),
                    t("home.s9-pass-line"),
                  )
                }
                None => tf("home.readiness-none", &[&bank_label()]),
              }}
            </p>
          </div>
          <LogStrip />
        </div>

        // 面板里只有一个主操作（发射键带绿灯）：出发通联。模拟考试留给下面带题库选择的「快速开始」。
        <div class="flex flex-wrap items-center gap-3">
          <ButtonLink href="/map" variant=Variant::Default size=Size::Lg led=LedTone::Rx led_on=true>
            {move || t("rpg.to-stage")}
          </ButtonLink>
          <ButtonLink href="/bestiary" variant=Variant::Ghost size=Size::Default>
            {move || t("rpg.bestiary")}
          </ButtonLink>
        </div>
        // 入口用电台隐喻命名，补一行白话对照，新用户也知道点进去是什么。
        <p class="text-xs text-muted-foreground">{move || t("home.rpg-entry-legend")}</p>
      </div>
    </section>
  }
}

/// 题库的显示名（「A 类」）：闭包里调用，切语言时跟着变。
fn bank_name(bank: Bank) -> impl Fn() -> String + Copy {
  move || bank_class(bank.as_str())
}

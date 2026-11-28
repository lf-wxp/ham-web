//! 现代风首页：电波带 → 今日开台清单 → 学习记录 → 实时传播 → 题库版本 → 快速入口 → 模块目录 → 收尾 CTA。
//!
//! 与像素版首页是**两份独立的页面实现**，共用同一份数据源（`rpg::load_profile`、
//! `study::load_stats`、`ham_web_core::registry`、i18n 词条）。像素版是「基地 + 任务板 +
//! 频谱装饰」，这里是面向个人的操作台：先给主张，再给今天要做的三件事，
//! 然后才是累计读数与目录 —— 首页的职责是「让人立刻开始」，不是「汇报总量」。

use ham_web_core::Bank;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::i18n::{t, tf};
use crate::pages::DEFAULT_TITLE;
use crate::ui::push_info;
use crate::util::set_title;

mod bank_picker;
mod bento;
mod hero;
mod live;
mod live_band;
mod live_metric;
mod module_grid;
mod onair;
mod onair_row;
mod record;
mod stat;
mod waterfall;

pub use stat::ModernStatValue;

use bank_picker::ModernBankPicker;
use bento::Bento;
use hero::ModernHero;
use live::Live;
use module_grid::ModuleGrid;
use onair::OnAir;
use record::Record;

#[component]
pub fn ModernHome() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  // 模块分组渲染 `crates/core` 的中文原文（`core_text_tracked`），译文词典要自己拉一次。
  crate::data::use_knowledge_i18n();

  // 滚动浮现：复用像素风那套 IntersectionObserver（`motion::reveal_children` 只打 `data-reveal`），
  // 节奏由 `style/modern/kit.css` 覆盖成平滑曲线。
  let page = NodeRef::<leptos::html::Div>::new();
  page.on_load(|el| crate::motion::reveal_children(&el));

  // 题库版本与类别提升到首页：选择器、首屏 CTA、快速入口三处共用同一份选择。
  let version = RwSignal::new(None::<String>);
  let bank = RwSignal::new(Bank::A);

  // 与像素首页同一套守卫：选中版本真的能用才放行 CTA，否则拦下并说明原因。
  // `generation` 用来丢弃过期结果 —— 连点两个题库时，先返回的旧请求不能覆盖新结论。
  let checking = RwSignal::new(false);
  let available = RwSignal::new(false);
  let generation = StoredValue::new(0u32);

  Effect::new(move |_| {
    let Some(v) = version.get() else {
      return;
    };
    let b = bank.get();
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    checking.set(true);
    spawn_local(async move {
      let ok = crate::data::bank_available(Some(&v), b).await;
      if generation.try_get_value() == Some(current) {
        available.set(ok);
        checking.set(false);
      }
    });
  });

  let guard = Callback::new(move |e: web_sys::MouseEvent| {
    if !available.get_untracked() || version.with_untracked(Option::is_none) {
      e.prevent_default();
      push_info(tf(
        "home.question-bank-is-unavailable",
        &[&bank.get_untracked().to_string()],
      ));
    }
  });

  view! {
    <div class="md-root">
      <ModernHero version=version bank=bank on_navigate=guard />

      <div class="md-page" node_ref=page>
        <OnAir />
        <Record />
        // 实时读数紧跟学习记录：两块都是「读数」，放在一起才读得成「我的 → 此刻的」；
        // 再往下才是动作（快速入口）与目录。
        <Live />
        <ModernBankPicker version=version bank=bank disabled=Signal::derive(move || checking.get()) />
        <Bento version=version bank=bank on_navigate=guard />
        <ModuleGrid />

        <section class="md-closing">
          <div class="md-closing-copy">
            <h2 class="md-closing-title">{move || t("home.closing-title")}</h2>
            <p class="md-sm opacity-80">{move || t("home.a-daily-timed-10")}</p>
          </div>
          <a class="md-btn md-btn-accent" href="/daily-challenge">
            <crate::kit_modern::ModernIcon kind=crate::kit_modern::MIcon::Flame />
            {move || t("learning.daily-challenge")}
          </a>
        </section>
      </div>
    </div>
  }
}

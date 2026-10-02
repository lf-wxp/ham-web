use ham_web_core::Bank;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::bank_selector::QuestionBankSelector;
use crate::components::bubble::Bubble;
use crate::components::study_plan_card::StudyPlanCard;
use crate::data;
use crate::icons::{Icon, IconKind};
use crate::pages::{DEFAULT_TITLE, bank_href};
use crate::ui::{
  CARD_HEADER, Size, Variant, button_class, card_class, card_content_class, card_title_class,
};
use crate::util::set_title;

use super::cards_card::CardsCard;
use super::daily_challenge_card::DailyChallengeCard;
use super::daily_question::DailyQuestion;
use super::propagation_widget::PropagationWidget;
use super::review_card::ReviewCard;
use super::wanted_card::WantedCard;
use crate::i18n::{t, tf};

/// 首页模块入口。
struct ModuleCard {
  title: &'static str,
  desc: &'static str,
  href: &'static str,
  icon: IconKind,
  links: &'static [(&'static str, &'static str)],
}

const MODULES: &[ModuleCard] = &[
  ModuleCard {
    title: "考试中心",
    desc: "A/B/C 类题库 · 练习 · 模拟考试",
    href: "/practice",
    icon: IconKind::Timer,
    links: &[
      ("练习", "/practice"),
      ("模拟考试", "/exam"),
      ("每日挑战", "/daily-challenge"),
      ("分类浏览", "/browse"),
      ("闪卡刷题", "/flashcards"),
      ("错题集", "/mistakes"),
      ("学习周报", "/weekly"),
    ],
  },
  ModuleCard {
    title: "知识库",
    desc: "30+ 专题 · 呼号 · 术语 · 模式 · 传播 · 天线 · 通联",
    href: "/reference",
    icon: IconKind::BookOpen,
    links: &[
      ("考试速查", "/reference"),
      ("呼号前缀", "/prefixes"),
      ("术语表", "/glossary"),
      ("莫尔斯电码", "/morse"),
      ("数字模式", "/modes"),
      ("通联实务", "/operating"),
    ],
  },
  ModuleCard {
    title: "工具",
    desc: "计算器 · 通联日志 · 实时数据",
    href: "/tools",
    icon: IconKind::Calculator,
    links: &[
      ("小工具", "/tools"),
      ("通联日志", "/log"),
      ("呼号抄收", "/callsign-copy"),
      ("太阳活动", "/solar"),
      ("业余卫星", "/satellites"),
      ("通知中心", "/notifications"),
    ],
  },
];

#[component]
pub fn HomePage() -> impl IntoView {
  set_title(DEFAULT_TITLE);
  let version = RwSignal::new(None::<String>);
  let bank = RwSignal::new(Bank::A);
  let checking = RwSignal::new(false);
  let available = RwSignal::new(false);
  let warn_open = RwSignal::new(false);
  let warn_text = RwSignal::new(String::new());
  let generation = StoredValue::new(0u32);

  Effect::new(move |_| {
    let (Some(v), b) = (version.get(), bank.get()) else {
      return;
    };
    generation.update_value(|g| *g += 1);
    let current = generation.get_value();
    checking.set(true);
    spawn_local(async move {
      let ok = data::bank_available(Some(&v), b).await;
      if generation.try_get_value() == Some(current) {
        available.set(ok);
        checking.set(false);
      }
    });
  });

  let guard = move |e: web_sys::MouseEvent| {
    if !available.get_untracked() || version.with_untracked(Option::is_none) {
      e.prevent_default();
      warn_text.set(tf(
        "题库 {} 暂不可用或为空，请先构建数据集",
        &[&bank.get_untracked().to_string()],
      ));
      warn_open.set(true);
    }
  };
  let practice_href = move || bank_href("/practice", version.get().as_deref(), bank.get());
  let exam_href = move || bank_href("/exam", version.get().as_deref(), bank.get());
  let custom_href = move || {
    format!(
      "{}&mode=custom",
      bank_href("/exam", version.get().as_deref(), bank.get())
    )
  };

  view! {
    <div class="container relative mx-auto max-w-5xl px-4 pb-12 pt-14 sm:pt-20">
      // Hero
      <section class="flex flex-col items-center text-center">
        <span
          class="reveal inline-flex items-center gap-2 rounded-full border bg-card px-3 py-1 text-xs font-medium text-muted-foreground shadow-sm"
          style="animation-delay: 0ms"
        >
          <span class="relative flex size-1.5">
            <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-primary opacity-60"></span>
            <span class="relative inline-flex size-1.5 rounded-full bg-primary"></span>
          </span>
          {move || t("业余无线电 · Amateur Radio")}
        </span>

        <h1
          class="reveal mt-6 text-4xl font-bold tracking-tight sm:text-6xl"
          style="animation-delay: 60ms"
        >
          <span class="hero-gradient-text">{move || t("业余无线电")}</span>
        </h1>

        <p
          class="reveal mt-4 max-w-xl text-sm text-muted-foreground sm:text-base"
          style="animation-delay: 120ms"
        >
          {move || t("一站式题库练习、知识速查与通联工具平台，从 A/B/C 备考到实时传播，一个入口全部搞定")}
        </p>

        <div class="reveal mt-8 flex flex-wrap justify-center gap-3" style="animation-delay: 180ms">
          <a href="/practice" class=button_class(Variant::Default, Size::Default, "")>
            {move || t("开始练习")}
          </a>
          <a href="/exam" class=button_class(Variant::Secondary, Size::Default, "")>
            {move || t("模拟考试")}
          </a>
          <a href="/reference" class=button_class(Variant::Outline, Size::Default, "")>
            {move || t("浏览知识库")}
          </a>
        </div>

        // 频谱瀑布视觉（SDR 意象）
        <div class="reveal mt-12 w-full max-w-2xl" style="animation-delay: 240ms">
          <div class="rounded-2xl border bg-card/70 p-4 shadow-sm backdrop-blur">
            <div class="mb-3 flex items-center justify-between text-xs font-medium text-muted-foreground">
              <span class="font-mono tracking-widest uppercase">"Spectrum · 14.000 MHz"</span>
              <span class="inline-flex items-center gap-1.5">
                <span class="inline-block size-1.5 rounded-full bg-primary"></span>
                {move || t("接收中")}
              </span>
            </div>
            <div class="spectrum-bar h-14 rounded-xl border"></div>
            <div class="spectrum-ticks mt-2 h-3 rounded-sm opacity-60"></div>
          </div>
        </div>
      </section>

      // 模块入口
      <div class="mt-12 grid grid-cols-1 gap-4 sm:grid-cols-3">
        {MODULES
          .iter()
          .enumerate()
          .map(|(i, m)| {
            view! {
              <a
                href=m.href
                class="reveal group relative flex flex-col gap-4 overflow-hidden rounded-2xl border bg-card p-5 shadow-sm transition-all duration-300 hover:-translate-y-1 hover:border-primary/40 hover:shadow-lg hover:shadow-primary/10"
                style=format!("animation-delay: {}ms", 300 + i * 90)
              >
                <div class="pointer-events-none absolute inset-x-6 top-0 h-px bg-gradient-to-r from-transparent via-primary/60 to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100"></div>
                <div class="flex items-center gap-3">
                  <div class="flex size-11 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary ring-1 ring-primary/10 transition-colors duration-300 group-hover:bg-primary group-hover:text-primary-foreground">
                    <Icon kind=m.icon class="h-5 w-5" />
                  </div>
                  <div class="min-w-0">
                    <div class="font-semibold tracking-tight">{move || t(m.title)}</div>
                    <div class="mt-0.5 text-xs text-muted-foreground">{move || t(m.desc)}</div>
                  </div>
                </div>
                <div class="flex flex-wrap gap-1.5">
                  {m
                    .links
                    .iter()
                    .map(|&(label, _)| {
                      view! {
                        <span class="rounded-full border bg-muted/50 px-2.5 py-0.5 text-xs text-muted-foreground transition-colors group-hover:bg-accent group-hover:text-accent-foreground">
                          {move || t(label)}
                        </span>
                      }
                    })
                    .collect_view()}
                </div>
              </a>
            }
          })
          .collect_view()}
      </div>

      // 快速开始练习（选择题库版本与类别）
      <div data-slot="card" class=card_class("mt-12")>
        <div data-slot="card-header" class=CARD_HEADER>
          <div data-slot="card-title" class=card_title_class("")>{move || t("快速开始练习")}</div>
          <div class="text-sm text-muted-foreground">{move || t("选择题库版本与类别，进入练习或模拟考试")}</div>
        </div>
        <div data-slot="card-content" class=card_content_class("space-y-4")>
          <QuestionBankSelector selected_version=version selected_bank=bank disabled=checking />
          <div class="flex flex-wrap gap-3 relative">
            <a
              href=practice_href
              data-slot="button"
              class=button_class(Variant::Default, Size::Default, "")
              on:click=guard
            >
              {move || t("开始练习")}
            </a>
            <a
              href=exam_href
              data-slot="button"
              class=button_class(Variant::Secondary, Size::Default, "")
              on:click=guard
            >
              {move || t("开始模拟考试")}
            </a>
            <a
              href=custom_href
              data-slot="button"
              class=button_class(Variant::Outline, Size::Default, "")
              on:click=guard
            >
              {move || t("自定义组卷")}
            </a>
            <Bubble open=warn_open text=warn_text />
          </div>
        </div>
      </div>

      <div class="space-y-6">
        <StudyPlanCard />
        <ReviewCard />
        <CardsCard />
        <DailyChallengeCard />
        <WantedCard />
        <DailyQuestion />
        <PropagationWidget />
      </div>
    </div>
  }
}
